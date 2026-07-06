use color_eyre::eyre::Result;
use std::fs::canonicalize;
use std::path::{Path, absolute};

/// Converts a path to a POSIX-style string, replacing backslashes with forward slashes.
#[must_use]
pub fn to_posix_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub trait MakeRelativePath {
    /// Get the relative path string for a given file.
    fn make_relative(&self, file: impl AsRef<Path>) -> Result<String>;

    /// Get the relative path string for a given file, using canonicalized media root and file.
    ///
    /// ⚠️ The media root has to be canonicalized already before calling this function.
    fn make_relative_canon(&self, file: impl AsRef<Path>) -> Result<String>;
}

impl<P: AsRef<Path>> MakeRelativePath for P {
    fn make_relative(&self, root_folder: impl AsRef<Path>) -> Result<String> {
        let file_abs = absolute(self)?;
        let relative_path = file_abs.strip_prefix(root_folder)?;
        Ok(to_posix_string(relative_path))
    }

    fn make_relative_canon(&self, root_folder: impl AsRef<Path>) -> Result<String> {
        let file_canon = canonicalize(self)?;
        let relative_path = file_canon.strip_prefix(root_folder)?;
        Ok(to_posix_string(relative_path))
    }
}
