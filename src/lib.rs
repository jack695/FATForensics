//! This is the main library module for the FAT32 file system tool.
//!
//! It provides functionality for interacting with FAT32 disk images, including
//! parsing Master Boot Records (MBR), handling user commands, and printing disk layouts.
//!
//! The module re-exports key components such as `Command` and `MBR` for external use.

pub mod commands;
pub mod file_system;
pub mod partitioning;
pub mod traits;
pub mod utils;
