use crate::project::OutputKind;
use colored::*;
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

pub(crate) fn resolve_clang_opt_flag(opt_level: Option<&str>) -> &'static str {
    let normalized = opt_level
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match normalized.as_str() {
        "" | "3" => "-O3",
        "0" => "-O0",
        "1" => "-O1",
        "2" => "-O2",
        "s" => "-Os",
        "z" => "-Oz",
        "fast" => "-Ofast",
        _ => "-O3",
    }
}

pub(crate) struct LinkConfig<'a> {
    pub(crate) opt_level: Option<&'a str>,
    pub(crate) target: Option<&'a str>,
    pub(crate) output_kind: OutputKind,
    pub(crate) link_search: &'a [String],
    pub(crate) link_libs: &'a [String],
    pub(crate) link_args: &'a [String],
}

pub(crate) fn shutil_which(tool: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let candidate = dir.join(tool);
            if candidate.is_file() {
                return true;
            }
            #[cfg(windows)]
            {
                let exe = dir.join(format!("{}.exe", tool));
                exe.is_file()
            }
            #[cfg(not(windows))]
            {
                false
            }
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
    pub(crate) fn clang_fuse_ld(self) -> &'static str {
        match self {
            #[cfg(target_os = "linux")]
            LinkerFlavor::Mold => "mold",
            #[cfg(any(target_os = "macos", windows))]
            LinkerFlavor::Lld => "lld",
        }
    }

    pub(crate) fn cache_key(self) -> &'static str {
        self.clang_fuse_ld()
    }
}

pub(crate) fn detect_linker_flavor() -> Result<LinkerFlavor, String> {
    #[cfg(target_os = "linux")]
    if shutil_which("mold") || shutil_which("ld.mold") {
        return Ok(LinkerFlavor::Mold);
    }

    #[cfg(target_os = "linux")]
    return Err(format!(
        "{}: Required linker 'mold' not found in PATH. Install mold and retry.",
        "error".red().bold()
    ));

    #[cfg(target_os = "macos")]
    if shutil_which("ld64.lld") || shutil_which("ld.lld") || shutil_which("lld") {
        return Ok(LinkerFlavor::Lld);
    }

    #[cfg(target_os = "macos")]
    return Err(format!(
        "{}: Required LLVM linker not found in PATH. Install lld/ld64.lld and retry.",
        "error".red().bold()
    ));

    #[cfg(windows)]
    if shutil_which("lld-link") || shutil_which("ld.lld") || shutil_which("lld") {
        return Ok(LinkerFlavor::Lld);
    }

    #[cfg(windows)]
    return Err(format!(
        "{}: Required LLVM linker not found in PATH. Install LLVM lld and retry.",
        "error".red().bold()
    ));

    #[allow(unreachable_code)]
    Err(format!(
        "{}: Unsupported host platform for linker detection.",
        "error".red().bold()
    ))
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

pub(crate) fn escape_response_file_arg(arg: &str) -> String {
    let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

pub(crate) fn write_link_response_file(path: &Path, objects: &[PathBuf]) -> Result<(), String> {
    let mut contents = String::new();
    for object in objects {
        contents.push_str(&escape_response_file_arg(&object.display().to_string()));
        contents.push('\n');
    }

    fs::write(path, contents).map_err(|e| {
        format!(
            "{}: Failed to write link response file '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })
}

/// Compile LLVM IR using clang
pub(crate) fn compile_ir(
    ir_path: &Path,
    output_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    let linker = detect_linker_flavor()?;
    let opt_flag = resolve_clang_opt_flag(link.opt_level);
    let run_clang = |march_native: bool, mtune_native: bool| {
        let mut cmd = Command::new("clang");
        cmd.arg(ir_path)
            .arg("-o")
            .arg(output_path)
            .arg("-Wno-override-module")
            .arg(opt_flag)
            .arg(format!("-fuse-ld={}", linker.clang_fuse_ld()));

        match link.output_kind {
            OutputKind::Bin => {}
            OutputKind::Shared => {
                cmd.arg("-shared");
            }
            OutputKind::Static => {
                cmd.arg("-c");
            }
        }

        if let Some(target_triple) = link.target {
            cmd.arg("--target").arg(target_triple);
        }

        if link.target.is_none() {
            if march_native {
                cmd.arg("-march=native");
            }
            if mtune_native {
                cmd.arg("-mtune=native");
            }
        }

        // Safe performance tweak: keep less frame bookkeeping in optimized binaries.
        cmd.arg("-fomit-frame-pointer");

        #[cfg(windows)]
        cmd.arg("-llegacy_stdio_definitions").arg("-lkernel32");

        #[cfg(not(windows))]
        cmd.arg("-lm").arg("-pthread");

        // GitHub Actions Ubuntu links executables as PIE by default; Arden bin objects/IR are
        // regular executable codegen, so request non-PIE explicitly on ELF toolchains.
        #[cfg(all(unix, not(target_os = "macos")))]
        if should_force_no_pie(link) {
            cmd.arg("-no-pie");
        }

        for path in link.link_search {
            cmd.arg(format!("-L{}", path));
        }

        for lib in link.link_libs {
            cmd.arg(format!("-l{}", lib));
        }

        for arg in link.link_args {
            cmd.arg(arg);
        }

        cmd.output()
    };

    // Keep aggressive native tuning, but degrade gracefully if one native flag is unsupported.
    let mut attempts: Vec<(bool, bool)> = vec![(true, true), (true, false), (false, false)];
    if link.target.is_some() {
        attempts = vec![(false, false)];
    }

    let mut last_stderr = String::new();
    for (march_native, mtune_native) in attempts {
        match run_clang(march_native, mtune_native) {
            Ok(output) if output.status.success() => {
                if link.output_kind == OutputKind::Static {
                    let object_path = output_path.with_extension("o");
                    fs::rename(output_path, &object_path).map_err(|e| {
                        format!(
                            "{}: Failed to stage object file for static archive: {}",
                            "error".red().bold(),
                            e
                        )
                    })?;
                    let status = Command::new("ar")
                        .arg("rcs")
                        .arg(output_path)
                        .arg(&object_path)
                        .status()
                        .map_err(|e| {
                            format!(
                                "{}: Failed to run ar for static library creation: {}",
                                "error".red().bold(),
                                e
                            )
                        })?;
                    let _ = fs::remove_file(&object_path);
                    if !status.success() {
                        return Err(format!(
                            "{}: ar failed while creating static library",
                            "error".red().bold()
                        ));
                    }
                }
                return Ok(());
            }
            Ok(output) => {
                last_stderr = String::from_utf8_lossy(&output.stderr).to_string();
            }
            Err(_) => {
                return Err(format!(
                    "{}: Clang not found. Install clang to compile.",
                    "error".red().bold()
                ));
            }
        }
    }

    Err(format!(
        "{}: Clang failed: {}",
        "error".red().bold(),
        last_stderr
    ))
}

pub(crate) fn link_objects(
    objects: &[PathBuf],
    output_path: &Path,
    link: &LinkConfig<'_>,
) -> Result<(), String> {
    let linker = detect_linker_flavor()?;
    if objects.is_empty() {
        return Err(format!(
            "{}: No object files generated for project build.",
            "error".red().bold()
        ));
    }

    let opt_flag = resolve_clang_opt_flag(link.opt_level);
    match link.output_kind {
        OutputKind::Static => {
            let status = Command::new("ar")
                .arg("rcs")
                .arg(output_path)
                .args(objects)
                .status()
                .map_err(|e| {
                    format!(
                        "{}: Failed to run ar for static library creation: {}",
                        "error".red().bold(),
                        e
                    )
                })?;
            if !status.success() {
                return Err(format!(
                    "{}: ar failed while creating static library",
                    "error".red().bold()
                ));
            }
            Ok(())
        }
        OutputKind::Bin | OutputKind::Shared => {
            let response_path = output_path.with_extension("link.rsp");
            write_link_response_file(&response_path, objects)?;
            let mut cmd = Command::new("clang");
            cmd.arg(format!("@{}", response_path.display()))
                .arg("-o")
                .arg(output_path)
                .arg(opt_flag)
                .arg(format!("-fuse-ld={}", linker.clang_fuse_ld()));

            if link.output_kind == OutputKind::Shared {
                cmd.arg("-shared");
            }
            if let Some(target_triple) = link.target {
                cmd.arg("--target").arg(target_triple);
            } else {
                cmd.arg("-march=native").arg("-mtune=native");
            }

            cmd.arg("-fomit-frame-pointer");

            #[cfg(windows)]
            cmd.arg("-llegacy_stdio_definitions").arg("-lkernel32");

            #[cfg(not(windows))]
            cmd.arg("-lm").arg("-pthread");

            // Avoid distro-dependent default PIE linking for normal executables on ELF hosts.
            #[cfg(all(unix, not(target_os = "macos")))]
            if should_force_no_pie(link) {
                cmd.arg("-no-pie");
            }

            for path in link.link_search {
                cmd.arg(format!("-L{}", path));
            }
            for lib in link.link_libs {
                cmd.arg(format!("-l{}", lib));
            }
            for arg in link.link_args {
                cmd.arg(arg);
            }

            let output = cmd.output().map_err(|_| {
                format!(
                    "{}: Clang not found. Install clang to compile.",
                    "error".red().bold()
                )
            })?;
            let _ = fs::remove_file(&response_path);
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Err(format!(
                    "{}: Clang failed while linking objects: {}",
                    "error".red().bold(),
                    stderr
                ))
            }
        }
    }
}
