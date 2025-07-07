//! Declaration of traits reused across the codebase.
//!
//! These traits provide extensibility for displaying layouts and writing to slack space
//! in FAT-family filesystems and disk images.

use std::{
    io::{Seek, Write},
    path::Path,
};
use thiserror::Error;

use crate::filesystem::fat_error::FATError;

/// Wrapper around all the potential errors for the different trait implementers.
#[derive(Error, Debug)]
pub enum TraitError {
    #[error("FAT Error: {0}")]
    FATError(#[from] FATError),
}

/// Trait for displaying the layout of a structure (e.g., disk, partition, volume).
///
/// Implementors should return a formatted string representing the structure's layout.
pub trait LayoutDisplay {
    /// Returns a formatted string representing the layout of the structure.
    ///
    /// # Parameters
    /// - `indent`: The number of spaces to indent the output.
    ///
    /// # Returns
    /// - `Ok(str)` A `String` containing the formatted layout.
    /// - `Err(Error)` if the string formatting failed.
    fn display_layout(&self, indent: u8) -> Result<String, std::fmt::Error>;
}

pub trait TreeDisplay {
    fn display_tree(&self) -> Result<(), TraitError>;
}

/// Trait for writing data to slack space in a volume or file.
///
/// Slack space is the unused space at the end of a cluster or file.
pub trait SlackWriter {
    /// Write data to the slack space of a volume.
    ///
    /// # Parameters
    /// - `writer`: A mutable reference to a type implementing `Write + Seek`.
    /// - `data`: The data to write into slack space.
    ///
    /// # Returns
    /// - `Ok(())` on success.
    /// - `Err(FATError)` if writing fails.
    fn write_to_volume_slack<T: Write + Seek>(
        &self,
        writer: &mut T,
        data: &[u8],
    ) -> Result<(), FATError>;

    /// Write data to the slack space of a specific file.
    ///
    /// # Parameters
    /// - `writer`: A mutable reference to a type implementing `Write + Seek`.
    /// - `file_path`: The path to the file whose slack space will be written.
    /// - `data`: The data to write into slack space.
    ///
    /// # Returns
    /// - `Ok(())` on success.
    /// - `Err(FATError)` if writing fails.
    fn write_to_file_slack<T: Write + Seek>(
        &self,
        writer: &mut T,
        file_path: &Path,
        data: &[u8],
    ) -> Result<(), FATError>;
}
