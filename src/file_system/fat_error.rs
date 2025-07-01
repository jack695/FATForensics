//! Error types for BIOS Parameter Block (BPB) parsing and validation.
//!
//! The BPB is a data structure that describes the physical layout and properties of a FAT file system.
//! This module defines errors that can occur while parsing and validating BPB fields according to the
//! FAT32 specification.

use std::io;
use thiserror::Error;

/// Errors that can occur during BPB parsing and validation.
#[derive(Error, Debug)]
pub enum FATError {
    /// The first three bytes of a FAT volume must contain a valid x86 jump instruction.
    #[error("Invalid jump instruction `{0}`")]
    InvalidJmp(String),

    /// Bytes per sector must be 512, 1024, 2048 or 4096.
    /// This value represents the fundamental unit of data transfer for the filesystem.
    #[error("Invalid count of bytes per sector: `{0}`. Legal values: 512, 1024, 2048 or 4096")]
    InvalidBytesPerSec(u16),

    /// Sectors per cluster must be a power of 2: 1, 2, 4, 8, 16, 32, 64, or 128.
    /// This value determines how many sectors make up one cluster.
    #[error(
        "Invalid number of sector per cluster: `{0}`. Legal values: 1, 2, 4, 8, 16, 32, 64, 128"
    )]
    InvalidSecPerClus(u8),

    /// Total cluster size (bytes per sector × sectors per cluster) must not exceed 32 KiB.
    #[error("Invalid cluster size: `{0}`. Any value greater than 32K is invalid.")]
    InvalidClusSz(u32),

    /// The count of reserved sectors must be greater than 0.
    /// These sectors precede the first FAT and typically contain the boot sector and FS information sector.
    #[error("Invalid count of reserved sectors: `{0}`. Any value greater than 0 is valid.")]
    InvalidRsvdSecCnt(u16),

    /// The number of File Allocation Tables must be valid (typically 2 for redundancy).
    #[error("Invalid number of FATs on this volume: `{0}`.")]
    InvalidNumFat(u8),

    /// For FAT32 volumes, the root directory entries count must be 0 as the root directory is stored as a regular cluster chain.
    #[error(
        "Invalid count of directory entries in the root directory: `{0}`. It should be 0 for a FAT32 volume. "
    )]
    InvalidRootEntCnt(u16),

    /// The total sector count must be valid for the volume size.
    #[error("Invalid total count of sectors on the volume: `{0}`")]
    InvalidTotSec(String),

    /// The FAT size in sectors must be valid and consistent with the volume layout.
    #[error("Invalid FAT size:`{0}`")]
    InvalidFatSz(String),

    /// The root directory's first cluster number must be greater than 2.
    /// Clusters 0 and 1 are reserved, and the data area starts at cluster 2.
    #[error(
        "Invalid cluster number of the first cluster of the root directory: `{0}`. This value should be greater than 2."
    )]
    InvalidRootClus(u32),

    /// The boot sector signature must be 0x55AA.
    #[error("Invalid BPB signature: `{0}`. Expected signature: 0x55AA")]
    InvalidSignature(String),

    /// Underlying I/O errors that occur while reading the BPB.
    #[error("IO Error: `{0}`")]
    IOError(io::Error),

    /// The detected FAT type is not supported (only FAT32 is supported).
    #[error("Unsupported FAT type: `{0}`")]
    UnsupportedFATType(String),

    /// The file was not found
    #[error("File not found")]
    FileNotFound,

    /// Insufficient slack space
    #[error("Insufficient slack space: {free} free bytes for storing {needed} bytes.")]
    InsufficientSlackSpace { free: u32, needed: u32 },

    /// No chain of free clusters
    #[error("No chain of `{0}` free clusters found.")]
    NoFreeClusterChain(u32),

    /// Unsupported feature
    #[error("Unsupported feature.")]
    UnsupportedFeature(String),

    /// Parsing error occured during structure initialization
    #[error("BinRead Error: `{0}`")]
    BinReadError(binread::Error),
}

/// Converts standard I/O errors into FATError.
impl From<io::Error> for FATError {
    fn from(err: io::Error) -> Self {
        FATError::IOError(err)
    }
}

/// Converts BinRead errors into FATError.
impl From<binread::Error> for FATError {
    fn from(err: binread::Error) -> Self {
        FATError::BinReadError(err)
    }
}
