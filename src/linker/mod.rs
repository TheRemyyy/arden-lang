use crate::cli::output::format_cli_path;
use crate::process_exit::command_failure_details;
use crate::project::OutputKind;
use colored::*;
use std::env;
#[cfg(any(windows, target_os = "macos"))]
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn validate_opt_level(opt_level: Option<&str>) -> Result<(), String> {
    let Some(raw) = opt_level else {
        return Ok(());
    };

    let normalized = raw.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "0" | "1" | "2" | "3" | "s" | "z" | "fast"
    ) {
        return Ok(());
    }

    Err(format!(
        "{}: Invalid optimization level '{}'. Expected one of: 0, 1, 2, 3, s, z, fast.",
        "error".red().bold(),
        raw
    ))
}

pub(crate) struct LinkConfig<'a> {
    pub(crate) opt_level: Option<&'a str>,
    pub(crate) target: Option<&'a str>,
    pub(crate) output_kind: OutputKind,
    pub(crate) link_search: &'a [String],
    pub(crate) link_libs: &'a [String],
    pub(crate) link_args: &'a [String],
}

fn find_tool_in_path(tool: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(tool);
            if candidate.is_file() {
                return Some(candidate);
            }
            #[cfg(windows)]
            {
                let exe = dir.join(format!("{tool}.exe"));
                if exe.is_file() {
                    return Some(exe);
                }
            }
            None
        })
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LinkerFlavor {
    #[cfg(target_os = "linux")]
    Mold,
    #[cfg(any(target_os = "macos", windows))]
    Lld,
}

impl LinkerFlavor {
    pub(crate) fn cache_key(self) -> &'static str {
        match self {
            #[cfg(target_os = "linux")]
            LinkerFlavor::Mold => "mold",
            #[cfg(any(target_os = "macos", windows))]
            LinkerFlavor::Lld => "lld",
        }
    }
}

pub(crate) fn detect_linker_flavor() -> Result<LinkerFlavor, String> {
    #[cfg(target_os = "linux")]
    {
        if find_tool_in_path("mold").is_some() || find_tool_in_path("ld.mold").is_some() {
            return Ok(LinkerFlavor::Mold);
        }
        Err(format!(
            "{}: Required linker 'mold' not found in PATH. Install mold and retry.",
            "error".red().bold()
        ))
    }

    #[cfg(target_os = "macos")]
    {
        if find_tool_in_path("ld64.lld").is_some()
            || find_tool_in_path("ld.lld").is_some()
            || find_tool_in_path("lld").is_some()
        {
            return Ok(LinkerFlavor::Lld);
        }
        Err(format!(
            "{}: Required LLVM linker not found in PATH. Install lld/ld64.lld and retry.",
            "error".red().bold()
        ))
    }

    #[cfg(windows)]
    {
        if find_tool_in_path("lld-link").is_some() {
            return Ok(LinkerFlavor::Lld);
        }
        Err(format!(
            "{}: Required LLVM linker 'lld-link' not found in PATH. Install LLVM lld and retry.",
            "error".red().bold()
        ))
    }

    #[cfg(not(any(windows, unix)))]
    {
        Err(format!(
            "{}: Unsupported host platform for linker detection.",
            "error".red().bold()
        ))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn should_force_no_pie(link: &LinkConfig<'_>) -> bool {
    if link.output_kind != OutputKind::Bin {
        return false;
    }

    match link.target {
        None => true,
        Some(target) => {
            let target = target.to_ascii_lowercase();
            !(target.contains("windows")
                || target.contains("mingw")
                || target.contains("darwin")
                || target.contains("apple"))
        }
    }
}

fn apply_fallback_current_dir(command: &mut Command) {
    if let Ok(working_dir) = env::current_dir() {
        command.current_dir(working_dir);
        return;
    }
    let temp_dir = env::temp_dir();
    if temp_dir.is_dir() {
        command.current_dir(temp_dir);
    }
}

fn apply_stable_command_dir(command: &mut Command, anchor_path: &Path) {
    if let Some(working_dir) = anchor_path
        .parent()
        .filter(|dir| dir.is_dir())
        .map(Path::to_path_buf)
    {
        command.current_dir(working_dir);
        return;
    }
    let temp_dir = env::temp_dir();
    if temp_dir.is_dir() {
        command.current_dir(temp_dir);
    }
}

fn run_link_command(mut command: Command, tool_label: &str) -> Result<(), String> {
    let command_program = command.get_program().to_string_lossy().into_owned();
    let output = command.output().map_err(|error| {
        format!(
            "{}: Failed to launch {} ('{}'): {}",
            "error".red().bold(),
            tool_label,
            command_program,
            error
        )
    })?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let details = command_failure_details(output.status, &stderr, &stdout);
    Err(format!(
        "{}: {} failed: {}",
        "error".red().bold(),
        tool_label,
        details
    ))
}

#[cfg(target_os = "linux")]
struct LinuxLinkContext {
    dynamic_linker: PathBuf,
    crt1: Option<PathBuf>,
    crti: PathBuf,
    crtn: PathBuf,
    crtbegin: Option<PathBuf>,
    crtend: Option<PathBuf>,
    system_lib_dirs: Vec<PathBuf>,
}

#[cfg(target_os = "linux")]
fn linux_target_descriptor(
    target: Option<&str>,
) -> Result<(&'static str, &'static [&'static str]), String> {
    let target = target.unwrap_or(env::consts::ARCH).to_ascii_lowercase();
    if target.contains("x86_64") {
        return Ok((
            "x86_64-linux-gnu",
            &[
                "/lib64/ld-linux-x86-64.so.2",
                "/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
            ],
        ));
    }
    if target.contains("aarch64") || target.contains("arm64") {
        return Ok((
            "aarch64-linux-gnu",
            &[
                "/lib/ld-linux-aarch64.so.1",
                "/lib64/ld-linux-aarch64.so.1",
                "/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1",
            ],
        ));
    }

    Err(format!(
        "{}: Unsupported Linux link target '{}'. Arden currently supports direct mold linking for x86_64 and aarch64 GNU Linux targets.",
        "error".red().bold(),
        target
    ))
}

#[cfg(target_os = "linux")]
fn first_existing_path(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.exists()).cloned()
}

#[cfg(target_os = "linux")]
fn collect_existing_dirs(candidates: &[PathBuf]) -> Vec<PathBuf> {
    let mut collected = Vec::new();
    for candidate in candidates {
        if candidate.is_dir() && !collected.contains(candidate) {
            collected.push(candidate.clone());
        }
    }
    collected
}

#[cfg(target_os = "linux")]
fn collect_gcc_version_dirs() -> Vec<PathBuf> {
    let mut version_dirs = Vec::new();
    for root in [Path::new("/usr/lib/gcc"), Path::new("/usr/lib64/gcc")] {
        let Ok(triples) = fs::read_dir(root) else {
            continue;
        };
        for triple in triples.filter_map(Result::ok) {
            if !triple.path().is_dir() {
                continue;
            }
            let Ok(versions) = fs::read_dir(triple.path()) else {
                continue;
            };
            for version in versions.filter_map(Result::ok) {
                if version.path().is_dir() {
                    version_dirs.push(version.path());
                }
            }
        }
    }
    version_dirs.sort();
    version_dirs.reverse();
    version_dirs
}

#[cfg(target_os = "linux")]
fn find_gcc_support_object(name: &str) -> Option<PathBuf> {
    for version_dir in collect_gcc_version_dirs() {
        let candidate = version_dir.join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn linux_link_context(link: &LinkConfig<'_>) -> Result<LinuxLinkContext, String> {
    let (triple, dynamic_linker_candidates) = linux_target_descriptor(link.target)?;
    let arch_lib_dirs = collect_existing_dirs(&[
        PathBuf::from(format!("/usr/lib/{triple}")),
        PathBuf::from(format!("/lib/{triple}")),
        PathBuf::from("/usr/lib64"),
        PathBuf::from("/lib64"),
        PathBuf::from("/usr/lib"),
        PathBuf::from("/lib"),
        PathBuf::from("/usr/local/lib64"),
        PathBuf::from("/usr/local/lib"),
    ]);
    let dynamic_linker = first_existing_path(
        &dynamic_linker_candidates
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    )
    .ok_or_else(|| {
        format!(
            "{}: Failed to locate the Linux dynamic loader for target '{}'.",
            "error".red().bold(),
            triple
        )
    })?;

    let crti = first_existing_path(
        &arch_lib_dirs
            .iter()
            .map(|dir| dir.join("crti.o"))
            .collect::<Vec<_>>(),
    )
    .ok_or_else(|| {
        format!(
            "{}: Failed to locate crti.o for target '{}'.",
            "error".red().bold(),
            triple
        )
    })?;
    let crtn = first_existing_path(
        &arch_lib_dirs
            .iter()
            .map(|dir| dir.join("crtn.o"))
            .collect::<Vec<_>>(),
    )
    .ok_or_else(|| {
        format!(
            "{}: Failed to locate crtn.o for target '{}'.",
            "error".red().bold(),
            triple
        )
    })?;

    let crt1 = if link.output_kind == OutputKind::Bin {
        first_existing_path(
            &arch_lib_dirs
                .iter()
                .flat_map(|dir| [dir.join("crt1.o"), dir.join("Scrt1.o")])
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let crtbegin_name = if link.output_kind == OutputKind::Shared {
        "crtbeginS.o"
    } else {
        "crtbegin.o"
    };
    let crtend_name = if link.output_kind == OutputKind::Shared {
        "crtendS.o"
    } else {
        "crtend.o"
    };
    let crtbegin = find_gcc_support_object(crtbegin_name);
    let crtend = find_gcc_support_object(crtend_name);

    let mut system_lib_dirs = arch_lib_dirs;
    if let Some(parent) = crtbegin.as_ref().and_then(|path| path.parent()) {
        let parent = parent.to_path_buf();
        if !system_lib_dirs.contains(&parent) {
            system_lib_dirs.push(parent);
        }
    }

    Ok(LinuxLinkContext {
        dynamic_linker,
        crt1,
        crti,
        crtn,
        crtbegin,
        crtend,
        system_lib_dirs,
    })
}

#[cfg(target_os = "linux")]
fn append_unix_link_inputs(command: &mut Command, link: &LinkConfig<'_>) {
    for path in link.link_search {
        command.arg("-L").arg(path);
    }
    for lib in link.link_libs {
        command.arg(format!("-l{lib}"));
    }
    command
        .arg("-lm")
        .arg("-lpthread")
        .arg("-lc")
        .arg("-lgcc_s")
        .arg("-lgcc");
    for arg in link.link_args {
        command.arg(arg);
    }
}

#[cfg(target_os = "linux")]
fn link_with_mold(
    objects: &[PathBuf],
    output_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    let linker_path = find_tool_in_path("mold")
        .or_else(|| find_tool_in_path("ld.mold"))
        .ok_or_else(|| {
            format!(
                "{}: Required linker 'mold' not found in PATH. Install mold and retry.",
                "error".red().bold()
            )
        })?;
    let context = linux_link_context(link)?;
    let mut command = Command::new(linker_path);
    apply_fallback_current_dir(&mut command);
    let thread_count = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    command
        .arg(format!("--thread-count={thread_count}"))
        .arg("--build-id")
        .arg("--as-needed")
        .arg("-o")
        .arg(output_path);

    if link.output_kind == OutputKind::Shared {
        command.arg("--shared");
    } else if should_force_no_pie(link) {
        command.arg("--no-pie");
    }

    if link.output_kind == OutputKind::Bin {
        command.arg("--dynamic-linker").arg(&context.dynamic_linker);
        if let Some(crt1) = &context.crt1 {
            command.arg(crt1);
        }
    }

    command.arg(&context.crti);
    if let Some(crtbegin) = &context.crtbegin {
        command.arg(crtbegin);
    }

    for object in objects {
        command.arg(object);
    }
    for dir in &context.system_lib_dirs {
        command.arg("-L").arg(dir);
    }
    append_unix_link_inputs(&mut command, link);
    if let Some(crtend) = &context.crtend {
        command.arg(crtend);
    }
    command.arg(&context.crtn);
    apply_stable_command_dir(&mut command, output_path);

    run_link_command(command, "mold")
}

#[cfg(target_os = "macos")]
fn macos_target_arch(target: Option<&str>) -> Result<&'static str, String> {
    let resolved_target = target
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| env::consts::ARCH.to_ascii_lowercase());
    if resolved_target.contains("aarch64") || resolved_target.contains("arm64") {
        return Ok("arm64");
    }
    if resolved_target.contains("x86_64") {
        return Ok("x86_64");
    }

    Err(format!(
        "{}: Unsupported macOS link target '{}'. Arden currently supports direct LLVM lld linking for x86_64 and arm64 macOS targets.",
        "error".red().bold(),
        resolved_target
    ))
}

#[cfg(target_os = "macos")]
fn macos_sdk_root() -> Result<PathBuf, String> {
    if let Some(root) = env::var_os("SDKROOT").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(root);
        if path.is_dir() {
            return Ok(path);
        }
    }

    let xcrun_path = find_tool_in_path("xcrun").unwrap_or_else(|| PathBuf::from("/usr/bin/xcrun"));
    let mut command = Command::new(&xcrun_path);
    command.arg("--sdk").arg("macosx").arg("--show-sdk-path");
    apply_fallback_current_dir(&mut command);
    let output = command.output().map_err(|error| {
        format!(
            "{}: Failed to launch xcrun '{}' to resolve the macOS SDK path: {}",
            "error".red().bold(),
            format_cli_path(&xcrun_path),
            error
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let details = command_failure_details(output.status, &stderr, &stdout);
        return Err(format!(
            "{}: Failed to resolve macOS SDK path with xcrun: {}",
            "error".red().bold(),
            details
        ));
    }

    let sdk_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sdk_path.is_empty() {
        return Err(format!(
            "{}: xcrun did not return a usable macOS SDK path.",
            "error".red().bold()
        ));
    }

    Ok(PathBuf::from(sdk_path))
}

#[cfg(target_os = "macos")]
fn macos_sdk_version() -> Result<String, String> {
    if let Some(version) = env::var_os("MACOSX_DEPLOYMENT_TARGET").filter(|value| !value.is_empty())
    {
        return Ok(version.to_string_lossy().into_owned());
    }

    let xcrun_path = find_tool_in_path("xcrun").unwrap_or_else(|| PathBuf::from("/usr/bin/xcrun"));
    let mut command = Command::new(&xcrun_path);
    command.arg("--sdk").arg("macosx").arg("--show-sdk-version");
    apply_fallback_current_dir(&mut command);
    let output = command.output().map_err(|error| {
        format!(
            "{}: Failed to launch xcrun '{}' to resolve the macOS SDK version: {}",
            "error".red().bold(),
            format_cli_path(&xcrun_path),
            error
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let details = command_failure_details(output.status, &stderr, &stdout);
        return Err(format!(
            "{}: Failed to resolve the macOS SDK version with xcrun: {}",
            "error".red().bold(),
            details
        ));
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        return Err(format!(
            "{}: xcrun did not return a usable macOS SDK version.",
            "error".red().bold()
        ));
    }

    Ok(version)
}

#[cfg(any(test, target_os = "macos"))]
pub(crate) fn escape_response_file_arg(arg: &str) -> String {
    let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

#[cfg(target_os = "macos")]
fn write_link_response_file(path: &Path, args: &[String]) -> Result<(), String> {
    let mut contents = String::new();
    for arg in args {
        contents.push_str(&escape_response_file_arg(arg));
        contents.push('\n');
    }

    fs::write(path, contents).map_err(|error| {
        format!(
            "{}: Failed to write link response file '{}': {}",
            "error".red().bold(),
            format_cli_path(path),
            error
        )
    })
}

#[cfg(target_os = "macos")]
fn path_to_response_arg(path: &Path, context: &str) -> Result<String, String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        format!(
            "{}: {} contains non-UTF-8 path '{}', which cannot be encoded into a linker response file",
            "error".red().bold(),
            context,
            format_cli_path(path)
        )
    })
}

#[cfg(any(test, windows))]
pub(crate) fn windows_machine_flag(target: Option<&str>) -> &'static str {
    let target = target
        .unwrap_or("x86_64-pc-windows-msvc")
        .to_ascii_lowercase();
    if target.contains("aarch64") || target.contains("arm64") {
        "arm64"
    } else if target.contains("x86_64") || target.contains("amd64") {
        "x64"
    } else if target.contains("i686") || target.contains("x86") {
        "x86"
    } else {
        "x64"
    }
}

#[cfg(windows)]
fn windows_search_paths(link: &LinkConfig<'_>) -> Vec<PathBuf> {
    let mut paths = link
        .link_search
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if let Some(lib_env) = env::var_os("LIB") {
        paths.extend(env::split_paths(&lib_env));
    }
    paths
}

#[cfg(windows)]
fn maybe_find_windows_builtins() -> Option<PathBuf> {
    if let Some(explicit_path) = env::var_os("ARDEN_WINDOWS_BUILTINS_LIB")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.exists())
    {
        return Some(explicit_path);
    }

    let prefixes = [
        env::var_os("ARDEN_LLVM_REAL_PREFIX"),
        env::var_os("LLVM_SYS_221_PREFIX"),
    ];
    for prefix in prefixes.into_iter().flatten() {
        let root = PathBuf::from(prefix);
        let clang_lib_root = root.join("lib").join("clang");
        let Ok(entries) = fs::read_dir(clang_lib_root) else {
            continue;
        };
        let mut versions = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .collect::<Vec<_>>();
        versions.sort_by_key(|entry| entry.file_name());
        versions.reverse();
        for version in versions {
            let candidate = version
                .path()
                .join("lib")
                .join("windows")
                .join("clang_rt.builtins-x86_64.lib");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(windows)]
fn normalize_windows_lib_name(lib: &str) -> OsString {
    if lib.ends_with(".lib") {
        return OsString::from(lib);
    }
    OsString::from(format!("{lib}.lib"))
}

#[cfg(windows)]
fn windows_flag_with_path(prefix: &str, path: &Path) -> OsString {
    let mut value = OsString::from(prefix);
    value.push(path.as_os_str());
    value
}

#[cfg(windows)]
fn link_with_lld_link(
    objects: &[PathBuf],
    output_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    let linker_path = find_tool_in_path("lld-link").ok_or_else(|| {
        format!(
            "{}: Required LLVM linker 'lld-link' not found in PATH. Install LLVM lld and retry.",
            "error".red().bold()
        )
    })?;
    let mut command = Command::new(linker_path);
    apply_fallback_current_dir(&mut command);
    let thread_count = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    command
        .arg(windows_flag_with_path("/out:", output_path))
        .arg(format!("/machine:{}", windows_machine_flag(link.target)))
        .arg(format!("/threads:{thread_count}"))
        .arg("/incremental:no")
        .arg("/opt:ref")
        .arg("/opt:icf")
        .arg("/Brepro")
        .arg("/release")
        .arg("/dynamicbase")
        .arg("/nxcompat");

    match link.output_kind {
        OutputKind::Bin => {
            command
                .arg("/subsystem:console")
                .arg("/entry:mainCRTStartup");
        }
        OutputKind::Shared => {
            command.arg("/dll");
            command.arg(windows_flag_with_path(
                "/implib:",
                &output_path.with_extension("lib"),
            ));
        }
        OutputKind::Static => {}
    }

    for object in objects {
        command.arg(object);
    }
    for path in windows_search_paths(link) {
        command.arg(windows_flag_with_path("/libpath:", &path));
    }
    if let Some(builtins) = maybe_find_windows_builtins() {
        command.arg(builtins);
    }

    command.arg("/defaultlib:msvcrt");

    for lib in ["oldnames", "legacy_stdio_definitions", "kernel32"] {
        command.arg(normalize_windows_lib_name(lib));
    }
    for lib in link.link_libs {
        command.arg(normalize_windows_lib_name(lib));
    }
    for arg in link.link_args {
        command.arg(arg);
    }
    apply_stable_command_dir(&mut command, output_path);

    run_link_command(command, "lld-link")
}

#[cfg(target_os = "macos")]
fn link_with_macos_lld(
    objects: &[PathBuf],
    output_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    let linker_path = find_tool_in_path("ld64.lld")
        .or_else(|| find_tool_in_path("ld.lld"))
        .or_else(|| find_tool_in_path("lld"))
        .ok_or_else(|| {
            format!(
                "{}: Required LLVM Mach-O linker not found in PATH. Install ld64.lld and retry.",
                "error".red().bold()
            )
        })?;
    let target_arch = macos_target_arch(link.target)?;
    let sdk_root = macos_sdk_root()?;
    let sdk_version = macos_sdk_version()?;
    let response_path = output_path.with_extension("link.rsp");
    let mut response_args = vec![
        "-arch".to_string(),
        target_arch.to_string(),
        "-platform_version".to_string(),
        "macos".to_string(),
        sdk_version.clone(),
        sdk_version,
        "-syslibroot".to_string(),
        path_to_response_arg(&sdk_root, "macOS SDK root")?,
        "-o".to_string(),
        path_to_response_arg(output_path, "macOS linker output path")?,
        "-dead_strip".to_string(),
        "-demangle".to_string(),
        "-adhoc_codesign".to_string(),
    ];

    if link.output_kind == OutputKind::Shared {
        response_args.push("-dylib".to_string());
    }

    for object in objects {
        response_args.push(path_to_response_arg(object, "macOS object file")?);
    }

    for path in link.link_search {
        response_args.push("-L".to_string());
        response_args.push(path.clone());
    }
    for lib in link.link_libs {
        response_args.push(format!("-l{lib}"));
    }
    for arg in link.link_args {
        response_args.push(arg.clone());
    }
    response_args.push("-lSystem".to_string());
    response_args.push("-lm".to_string());

    write_link_response_file(&response_path, &response_args)?;
    let mut command = Command::new(linker_path);
    apply_fallback_current_dir(&mut command);
    let mut response_arg = OsString::from("@");
    response_arg.push(response_path.as_os_str());
    command.arg(response_arg);
    apply_stable_command_dir(&mut command, output_path);
    let result = run_link_command(command, "ld64.lld");
    if let Err(err) = fs::remove_file(&response_path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            eprintln!(
                "warning: failed to remove temporary linker response file '{}': {}",
                format_cli_path(&response_path),
                err
            );
        }
    }
    result
}

pub(crate) fn link_objects(
    objects: &[PathBuf],
    output_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    if objects.is_empty() {
        return Err(format!(
            "{}: No object files generated for project build.",
            "error".red().bold()
        ));
    }

    match link.output_kind {
        OutputKind::Static => {
            let mut command = Command::new("ar");
            command.arg("rcs").arg(output_path).args(objects);
            apply_fallback_current_dir(&mut command);
            let output = command.output().map_err(|e| {
                format!(
                    "{}: Failed to run ar for static library creation '{}': {}",
                    "error".red().bold(),
                    format_cli_path(output_path),
                    e
                )
            })?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let details = command_failure_details(output.status, &stderr, &stdout);
                return Err(format!(
                    "{}: ar failed while creating static library '{}': {}",
                    "error".red().bold(),
                    format_cli_path(output_path),
                    details
                ));
            }
            Ok(())
        }
        OutputKind::Bin | OutputKind::Shared => {
            #[cfg(target_os = "linux")]
            {
                link_with_mold(objects, output_path, link)
            }
            #[cfg(target_os = "macos")]
            {
                link_with_macos_lld(objects, output_path, link)
            }
            #[cfg(windows)]
            {
                link_with_lld_link(objects, output_path, link)
            }
        }
    }
}
