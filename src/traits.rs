//! Declaration of traits reused across the code.

use std::{
    io::{Seek, Write},
    path::Path,
};

use crate::filesystem::fat_error::FATError;

/// Implementation of the LayoutDisplay trait.
/// It is used to display the layout of a given structure such as a disk or partition.
pub trait LayoutDisplay {
    fn display_layout(&self, indent: u8) -> Result<String, std::fmt::Error>;
}

pub trait SlackWriter {
    fn write_to_volume_slack<T: Write + Seek>(
        &self,
        writer: &mut T,
        data: &[u8],
    ) -> Result<(), FATError>;

    fn write_to_file_slack<T: Write + Seek>(
        &self,
        writer: &mut T,
        file_path: &Path,
        data: &[u8],
    ) -> Result<(), FATError>;
}
