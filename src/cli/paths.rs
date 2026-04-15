use colored::Colorize;
use std::cell::Cell;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

#[derive(Debug)]
enum CliPathError {
    CurrentDirRead(String),
    CurrentDirChange(String),
    ScopedAction(String),
    AncestorInspect(String),
    ParentResolve(String),
    PathResolve(String),
    DirectoryRead(String),
    DirectoryEntryRead(String),
    DirectoryEntryInspect(String),
    TempBinaryName(String),
}

impl fmt::Display for CliPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirRead(message)
            | Self::CurrentDirChange(message)
            | Self::ScopedAction(message)
            | Self::AncestorInspect(message)
            | Self::ParentResolve(message)
            | Self::PathResolve(message)
            | Self::DirectoryRead(message)
            | Self::DirectoryEntryRead(message)
            | Self::DirectoryEntryInspect(message)
            | Self::TempBinaryName(message) => write!(f, "{message}"),
        }
    }
}

impl From<CliPathError> for String {
    fn from(value: CliPathError) -> Self {
        value.to_string()
    }
}

use crate::cli::output::format_cli_path;
use crate::project::find_project_root;

pub(crate) struct CwdRestore {
    previous: PathBuf,
}

struct DirLockDepthGuard;

fn fallback_working_dir() -> PathBuf {
    std::env::temp_dir()
}

pub(crate) fn capture_working_dir() -> Result<PathBuf, String> {
    capture_working_dir_impl().map_err(Into::into)
}

fn capture_working_dir_impl() -> Result<PathBuf, CliPathError> {
    current_dir_checked_impl()
}

pub(crate) fn process_current_dir_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

thread_local! {
    static CURRENT_DIR_LOCK_DEPTH: Cell<usize> = const { Cell::new(0) };
}

impl Drop for CwdRestore {
    fn drop(&mut self) {
        if std::env::set_current_dir(&self.previous).is_err() {
            if let Err(err) = std::env::set_current_dir(fallback_working_dir()) {
                eprintln!(
                    "warning: failed to restore process current directory to '{}' and failed fallback: {}",
                    format_cli_path(&self.previous),
                    err
                );
            }
        }
    }
}

impl Drop for DirLockDepthGuard {
    fn drop(&mut self) {
        CURRENT_DIR_LOCK_DEPTH.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

pub(crate) fn with_process_current_dir<T>(
    dir: &Path,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    with_process_current_dir_impl(dir, f).map_err(Into::into)
}

fn with_process_current_dir_impl<T>(
    dir: &Path,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, CliPathError> {
    CURRENT_DIR_LOCK_DEPTH.with(|depth| {
        if depth.get() > 0 {
            return with_process_current_dir_locked_impl(dir, f);
        }

        let _lock = process_current_dir_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        depth.set(depth.get() + 1);
        let _guard = DirLockDepthGuard;
        with_process_current_dir_locked_impl(dir, f)
    })
}

fn with_process_current_dir_locked_impl<T>(
    dir: &Path,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, CliPathError> {
    let previous = capture_working_dir().map_err(CliPathError::PathResolve)?;
    std::env::set_current_dir(dir).map_err(|e| {
        CliPathError::CurrentDirChange(format!(
            "{}: Failed to change current directory to '{}': {}",
            "error".red().bold(),
            format_cli_path(dir),
            e
        ))
    })?;
    let _restore = CwdRestore { previous };
    f().map_err(CliPathError::ScopedAction)
}

pub(crate) fn current_dir_checked() -> Result<PathBuf, String> {
    current_dir_checked_impl().map_err(Into::into)
}

fn current_dir_checked_impl() -> Result<PathBuf, CliPathError> {
    std::env::current_dir()
        .or_else(|_| {
            let fallback = std::env::temp_dir();
            if fallback.is_dir() {
                Ok(fallback)
            } else {
                Err(std::io::Error::other("temporary directory is unavailable"))
            }
        })
        .map_err(|e| {
            CliPathError::CurrentDirRead(format!(
                "{}: Failed to read current directory: {}",
                "error".red().bold(),
                e
            ))
        })
}

pub(crate) fn format_target_label(path: Option<&Path>, current_dir: &Path) -> String {
    if let Some(path) = path {
        return format_cli_path(path);
    }
    if let Some(project_root) = find_project_root(current_dir) {
        return format_cli_path(&project_root);
    }
    format_cli_path(current_dir)
}

pub(crate) fn path_traverses_symlinked_directories(path: &Path) -> Result<bool, String> {
    path_traverses_symlinked_directories_impl(path).map_err(Into::into)
}

fn path_traverses_symlinked_directories_impl(path: &Path) -> Result<bool, CliPathError> {
    let mut current = if path.is_dir() {
        Some(path)
    } else {
        path.parent()
    };
    while let Some(dir) = current {
        if dir.as_os_str().is_empty() {
            break;
        }
        let metadata = fs::symlink_metadata(dir).map_err(|e| {
            CliPathError::AncestorInspect(format!(
                "{}: Failed to inspect path ancestor '{}': {}",
                "error".red().bold(),
                format_cli_path(dir),
                e
            ))
        })?;
        if metadata.file_type().is_symlink() {
            #[cfg(target_os = "macos")]
            {
                let temp_dir = std::env::temp_dir();
                let canonical_temp_dir = temp_dir.canonicalize().ok();
                let canonical_dir = dir.canonicalize().ok();
                let canonical_temp_starts_with_dir = canonical_temp_dir
                    .as_ref()
                    .zip(canonical_dir.as_ref())
                    .map(|(temp, ancestor)| temp.starts_with(ancestor))
                    .unwrap_or(false);
                let is_macos_temp_symlink_prefix = dir == Path::new("/var")
                    || dir == Path::new("/tmp")
                    || dir == Path::new("/private")
                    || dir == Path::new("/private/var")
                    || dir == Path::new("/private/tmp");
                if dir == Path::new("/var")
                    || dir == Path::new("/tmp")
                    || temp_dir.starts_with(dir)
                    || canonical_temp_starts_with_dir
                    || (is_macos_temp_symlink_prefix
                        && canonical_temp_dir
                            .as_ref()
                            .map(|temp| temp.starts_with("/private/var"))
                            .unwrap_or(false))
                {
                    current = dir.parent();
                    continue;
                }
            }
            return Ok(true);
        }
        current = dir.parent();
    }
    Ok(false)
}

pub(crate) fn validate_source_file_path(path: &Path) -> Result<(), String> {
    validate_source_file_path_impl(path).map_err(Into::into)
}

fn validate_source_file_path_impl(path: &Path) -> Result<(), CliPathError> {
    if !path.exists() {
        return Err(CliPathError::PathResolve(format!(
            "Path '{}' does not exist",
            format_cli_path(path)
        )));
    }
    if !path.is_file() {
        return Err(CliPathError::PathResolve(format!(
            "Path '{}' is not a file",
            format_cli_path(path)
        )));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("arden") {
        return Err(CliPathError::PathResolve(format!(
            "Path '{}' is not an .arden file",
            format_cli_path(path)
        )));
    }

    let parent_dir = path.parent().unwrap_or(Path::new("."));
    let normalized_parent = if parent_dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent_dir
    };
    let canonical_parent = normalized_parent.canonicalize().map_err(|e| {
        CliPathError::ParentResolve(format!(
            "{}: Failed to resolve parent directory for '{}': {}",
            "error".red().bold(),
            format_cli_path(path),
            e
        ))
    })?;
    let canonical_path = path.canonicalize().map_err(|e| {
        CliPathError::PathResolve(format!(
            "{}: Failed to resolve path '{}': {}",
            "error".red().bold(),
            format_cli_path(path),
            e
        ))
    })?;
    if !canonical_path.starts_with(&canonical_parent) {
        return Err(CliPathError::PathResolve(format!(
            "Path '{}' resolves outside the requested directory tree",
            format_cli_path(path)
        )));
    }
    if path_traverses_symlinked_directories_impl(path)? {
        return Err(CliPathError::PathResolve(format!(
            "Path '{}' must not traverse symlinked directories",
            format_cli_path(path)
        )));
    }
    Ok(())
}

pub(crate) fn collect_arden_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    collect_arden_files_impl(path).map_err(Into::into)
}

fn collect_arden_files_impl(path: &Path) -> Result<Vec<PathBuf>, CliPathError> {
    if path.is_file() {
        validate_source_file_path_impl(path)?;
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        return Err(CliPathError::PathResolve(format!(
            "Path '{}' does not exist",
            format_cli_path(path)
        )));
    }
    if path_traverses_symlinked_directories_impl(path)? {
        return Err(CliPathError::PathResolve(format!(
            "Path '{}' must not traverse symlinked directories",
            format_cli_path(path)
        )));
    }

    let mut files = Vec::new();
    collect_arden_files_recursive(path, &mut files)?;
    files.sort();
    Ok(files)
}

pub(crate) fn unique_temp_binary_path(tag: &str, source: &Path) -> Result<PathBuf, String> {
    unique_temp_binary_path_impl(tag, source).map_err(Into::into)
}

fn unique_temp_binary_path_impl(tag: &str, source: &Path) -> Result<PathBuf, CliPathError> {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            CliPathError::TempBinaryName(format!(
                "{}: Failed to create unique temporary binary name for '{}': {}",
                "error".red().bold(),
                format_cli_path(source),
                e
            ))
        })?
        .as_nanos();
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("input");
    let filename = format!("{tag}-{stem}-{}-{now}", std::process::id());

    #[cfg(windows)]
    let path = std::env::temp_dir().join(format!("{filename}.exe"));
    #[cfg(not(windows))]
    let path = std::env::temp_dir().join(filename);

    Ok(path)
}

fn collect_arden_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliPathError> {
    for entry in fs::read_dir(dir).map_err(|e| {
        CliPathError::DirectoryRead(format!(
            "Failed to read directory '{}' while collecting .arden files: {}",
            format_cli_path(dir),
            e
        ))
    })? {
        let entry = entry.map_err(|e| {
            CliPathError::DirectoryEntryRead(format!(
                "Failed to read directory entry in '{}' while collecting .arden files: {}",
                format_cli_path(dir),
                e
            ))
        })?;
        let file_type = entry.file_type().map_err(|e| {
            CliPathError::DirectoryEntryInspect(format!(
                "Failed to inspect directory entry '{}' while collecting .arden files: {}",
                format_cli_path(&entry.path()),
                e
            ))
        })?;
        let path = entry.path();
        if file_type.is_symlink() {
            return Err(CliPathError::PathResolve(format!(
                "Path '{}' must not contain symlink entries",
                format_cli_path(&path)
            )));
        } else if file_type.is_dir() {
            collect_arden_files_recursive(&path, files)?;
        } else if file_type.is_file()
            && path.extension().and_then(|ext| ext.to_str()) == Some("arden")
        {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn collect_arden_files_rejects_symlinked_root_directory() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let temp_root = std::env::temp_dir().join(format!(
            "arden-paths-symlink-root-{}-{suffix}",
            std::process::id()
        ));
        let real_dir = temp_root.join("real");
        let linked_dir = temp_root.join("linked");
        fs::create_dir_all(&real_dir).expect("create real directory");
        fs::write(
            real_dir.join("demo.arden"),
            "function main(): None { return None; }\n",
        )
        .expect("write source file");
        std::os::unix::fs::symlink(&real_dir, &linked_dir).expect("create symlink directory");

        let err = collect_arden_files(&linked_dir).expect_err("symlinked root should be rejected");
        assert!(
            err.contains("must not traverse symlinked directories"),
            "{err}"
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn with_process_current_dir_recovers_lock_depth_after_panic() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        use std::time::{SystemTime, UNIX_EPOCH};

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!(
            "arden-paths-lock-depth-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).expect("create temp directory");

        let panic_result = catch_unwind(AssertUnwindSafe(|| {
            let _ = with_process_current_dir(&temp_dir, || -> Result<(), String> {
                panic!("intentional panic for lock-depth regression test");
            });
        }));
        assert!(panic_result.is_err(), "closure should panic");

        with_process_current_dir(&temp_dir, || Ok(())).expect("lock depth should recover");

        let _ = fs::remove_dir_all(temp_dir);
    }
}
