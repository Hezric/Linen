use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

/// A marker beside the running executable opts into local application data.
pub fn portable_data_directory(executable: &Path) -> io::Result<Option<PathBuf>> {
    let directory = executable.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Executable directory is missing",
        )
    })?;
    match fs::metadata(directory.join("portable")) {
        Ok(marker) if marker.is_file() => Ok(Some(directory.join("data"))),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The portable marker must be a file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Resolve settings without creating directories or migrating existing data.
pub fn settings_path(executable: &Path, roaming: Option<&OsStr>, app: &str) -> io::Result<PathBuf> {
    let directory =
        match portable_data_directory(executable)? {
            Some(directory) => directory,
            None => PathBuf::from(roaming.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "APPDATA is not available")
            })?)
            .join(app),
        };
    Ok(directory.join(format!("{app}.toml")))
}
