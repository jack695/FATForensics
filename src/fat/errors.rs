use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BPBError {
    #[error("Invalid jump instruction `{0}`")]
    InvalidJmp(String),
    #[error("Invalid count of bytes per sector: `{0}`. Legal values: 512, 1024, 2048 or 4096")]
    InvalidBytesPerSec(u16),
    #[error(
        "Invalid number of sector per cluster: `{0}`. Legal values: 1, 2, 4, 8, 16, 32, 64, 128"
    )]
    InvalidSecPerClus(u8),
    #[error("Invalid cluster size: `{0}`. Any value greater than 32K is invalid.")]
    InvalidClusSz(u32),
    #[error("Invalid count of reserved sectors: `{0}`. Any value greater than 0 is valid.")]
    InvalidRsvdSecCnt(u16),
    #[error("Invalid number of FATs on this volume: `{0}`.")]
    InvalidNumFat(u8),
    #[error(
        "Invalid count of directory entries in the root directory: `{0}`. It should be 0 for a FAT32 volume. "
    )]
    InvalidRootEntCnt(u16),
    #[error("Invalid total count of sectors on the volume: `{0}`")]
    InvalidTotSec(String),
    #[error("Invalid FAT size:`{0}`")]
    InvalidFatSz(String),
    #[error(
        "Invalid cluster number of the first cluster of the root directory: `{0}`. This value should be greater than 2."
    )]
    InvalidRootClus(u32),
    #[error("Invalid BPB signature: `{0}`. Expected signature: 0x55AA")]
    InvalidSignature(String),
    #[error("IO Error: `{0}`")]
    IOError(io::Error),
    #[error("Unsupported FAT type: `{0}`")]
    UnsupportedFATType(String),
}

impl From<io::Error> for BPBError {
    fn from(err: io::Error) -> Self {
        return BPBError::IOError(err);
    }
}
