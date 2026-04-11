use core::error::Error;

#[derive(Copy, Clone, Debug)]
pub enum AvlError {
    InvalidFrame,
    InvalidChecksum { expected: u16, actual: u16 },
    InvalidDataCount { data_1_count: u8, data_2_count: u8 },
    InvalidPriority(u8),
    InvalidIoElementValueSize(usize),
}

impl core::fmt::Display for AvlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AvlError::InvalidFrame => write!(f, "Invalid frame"),
            AvlError::InvalidChecksum { expected, actual } => {
                write!(
                    f,
                    "Invalid checksum: expected {}, actual {}",
                    expected, actual
                )
            }
            AvlError::InvalidDataCount {
                data_1_count,
                data_2_count,
            } => {
                write!(
                    f,
                    "Invalid data count: data_1_count {}, data_2_count {}. data_1_count must equal data_2_count",
                    data_1_count, data_2_count
                )
            }
            AvlError::InvalidPriority(value) => write!(f, "Invalid priority: {}", value),
            AvlError::InvalidIoElementValueSize(size) => {
                write!(f, "Invalid IO element value size: {}", size)
            }
        }
    }
}

impl Error for AvlError {}
