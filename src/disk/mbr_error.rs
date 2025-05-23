//! Error types for Master Boot Record (MBR) operations.
//!
//! This module provides error handling for various MBR-related operations,
//! including I/O errors, partition table validation, and boot signature verification.

use std::fmt;
use std::io;

/// Represents errors that can occur during MBR parsing.
#[derive(Debug)]
pub enum MBRError {
    /// Wraps an I/O error that occurred during disk operations.
    Io(io::Error),
    /// Indicates that the partition table entries are not in ascending order by starting sector.
    PartitionTableNotSorted,
    /// Indicates that two or more partitions have overlapping sectors.
    OverlappingPartitions,
    /// Indicates that the boot signature is not valid (should be 0xAA55).
    /// Contains the invalid signature value that was found.
    InvalidSignature(u16),
}

/// Converts standard I/O errors into MBRError.
impl From<io::Error> for MBRError {
    fn from(err: io::Error) -> Self {
        MBRError::Io(err)
    }
}

/// Implements string formatting for MBRError.
impl fmt::Display for MBRError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MBRError::Io(e) => write!(f, "I/O error: {}", e),
            MBRError::PartitionTableNotSorted => {
                write!(f, "Partition table is not sorted")
            }
            MBRError::OverlappingPartitions => {
                write!(f, "Some partitions are overlapping")
            }
            MBRError::InvalidSignature(sig) => write!(f, "Invalid boot signature: {}", sig),
        }
    }
}

/// Implements the standard error trait for MBRError.
impl std::error::Error for MBRError {}
