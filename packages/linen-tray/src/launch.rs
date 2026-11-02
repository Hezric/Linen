use std::{io, path::Path, path::PathBuf, process::Command};

fn runtime_executable(executable: &Path) -> io::Result<Option<PathBuf>> {
    let runtime = executable
        .parent()
        .unwrap()
        .join("runtime")
        .join(executable.file_name().unwrap());
    if runtime.is_file() {
        runtime.canonicalize().map(Some)
    } else {
        Ok(None)
    }
}

/// Scoop's persisted runtime keeps Explorer's unsigned executable identity stable.
/// Portable archives have no runtime directory and run directly.
pub fn relaunch_from_runtime() -> io::Result<bool> {
    let Some(runtime) = runtime_executable(&std::env::current_exe()?)? else {
        return Ok(false);
    };
    Command::new(runtime)
        .args(std::env::args_os().skip(1))
        .spawn()?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_launch_uses_the_same_runtime_across_versions_and_portable_stays_direct() {
        let root = std::env::temp_dir().join(format!("linen-launch-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let checked = (|| -> io::Result<()> {
            let executable = root.join("app.exe");
            assert_eq!(runtime_executable(&executable)?, None);
            std::fs::create_dir(root.join("runtime"))?;
            let runtime = root.join("runtime/app.exe");
            std::fs::write(&runtime, b"version one")?;
            let first = runtime_executable(&executable)?.unwrap();
            std::fs::write(&runtime, b"version two")?;
            assert_eq!(runtime_executable(&executable)?, Some(first));
            assert_eq!(
                runtime_executable(&runtime)?,
                None,
                "the runtime must not relaunch itself"
            );
            Ok(())
        })();
        std::fs::remove_dir_all(root).unwrap();
        checked.unwrap();
    }
}
