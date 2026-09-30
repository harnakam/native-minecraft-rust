use rmc_net::{
    driver::{HeadlessDriver, HeadlessDriverConfig},
    transport::DriverTransport,
};
use std::io::{self, Read, Write};

struct CongestedStream {
    bytes: Vec<u8>,
    writes: usize,
}
impl Read for CongestedStream {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}
impl Write for CongestedStream {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        if self.writes == 2 {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let length = data.len().min(2);
        self.bytes.extend_from_slice(&data[..length]);
        Ok(length)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn partial_nonblocking_write_preserves_all_unsent_bytes() {
    assert_backpressure(false);
}

#[test]
fn encrypted_partial_write_does_not_encrypt_pending_bytes_twice() {
    assert_backpressure(true);
}

fn assert_backpressure(encrypted: bool) {
    let config = HeadlessDriverConfig::vanilla_headless(47, "localhost", 25565, "Verify");
    let mut expected_driver = HeadlessDriver::new(config.clone());
    expected_driver.bootstrap_login(None, None).unwrap();
    let expected: Vec<u8> = expected_driver
        .drain_outbound_frames()
        .into_iter()
        .flatten()
        .collect();
    let mut driver = HeadlessDriver::new(config);
    driver.bootstrap_login(None, None).unwrap();
    let mut transport = DriverTransport::new(CongestedStream {
        bytes: Vec::new(),
        writes: 0,
    });
    if encrypted {
        transport.enable_encryption([7; 16]).unwrap();
    }
    transport.flush_outbound(&mut driver).unwrap();
    transport.flush_outbound(&mut driver).unwrap();
    let mut actual = transport.stream().bytes.clone();
    if encrypted {
        rmc_net::crypto::TransportEncryption::new([7; 16])
            .unwrap()
            .decrypt(&mut actual);
    }
    assert_eq!(actual, expected);
}
