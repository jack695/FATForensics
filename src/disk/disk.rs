use std::fs::File;

use super::disk_error::DiskError;
use super::mbr::MBR;
use super::mbr::PTType;
use crate::traits::LayoutDisplay;
use crate::volume::BPB;

enum PartTable {
    Mbr(MBR),
}

enum Volume {
    FAT32(BPB),
}

pub struct Disk {
    part_table: PartTable,
    volumes: Vec<(u32, Volume)>,
}

impl Disk {
    pub fn from_file(path: &str, sector_size: usize, validation: bool) -> Result<Self, DiskError> {
        let mut f = File::open(path)?;

        let mbr = MBR::from_file(&mut f, sector_size)?;

        let mut vol = vec![];
        for (part_idx, pt_entry) in mbr.pt_entries().iter().enumerate() {
            if let PTType::LBAFat32 = pt_entry.pt_type() {
                {
                    match BPB::from_file(&mut f, pt_entry.lba_start(), validation, sector_size) {
                        Ok(bpb) => {
                            vol.push((pt_entry.lba_start(), Volume::FAT32(bpb)));
                        }
                        Err(error) => {
                            eprintln!("Error while reading partition #{}: {}", part_idx, error)
                        }
                    }
                }
            }
        }

        let disk = Disk {
            part_table: PartTable::Mbr(mbr),
            volumes: vol,
        };

        Ok(disk)
    }

    pub fn print_layout(&self, indent: u8) {
        match &self.part_table {
            PartTable::Mbr(mbr) => print!("{}", mbr.display_layout(0, indent)),
        }

        for (offset, vol) in self.volumes.iter() {
            match vol {
                Volume::FAT32(bpb) => {
                    print!("\n{}", bpb.display_layout((*offset).into(), indent + 3))
                }
            }
        }
    }

    pub fn vol_count(&self) -> usize {
        self.volumes.len()
    }
}
