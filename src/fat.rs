mod errors;

use crate::constants;
use crate::utils;
use errors::BPBError;

use binread::{BinRead, BinReaderExt};
use std::fmt;
use std::fs::File;
use std::io::Cursor;
use std::vec;

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
        write!(f, "{}", s)
    }
}

#[derive(BinRead, Debug)]
#[br(little)]
pub struct BPB {
    jmp: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sec: u16,
    sec_per_clus: u8,
    rsvd_sec_cnt: u16,
    num_fat: u8,
    root_ent_cnt: u16,
    tot_sec_16: u16,
    media: u8,
    fat_sz_16: u16,
    sec_per_trl: u16,
    num_heds: u16,
    hidd_sec: u32,
    tot_sec_32: u32,

    // FAT32-specific fields
    fat_sz_32: u32,
    ext_flags: u16,
    fs_ver: u16,
    root_clus: u32,
    fs_info: u16,
    bk_boot_sec: u16,
    reserved: [u8; 12],
    drv_num: u8,
    reserved_1: u8,
    boot_sig: u8,
    vol_id: u32,
    vol_lab: [u8; 11],
    fil_sys_type: [u8; 8],

    // Not part of BPB, but added for convenience
    #[br(count = 420)]
    boot_code: Vec<u8>,
    sig: [u8; 2],
}

impl BPB {
    pub fn from_file(file: &mut File, sector: u32, validate: bool) -> Result<BPB, BPBError> {
        let mut buf = vec![0; constants::SECTOR_SIZE];
        utils::read_sector(file, sector.into(), &mut buf)?;

        let mut reader = Cursor::new(buf);
        let bpb: BPB = reader.read_be().unwrap();

        if validate { bpb.validate() } else { Ok(bpb) }
    }

    pub fn fat_type(&self) -> FATType {
        let root_dir_sectors =
            (((self.root_ent_cnt * 32) + (self.bytes_per_sec - 1)) / self.bytes_per_sec) as u32;

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
        let clus_cnt = data_sec / self.sec_per_clus as u32;

        println!(
            "{} {} {} {} {}",
            root_dir_sectors, fat_sz, tot_sec, data_sec, clus_cnt
        );

        if clus_cnt < 4085 {
            return FATType::FAT12;
        } else if clus_cnt < 65525 {
            return FATType::FAT16;
        } else {
            return FATType::FAT32;
        }
    }

    fn validate(self) -> Result<Self, BPBError> {
        // General verification
        if !((self.jmp[0] == 0xEB && self.jmp[2] == 0x90) || self.jmp[0] == 0xE9) {
            return Err(BPBError::InvalidJmp(format!(
                "0x{:02X}{:02X}{:02X}",
                self.jmp[0], self.jmp[1], self.jmp[2],
            )));
        }

        const VALID_BYTES_PER_SEC: [u16; 4] = [512, 1024, 2048, 4096];
        if !VALID_BYTES_PER_SEC.contains(&self.bytes_per_sec) {
            return Err(BPBError::InvalidBytesPerSec(self.bytes_per_sec));
        }

        const VALID_SEC_PER_CLUS: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
        if !VALID_SEC_PER_CLUS.contains(&self.sec_per_clus) {
            return Err(BPBError::InvalidSecPerClus(self.sec_per_clus));
        }

        if self.bytes_per_sec as u32 * self.sec_per_clus as u32 > 32 * 1024 {
            return Err(BPBError::InvalidClusSz(
                self.bytes_per_sec as u32 * self.sec_per_clus as u32,
            ));
        }

        const SIG: [u8; 2] = [0x55, 0xAA];
        if !self.sig.eq(&SIG) {
            return Err(BPBError::InvalidSignature(format!(
                "0x{:02X}{:02X}",
                self.sig[0], self.sig[1]
            )));
        }

        // Specific verification depending on the type of FAT
        let fat_type = self.fat_type();
        if fat_type == FATType::FAT32 {
            self.validate_fat32()
        } else {
            return Err(BPBError::UnsupportedFATType(fat_type.to_string()));
        }
    }

    fn validate_fat32(self) -> Result<Self, BPBError> {
        assert!(self.fat_type() == FATType::FAT32);

        if self.rsvd_sec_cnt == 0 {
            return Err(BPBError::InvalidRsvdSecCnt(self.rsvd_sec_cnt));
        }

        if self.num_fat == 0 {
            return Err(BPBError::InvalidNumFat(self.num_fat));
        }

        if self.root_ent_cnt != 0 {
            return Err(BPBError::InvalidRootEntCnt(self.root_ent_cnt));
        }

        // Check for the count of sectors
        if self.tot_sec_16 != 0 {
            return Err(BPBError::InvalidTotSec(String::from(
                "BPB_TotSec16 should be 0 for a FAT32 volume.",
            )));
        }
        if self.tot_sec_32 == 0 {
            return Err(BPBError::InvalidTotSec(String::from(
                "BPB_TotSec32 should be greater than 0 for a FAT32 volume.",
            )));
        }

        // Check the FAT size
        if self.fat_sz_16 != 0 {
            return Err(BPBError::InvalidFatSz(String::from(
                "BPB_FATSz32 should be 0 for a FAT32 volume.",
            )));
        }
        if self.fat_sz_32 == 0 {
            return Err(BPBError::InvalidFatSz(String::from(
                "BPB_FATSz32 should be greater than 0 for a FAT32 volume.",
            )));
        }

        if self.root_clus < 2 {
            return Err(BPBError::InvalidRootClus(self.root_clus));
        }

        Ok(self)
    }
}
