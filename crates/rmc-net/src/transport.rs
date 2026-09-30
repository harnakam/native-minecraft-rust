//! Blocking stream helper for the headless protocol driver.

use crate::compression::ZlibCodec;
use crate::crypto::{CryptoError, TransportEncryption};
use crate::driver::{DriverError, DriverEvent, HeadlessDriver};
use crate::trace::TraceSink;
use std::io::{self, Read, Write};

pub const DEFAULT_READ_BUFFER_SIZE: usize = 8 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct ReadCycle {
    pub bytes_read: usize,
    pub reached_eof: bool,
    pub events: Vec<DriverEvent>,
}

#[derive(Debug)]
pub enum TransportError {
    Io(io::Error),
    Driver(DriverError),
    Crypto(CryptoError),
}

impl From<io::Error> for TransportError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<DriverError> for TransportError {
    fn from(value: DriverError) -> Self {
        Self::Driver(value)
    }
}

impl From<CryptoError> for TransportError {
    fn from(value: CryptoError) -> Self {
        Self::Crypto(value)
    }
}

pub struct DriverTransport<S> {
    stream: S,
    read_buffer: Vec<u8>,
    encryption: Option<TransportEncryption>,
    pending_outbound: Vec<u8>,
    write_offset: usize,
}

impl<S> DriverTransport<S> {
    pub fn new(stream: S) -> Self {
        Self::with_read_buffer_size(stream, DEFAULT_READ_BUFFER_SIZE)
    }

    pub fn with_read_buffer_size(stream: S, read_buffer_size: usize) -> Self {
        Self {
            stream,
            read_buffer: vec![0; read_buffer_size.max(1)],
            encryption: None,
            pending_outbound: Vec::new(),
            write_offset: 0,
        }
    }

    pub fn stream(&self) -> &S {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    pub fn into_inner(self) -> S {
        self.stream
    }

    pub fn read_buffer_size(&self) -> usize {
        self.read_buffer.len()
    }

    pub fn is_encrypted(&self) -> bool {
        self.encryption.is_some()
    }

    pub fn has_pending_outbound(&self) -> bool {
        self.write_offset < self.pending_outbound.len()
    }

    pub fn enable_encryption(&mut self, shared_secret: [u8; 16]) -> Result<(), TransportError> {
        self.encryption = Some(TransportEncryption::new(shared_secret)?);
        Ok(())
    }
}

impl<S: Read + Write> DriverTransport<S> {
    pub fn flush_outbound(&mut self, driver: &mut HeadlessDriver) -> Result<usize, TransportError> {
        let frames = driver.drain_outbound_frames();
        let mut total_written = 0;

        for mut frame in frames {
            if self.pending_outbound.len().saturating_add(frame.len()) > 8 * 1024 * 1024 {
                return Err(TransportError::Io(io::Error::other(
                    "outbound backlog exceeded 8 MiB",
                )));
            }
            if let Some(encryption) = self.encryption.as_mut() {
                encryption.encrypt(&mut frame);
            }

            self.pending_outbound.extend_from_slice(&frame);
        }

        while self.write_offset < self.pending_outbound.len() {
            match self
                .stream
                .write(&self.pending_outbound[self.write_offset..])
            {
                Ok(0) => return Err(TransportError::Io(io::ErrorKind::WriteZero.into())),
                Ok(written) => {
                    self.write_offset += written;
                    total_written += written;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(TransportError::Io(error)),
            }
        }
        if !self.has_pending_outbound() {
            self.pending_outbound.clear();
            self.write_offset = 0;
        }

        if total_written != 0 {
            match self.stream.flush() {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                result => result?,
            }
        }

        Ok(total_written)
    }

    pub fn read_once(
        &mut self,
        driver: &mut HeadlessDriver,
        codec: Option<&dyn ZlibCodec>,
        trace_sink: Option<&mut dyn TraceSink>,
    ) -> Result<ReadCycle, TransportError> {
        let bytes_read = self.stream.read(&mut self.read_buffer)?;

        if bytes_read == 0 {
            return Ok(ReadCycle {
                bytes_read,
                reached_eof: true,
                events: Vec::new(),
            });
        }

        if let Some(encryption) = self.encryption.as_mut() {
            encryption.decrypt(&mut self.read_buffer[..bytes_read]);
        }

        let events = driver.feed_wire_bytes(&self.read_buffer[..bytes_read], codec, trace_sink)?;
        Ok(ReadCycle {
            bytes_read,
            reached_eof: false,
            events,
        })
    }
}
