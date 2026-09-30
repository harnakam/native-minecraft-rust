//! Zlib backend used by packet framing once compression is enabled.

use crate::compression::{CompressionError, ZlibCodec};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DefaultZlibCodec;

impl ZlibCodec for DefaultZlibCodec {
    fn compress(&self, input: &[u8]) -> Result<Vec<u8>, CompressionError> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(input)
            .map_err(|_| CompressionError::Backend("zlib-compress-write"))?;
        encoder
            .finish()
            .map_err(|_| CompressionError::Backend("zlib-compress-finish"))
    }

    fn decompress(&self, input: &[u8], expected_len: usize) -> Result<Vec<u8>, CompressionError> {
        let mut decoder = ZlibDecoder::new(input);
        let mut output = Vec::with_capacity(expected_len);
        decoder
            .read_to_end(&mut output)
            .map_err(|_| CompressionError::Backend("zlib-decompress-read"))?;
        Ok(output)
    }
}
