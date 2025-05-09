use std::fmt;
use std::io;

#[derive(Debug)]
pub enum MBRError {
    Io(io::Error),
    PartitionTableNotSorted,
    OverlappingPartitions,
    InvalidSignature(u16),
}

impl From<io::Error> for MBRError {
    fn from(err: io::Error) -> Self {
        MBRError::Io(err)
    }
}

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

impl std::error::Error for MBRError {}
