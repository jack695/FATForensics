//! Error types for disk and partition operations.
//!
//! This module provides error handling for various disk and partition-related operations,
//! including I/O errors, partition table validation, and boot signature verification.

use std::io;
use thiserror;

/// Represents errors that can occur during MBR parsing.
#[derive(thiserror::Error, Debug)]
pub enum DiskError {
    /// Wraps an I/O error that occurred during disk operations.
    #[error("I/O error: {0}")]
    Io(io::Error),
    /// Indicates that the partition table entries are not in ascending order by starting sector.
    #[error("Partition table is not sorted")]
    PartitionTableNotSorted,
    /// Indicates that two or more partitions have overlapping sectors.
    #[error("Some partitions are overlapping")]
    OverlappingPartitions,
    /// Indicates that the boot signature is not valid.
    /// Contains the invalid signature value that was found.
    #[error("Invalid signature: {0}")]
    InvalidSignature(u16),
    /// Parsing error
    #[error("Parsing error: {0}")]
    ParsingError(String),
}

/// Converts standard I/O errors into MBRError.
impl From<io::Error> for DiskError {
    fn from(err: io::Error) -> Self {
        DiskError::Io(err)
    }
}
