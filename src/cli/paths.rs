use colored::Colorize;
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use crate::project::find_project_root;

pub(crate) struct CwdRestore {
    previous: PathBuf,
}

struct DirLockDepthGuard;

fn fallback_working_dir() -> PathBuf {
    std::env::temp_dir()
}

pub(crate) fn capture_working_dir() -> Result<PathBuf, String> {
    current_dir_checked()
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
                    self.previous.display(),
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
    CURRENT_DIR_LOCK_DEPTH.with(|depth| {
        if depth.get() > 0 {
            return with_process_current_dir_locked(dir, f);
        }

        let _lock = process_current_dir_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        depth.set(depth.get() + 1);
        let _guard = DirLockDepthGuard;
        with_process_current_dir_locked(dir, f)
    })
}

fn with_process_current_dir_locked<T>(
    dir: &Path,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let previous = capture_working_dir()?;
    std::env::set_current_dir(dir).map_err(|e| {
        format!(
            "{}: Failed to change current directory to '{}': {}",
            "error".red().bold(),
            dir.display(),
            e
        )
    })?;
    let _restore = CwdRestore { previous };
    f()
}

pub(crate) fn current_dir_checked() -> Result<PathBuf, String> {
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
            format!(
                "{}: Failed to read current directory: {}",
                "error".red().bold(),
                e
            )
        })
}

pub(crate) fn format_target_label(path: Option<&Path>, current_dir: &Path) -> String {
    if let Some(path) = path {
        return path.display().to_string();
    }
    if let Some(project_root) = find_project_root(current_dir) {
        return project_root.display().to_string();
    }
    current_dir.display().to_string()
}

pub(crate) fn path_traverses_symlinked_directories(path: &Path) -> Result<bool, String> {
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
            format!(
                "{}: Failed to inspect path ancestor '{}': {}",
                "error".red().bold(),
                dir.display(),
                e
            )
        })?;
        if metadata.file_type().is_symlink() {
            #[cfg(target_os = "macos")]
            {
                if dir == Path::new("/var") || dir == Path::new("/tmp") {
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
    if !path.exists() {
        return Err(format!("Path '{}' does not exist", path.display()));
    }
    if !path.is_file() {
        return Err(format!("Path '{}' is not a file", path.display()));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("arden") {
        return Err(format!("Path '{}' is not an .arden file", path.display()));
    }

    let parent_dir = path.parent().unwrap_or(Path::new("."));
    let normalized_parent = if parent_dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent_dir
    };
    let canonical_parent = normalized_parent.canonicalize().map_err(|e| {
        format!(
            "{}: Failed to resolve parent directory for '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    let canonical_path = path.canonicalize().map_err(|e| {
        format!(
            "{}: Failed to resolve path '{}': {}",
            "error".red().bold(),
            path.display(),
            e
        )
    })?;
    if !canonical_path.starts_with(&canonical_parent) {
        return Err(format!(
            "Path '{}' resolves outside the requested directory tree",
            path.display()
        ));
    }
    if path_traverses_symlinked_directories(path)? {
        return Err(format!(
            "Path '{}' must not traverse symlinked directories",
            path.display()
        ));
    }

    Ok(())
}

pub(crate) fn collect_arden_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        validate_source_file_path(path)?;
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        return Err(format!("Path '{}' does not exist", path.display()));
    }
    if path_traverses_symlinked_directories(path)? {
        return Err(format!(
            "Path '{}' must not traverse symlinked directories",
            path.display()
        ));
    }

    let mut files = Vec::new();
    collect_arden_files_recursive(path, &mut files)?;
    files.sort();
    Ok(files)
}

pub(crate) fn unique_temp_binary_path(tag: &str, source: &Path) -> Result<PathBuf, String> {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            format!(
                "{}: Failed to create unique temporary binary name for '{}': {}",
                "error".red().bold(),
                source.display(),
                e
            )
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

fn collect_arden_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read directory entry in '{}': {}",
                dir.display(),
                e
            )
        })?;
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "Failed to inspect directory entry '{}': {}",
                entry.path().display(),
                e
            )
        })?;
        let path = entry.path();
        if file_type.is_symlink() {
            return Err(format!(
                "Path '{}' must not contain symlink entries",
                path.display()
            ));
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
