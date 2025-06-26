//! Declaration of traits reused across the code.

use std::io::{Result, Seek, Write};

/// Implementation of the LayoutDisplay trait.
/// It is used to display the layout of a given structure such as a disk or partition.
pub trait LayoutDisplay {
    fn display_layout(&self, indent: u8) -> String;
}

pub trait SlackWriter {
    fn write_to_volume_slack<T: Write + Seek>(&self, writer: &mut T, data: &[u8]) -> Result<()>;
}
