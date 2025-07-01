//! FAT Bpb structure.
//!
//! This module implements:
//! - BIOS Parameter Block (Bpb) parsing and validation
//! - FAT type detection (FAT12/16/32)
//! - Filesystem structure validation according to Microsoft's FAT specification

use binread::{BinRead, BinReaderExt};
use getset::Getters;
use std::fmt;
use std::io;
use std::vec;

use super::fat_error::FATError;
use super::fat_type::FATType;
use crate::utils;

/// BIOS Parameter Block structure for FAT filesystems.
///
/// The Bpb contains essential information about the filesystem layout and properties.
/// This implementation follows Microsoft's FAT32 specification.
#[derive(BinRead, Debug, Getters)]
#[br(little)]
pub struct Bpb {
    /// Jump instruction to boot code (must be 0xEB ?? 0x90 or 0xE9 ?? ??)
    jmp: [u8; 3],
    /// OEM identifier (e.g., "MSWIN4.1")
    oem_name: [u8; 8],
    /// Number of bytes per sector (512, 1024, 2048, or 4096)
    #[get = "pub(super)"]
    bytes_per_sec: u16,
    /// Number of sectors per cluster (power of 2: 1, 2, 4, 8, 16, 32, 64, or 128)
    #[get = "pub(super)"]
    sec_per_clus: u8,
    /// Number of reserved sectors from start of volume
    #[get = "pub(super)"]
    rsvd_sec_cnt: u16,
    /// Number of FAT copies (typically 2 for redundancy)
    #[get = "pub(super)"]
    num_fat: u8,
    /// Maximum number of root directory entries (0 for FAT32)
    #[get = "pub(super)"]
    root_ent_cnt: u16,
    /// Total sectors for volumes < 32MB (0 for FAT32)
    tot_sec_16: u16,
    /// Media descriptor (0xF8 for fixed disk)
    media: u8,
    /// Sectors per FAT for FAT12/FAT16 (0 for FAT32)
    fat_sz_16: u16,
    /// Sectors per track
    sec_per_trl: u16,
    /// Number of heads
    num_heds: u16,
    /// Number of hidden sectors preceding the partition
    hidd_sec: u32,
    /// Total sectors for volumes >= 32MB
    tot_sec_32: u32,

    // FAT32-specific fields
    /// Sectors per FAT
    fat_sz_32: u32,
    /// FAT flags (mirroring, active FAT)
    ext_flags: u16,
    /// Filesystem version (should be 0:0)
    fs_ver: u16,
    /// First cluster of root directory (typically 2)
    #[get = "pub(super)"]
    root_clus: u32,
    /// Sector number of FSINFO structure
    fs_info: u16,
    /// Sector number of backup boot sector
    bk_boot_sec: u16,
    /// Reserved for future expansion
    reserved: [u8; 12],
    /// Drive number (0x80 for hard disk)
    drv_num: u8,
    /// Reserved (used by Windows NT)
    reserved_1: u8,
    /// Extended boot signature (0x29)
    boot_sig: u8,
    /// Volume serial number
    vol_id: u32,
    /// Volume label (11 bytes)
    vol_lab: [u8; 11],
    /// Filesystem type label ("FAT32   ")
    fil_sys_type: [u8; 8],

    /// Boot code (not part of Bpb specification)
    #[br(count = 420)]
    boot_code: Vec<u8>,
    /// Boot sector signature (0x55 0xAA)
    sig: [u8; 2],
}

impl Bpb {
    /// Reads and optionally validates a Bpb from a file at the specified sector.
    ///
    /// # Parameters
    /// - `file`: The file containing the filesystem
    /// - `sector`: The sector number where the Bpb is located
    /// - `validate`: Whether to perform validation checks on the Bpb
    /// - `sector_size`: The size of each sector in bytes
    ///
    /// # Returns
    /// - `Ok(Bpb)`: The parsed and optionally validated Bpb structure
    /// - `Err(FATError)`: If reading fails or validation fails
    ///
    /// # Errors
    /// - Returns `FATError::IOError` if reading from the file fails
    /// - Returns various `FATError` variants if validation fails and `validate` is true
    pub fn from<T: io::Read + io::Seek>(
        file: &mut T,
        sector: u32,
        validate: bool,
        sector_size: usize,
    ) -> Result<Bpb, FATError> {
        let mut buf = vec![0; sector_size];
        utils::read_sector(file, sector.into(), sector_size, &mut buf)?;

        let mut reader = io::Cursor::new(buf);
        let bpb: Bpb = reader.read_be()?;

        if validate { bpb.validate() } else { Ok(bpb) }
    }

    /// Determines the number of clusters in the data section.
    ///
    /// # Returns
    /// - The number of data clusters.
    pub fn cluster_count(&self) -> u32 {
        let root_dir_sectors = (self.root_ent_cnt * 32).div_ceil(self.bytes_per_sec) as u32;

        let fat_sz = if self.fat_sz_16 > 0 {
            self.fat_sz_16 as u32
        } else {
            self.fat_sz_32
        };

        let tot_sec = if self.tot_sec_16 != 0 {
            self.tot_sec_16 as u32
        } else {
            self.tot_sec_32
        };

        let data_sec = tot_sec
            - (self.rsvd_sec_cnt as u32 + (self.num_fat as u32 * fat_sz) + root_dir_sectors);
        data_sec / self.sec_per_clus as u32
    }

    pub fn fat_sz(&self) -> u32 {
        match self.fat_type() {
            FATType::FAT32 => self.fat_sz_32,
            _ => self.fat_sz_16.into(),
        }
    }

    pub fn tot_sec(&self) -> u32 {
        match self.fat_type() {
            FATType::FAT32 => self.tot_sec_32,
            _ => {
                if self.tot_sec_16 == 0 {
                    self.tot_sec_32
                } else {
                    self.tot_sec_16.into()
                }
            }
        }
    }

    /// Determines the FAT type based on the number of clusters in the filesystem.
    ///
    /// # Returns
    /// - `FATType`: The detected filesystem type based on cluster count:
    ///   - `FAT12` if cluster count < 4085
    ///   - `FAT16` if cluster count < 65525
    ///   - `FAT32` if cluster count >= 65525
    pub(super) fn fat_type(&self) -> FATType {
        let clus_cnt = self.cluster_count();

        if clus_cnt < 4085 {
            FATType::FAT12
        } else if clus_cnt < 65525 {
            FATType::FAT16
        } else {
            FATType::FAT32
        }
    }

    /// Validates the Bpb structure according to FAT32 specification requirements.
    ///
    /// # Returns
    /// - `Ok(Self)`: If all validation checks pass
    /// - `Err(FATError)`: If any validation check fails
    ///
    /// # Errors
    /// - `FATError::InvalidJmp`: If the jump instruction is invalid
    /// - `FATError::InvalidBytesPerSec`: If bytes per sector is not a valid value
    /// - `FATError::InvalidSecPerClus`: If sectors per cluster is not a valid value
    /// - `FATError::InvalidClusSz`: If cluster size exceeds 32 KiB
    /// - `FATError::InvalidSignature`: If boot sector signature is not 0x55AA
    /// - `FATError::UnsupportedFATType`: If filesystem is not FAT32
    fn validate(self) -> Result<Self, FATError> {
        // General verification
        if !((self.jmp[0] == 0xEB && self.jmp[2] == 0x90) || self.jmp[0] == 0xE9) {
            return Err(FATError::InvalidJmp(format!(
                "0x{:02X}{:02X}{:02X}",
                self.jmp[0], self.jmp[1], self.jmp[2],
            )));
        }

        const VALID_BYTES_PER_SEC: [u16; 4] = [512, 1024, 2048, 4096];
        if !VALID_BYTES_PER_SEC.contains(&self.bytes_per_sec) {
            return Err(FATError::InvalidBytesPerSec(self.bytes_per_sec));
        }

        const VALID_SEC_PER_CLUS: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
        if !VALID_SEC_PER_CLUS.contains(&self.sec_per_clus) {
            return Err(FATError::InvalidSecPerClus(self.sec_per_clus));
        }

        if self.bytes_per_sec as u32 * self.sec_per_clus as u32 > 32 * 1024 {
            return Err(FATError::InvalidClusSz(
                self.bytes_per_sec as u32 * self.sec_per_clus as u32,
            ));
        }

        const SIG: [u8; 2] = [0x55, 0xAA];
        if !self.sig.eq(&SIG) {
            return Err(FATError::InvalidSignature(format!(
                "0x{:02X}{:02X}",
                self.sig[0], self.sig[1]
            )));
        }

        // Specific verification depending on the type of FAT
        let fat_type = self.fat_type();
        if fat_type == FATType::FAT32 {
            self.validate_fat32()
        } else {
            Err(FATError::UnsupportedFATType(fat_type.to_string()))
        }
    }

    /// Performs FAT32-specific validation checks.
    ///
    /// # Returns
    /// - `Ok(Self)`: If all FAT32-specific validation checks pass
    /// - `Err(FATError)`: If any validation check fails
    ///
    /// # Errors
    /// - `FATError::InvalidRsvdSecCnt`: If reserved sector count is 0
    /// - `FATError::InvalidNumFat`: If number of FATs is 0
    /// - `FATError::InvalidRootEntCnt`: If root directory entries is not 0
    /// - `FATError::InvalidTotSec`: If total sector fields are invalid for FAT32
    /// - `FATError::InvalidFatSz`: If FAT size fields are invalid for FAT32
    /// - `FATError::InvalidRootClus`: If root directory cluster is less than 2
    fn validate_fat32(self) -> Result<Self, FATError> {
        assert!(self.fat_type() == FATType::FAT32);

        if self.rsvd_sec_cnt == 0 {
            return Err(FATError::InvalidRsvdSecCnt(self.rsvd_sec_cnt));
        }

        if self.num_fat == 0 {
            return Err(FATError::InvalidNumFat(self.num_fat));
        }

        if self.root_ent_cnt != 0 {
            return Err(FATError::InvalidRootEntCnt(self.root_ent_cnt));
        }

        // Check for the count of sectors
        if self.tot_sec_16 != 0 {
            return Err(FATError::InvalidTotSec(String::from(
                "BPB_TotSec16 should be 0 for a FAT32 volume.",
            )));
        }
        if self.tot_sec() == 0 {
            return Err(FATError::InvalidTotSec(String::from(
                "BPB_TotSec32 should be greater than 0 for a FAT32 volume.",
            )));
        }

        // Check the FAT size
        if self.fat_sz_16 != 0 {
            return Err(FATError::InvalidFatSz(String::from(
                "BPB_FATSz32 should be 0 for a FAT32 volume.",
            )));
        }
        if self.fat_sz_32 == 0 {
            return Err(FATError::InvalidFatSz(String::from(
                "BPB_FATSz32 should be greater than 0 for a FAT32 volume.",
            )));
        }

        if self.root_clus < 2 {
            return Err(FATError::InvalidRootClus(self.root_clus));
        }

        Ok(self)
    }
}

/// Implements the Display trait for Bpb
impl fmt::Display for Bpb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut offset = 0;

        macro_rules! field {
            ($name:expr, $val:expr, $size:expr) => {{
                writeln!(f, "  {:<20} 0x{:>04X}: {}", $name, offset, $val)?;
                offset += $size;
            }};
        }

        writeln!(f, "BIOS Parameter Block (Bpb):")?;

        field!("jmp", format!("{:02X?}", self.jmp), 3);
        field!("oem_name", String::from_utf8_lossy(&self.oem_name), 8);
        field!("bytes_per_sec", self.bytes_per_sec, 2);
        field!("sec_per_clus", self.sec_per_clus, 1);
        field!("rsvd_sec_cnt", self.rsvd_sec_cnt, 2);
        field!("num_fat", self.num_fat, 1);
        field!("root_ent_cnt", self.root_ent_cnt, 2);
        field!("tot_sec_16", self.tot_sec_16, 2);
        field!("media", format!("0x{:X}", self.media), 1);
        field!("fat_sz_16", self.fat_sz_16, 2);
        field!("sec_per_trl", self.sec_per_trl, 2);
        field!("num_heds", self.num_heds, 2);
        field!("hidd_sec", self.hidd_sec, 4);
        field!("tot_sec_32", self.tot_sec_32, 4);
        field!("fat_sz_32", self.fat_sz_32, 4);
        field!("ext_flags", self.ext_flags, 2);
        field!("fs_ver", self.fs_ver, 2);
        field!("root_clus", self.root_clus, 4);
        field!("fs_info", self.fs_info, 2);
        field!("bk_boot_sec", self.bk_boot_sec, 2);
        field!("reserved", format!("{:02X?}", &self.reserved[..]), 12);
        field!("drv_num", format!("0x{:X}", self.drv_num), 1);
        field!("reserved_1", self.reserved_1, 1);
        field!("boot_sig", format!("0x{:X}", self.boot_sig), 1);
        field!("vol_id", format!("0x{:X}", self.vol_id), 4);
        field!("vol_lab", String::from_utf8_lossy(&self.vol_lab), 11);
        field!(
            "fil_sys_type",
            String::from_utf8_lossy(&self.fil_sys_type),
            8
        );

        // Now dump boot code with offsets
        writeln!(
            f,
            "\nBoot Code 0x{:04X} ({} bytes):",
            offset,
            self.boot_code.len()
        )?;
        for (i, chunk) in self.boot_code.chunks(16).enumerate() {
            write!(f, "  0x{:04X}: ", offset + i * 16)?;
            for byte in chunk {
                write!(f, "{byte:02X} ")?;
            }
            writeln!(f)?;
        }
        offset += self.boot_code.len();

        // Signature
        writeln!(f, "\nSignature 0x{:04X}: {:02X?}", offset, self.sig)?;

        Ok(())
    }
}
