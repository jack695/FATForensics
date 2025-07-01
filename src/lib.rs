//!
//! FATForensics: A library and CLI for analyzing FAT32 filesystems and disk images.
//!
//! This crate provides tools for:
//! - Parsing and validating Master Boot Records (MBR)
//! - Interacting with FAT32 filesystems 
//! - Handling user commands for disk and filesystem operations
//! - Printing disk and filesystem layouts
//!
//! The library is designed for extensibility and can be used both as a CLI tool and as a Rust library.
//!
//! # Re-exports
//! - [`FATVol`]: FAT volume abstraction
//! - [`Disk`]: Disk abstraction with partition and volume management
//! - [`Volume`]: Enum for supported volume types

pub mod commands;
pub mod filesystem;
pub mod partition;
pub mod traits;
pub mod utils;

/// FAT volume abstraction (see [`filesystem::fat::FATVol`]).
pub use crate::filesystem::fat::FATVol;
/// Disk abstraction with partition and volume management (see [`partition::disk::Disk`]).
pub use crate::partition::disk::Disk;
/// Enum for supported volume types (see [`partition::disk::Volume`]).
pub use crate::partition::disk::Volume;
