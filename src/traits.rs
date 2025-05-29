//! Declaration of traits reused across the code.

/// Implementation of the LayoutDisplay trait.
/// It is used to display the layout of a given structure such as a disk or partition.
pub trait LayoutDisplay {
    fn display_layout(&self, sector_offset: u64, indent: u8) -> String;
}
