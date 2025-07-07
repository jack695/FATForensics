//! Enum for the different FAT types (FAT12, FAT16, FAT32).
//!
//! This module defines the FATType enum, which is used to distinguish between FAT12, FAT16, and FAT32
//! filesystems based on the number of clusters and other BPB fields.

use std::fmt;

/// Represents the different types of FAT filesystems.
///
/// # Values
/// - `FAT12`: 12-bit File Allocation Table entries
/// - `FAT16`: 16-bit File Allocation Table entries
/// - `FAT32`: 32-bit File Allocation Table entries (most common on large volumes)
///
/// Note: Currently only FAT32 is fully supported for analysis.
#[derive(PartialEq)]
pub enum FATType {
    FAT12,
    FAT16,
    FAT32,
}

impl fmt::Display for FATType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FATType::FAT12 => "FAT12",
            FATType::FAT16 => "FAT16",
            FATType::FAT32 => "FAT32",
        };
        write!(f, "{s}")
    }
}
