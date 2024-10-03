//! This module provides functionality for managing file operations.
//!
//! The `file_hdl` module defines the `FileHdl` struct, which is responsible for creating,
//! writing to, and managing a file. It ensures that the file is removed when the `FileHdl`
//! instance is dropped, providing a convenient way to handle temporary files.

use std::{
    fs::{remove_file, File},
    io::Write,
    path::{Path, PathBuf},
};

use std::fs::OpenOptions;

use anyhow::anyhow;
use log::{error, info};

use crate::error::Result;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait FileHdlOps {
    fn open(&mut self) -> Result<()>;
    fn write_data(&mut self, data: &[u8]) -> Result<()>;
    fn get_path(&self) -> &Path;
}

/// A handler for managing file operations.
///
/// The `FileHdl` struct provides methods to create, write to, and manage a file.
/// It ensures that the file is removed when the `FileHdl` instance is dropped.
pub struct FileHdl {
    path: PathBuf,
    file: Option<File>,
}

impl FileHdl {
    /// Creates a new `FileHdl` for the specified path.
    ///
    /// This method attempts to create a new file at the given path. If the file already exists,
    /// it opens the existing file. The file is opened with read and write permissions.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to the path of the file to be managed.
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - A result containing the `FileHdl` or an error.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be created or opened.
    pub fn from_path(path: &str) -> Self {
        Self { path: path.into(), file: None }
    }

    /// Gets the file object or returns an error.
    ///
    /// # Returns
    ///
    /// * `Result<&mut File>` - A result containing a mutable reference to the file or an error.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file is not created or opened.
    fn get_file(&mut self) -> Result<&mut File> {
        self.file.as_mut().ok_or_else(|| {
            error!("File not created or opened: {:?}", self.path);
            anyhow!("File not created or opened")
        })
    }
}

impl FileHdlOps for FileHdl {
    /// Creates a new file at the specified path.
    ///
    /// This method opens a file at the path specified when the `FileHdl` instance was
    /// created. If the file does not exist, it will be created.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be created or opened.
    fn open(&mut self) -> Result<()> {
        if self.file.is_some() {
            info!("File already created: {:?}", self.path);
            return Ok(());
        }

        info!("Creating file: {:?}", self.path);
        self.file = Some(
            OpenOptions::new()
                .write(true)
                .read(true)
                .create(true) // Create the file if it doesn't exist
                .open(&self.path)?,
        );

        Ok(())
    }

    /// Writes data to the file.
    ///
    /// This method writes the provided data to the file managed by the `FileHdl`.
    ///
    /// # Arguments
    ///
    /// * `data` - A byte slice containing the data to be written.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data cannot be written to the file.
    fn write_data(&mut self, data: &[u8]) -> Result<()> {
        info!("Writing data to file: {:?}", self.path);

        let file = self.get_file()?;

        if let Err(e) = file.write_all(data) {
            error!(
                "Failed to write data to file: {:?}, error: {}",
                self.path, e
            );
            return Err(e.into());
        }
        file.flush()?;
        Ok(())
    }

    fn get_path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FileHdl {
    /// Removes the file when the `FileHdl` is dropped.
    ///
    /// This method ensures that the file is removed from the filesystem when the `FileHdl`
    /// instance goes out of scope. If the file cannot be removed, an error is logged.
    fn drop(&mut self) {
        info!("Removing file: {:?}", self.path);
        if let Err(e) = remove_file(&self.path) {
            error!("Failed to remove file: {:?}, error: {}", self.path, e);
        }
    }
}
