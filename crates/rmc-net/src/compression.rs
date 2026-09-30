//! Compression state and backend abstraction for Minecraft framed packets.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionState {
    Disabled,
    Enabled { threshold: usize },
}

impl CompressionState {
    pub fn disabled() -> Self {
        Self::Disabled
    }

    pub fn enabled(threshold: usize) -> Self {
        Self::Enabled { threshold }
    }

    pub fn threshold(self) -> Option<usize> {
        match self {
            Self::Disabled => None,
            Self::Enabled { threshold } => Some(threshold),
        }
    }

    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompressionError {
    MissingCodec,
    InvalidThreshold { threshold: usize },
    BelowThreshold { actual: usize, threshold: usize },
    PayloadTooLarge { actual: usize, limit: usize },
    Backend(&'static str),
}

pub trait ZlibCodec {
    fn compress(&self, input: &[u8]) -> Result<Vec<u8>, CompressionError>;

    fn decompress(&self, input: &[u8], expected_len: usize) -> Result<Vec<u8>, CompressionError>;
}
