//! Packet trace primitives for Java-vs-Rust wire comparison.

use crate::framing::CompressionDisposition;
use crate::protocol::{find_packet_spec, PacketDirection, ProtocolState};
use crate::varint::{decode_i32, VarIntError};
use std::io::{self, Write};
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceError {
    VarInt(VarIntError),
    PacketIdOutOfRange(i32),
    UnknownPacketId {
        state: ProtocolState,
        direction: PacketDirection,
        packet_id: i32,
    },
}

impl From<VarIntError> for TraceError {
    fn from(value: VarIntError) -> Self {
        Self::VarInt(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PacketTraceEvent {
    pub timestamp: SystemTime,
    pub state: ProtocolState,
    pub direction: PacketDirection,
    pub packet_id: i32,
    pub packet_name: String,
    pub known_packet: bool,
    pub compression: CompressionDisposition,
    pub packet_len: usize,
    pub packet_bytes: Vec<u8>,
}

impl PacketTraceEvent {
    pub fn summary_line(&self) -> String {
        format!(
            "{:?} {:?} id=0x{packet_id:02x} {name} len={len} compression={compression:?}",
            self.state,
            self.direction,
            packet_id = self.packet_id,
            name = self.packet_name,
            len = self.packet_len,
            compression = self.compression
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PacketTelemetrySnapshot {
    pub total_packets: usize,
    pub inbound_packets: usize,
    pub outbound_packets: usize,
    pub total_bytes: usize,
    pub average_packet_len: f32,
    pub average_inter_arrival_ms: f32,
    pub max_inter_arrival_ms: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PacketTelemetryHistory {
    total_packets: usize,
    inbound_packets: usize,
    outbound_packets: usize,
    total_bytes: usize,
    total_inter_arrival: Duration,
    max_inter_arrival: Duration,
    last_timestamp: Option<SystemTime>,
}

impl PacketTelemetryHistory {
    pub fn record(&mut self, event: &PacketTraceEvent) {
        self.total_packets += 1;
        self.total_bytes += event.packet_len;

        match event.direction {
            PacketDirection::Clientbound => self.inbound_packets += 1,
            PacketDirection::Serverbound => self.outbound_packets += 1,
        }

        if let Some(last_timestamp) = self.last_timestamp {
            if let Ok(delta) = event.timestamp.duration_since(last_timestamp) {
                self.total_inter_arrival += delta;
                self.max_inter_arrival = self.max_inter_arrival.max(delta);
            }
        }

        self.last_timestamp = Some(event.timestamp);
    }

    pub fn extend<'a, I>(&mut self, events: I)
    where
        I: IntoIterator<Item = &'a PacketTraceEvent>,
    {
        for event in events {
            self.record(event);
        }
    }

    pub fn snapshot(&self) -> PacketTelemetrySnapshot {
        let packet_count = self.total_packets.max(1);
        let interval_count = self.total_packets.saturating_sub(1).max(1);

        PacketTelemetrySnapshot {
            total_packets: self.total_packets,
            inbound_packets: self.inbound_packets,
            outbound_packets: self.outbound_packets,
            total_bytes: self.total_bytes,
            average_packet_len: self.total_bytes as f32 / packet_count as f32,
            average_inter_arrival_ms: self.total_inter_arrival.as_secs_f32() * 1000.0
                / interval_count as f32,
            max_inter_arrival_ms: self.max_inter_arrival.as_secs_f32() * 1000.0,
        }
    }
}

pub trait TraceSink {
    fn record(&mut self, event: PacketTraceEvent);
}

pub struct NoopTraceSink;

impl TraceSink for NoopTraceSink {
    fn record(&mut self, _event: PacketTraceEvent) {}
}

#[derive(Default)]
pub struct InMemoryTrace {
    events: Vec<PacketTraceEvent>,
}

impl InMemoryTrace {
    pub fn events(&self) -> &[PacketTraceEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<PacketTraceEvent> {
        self.events
    }

    pub fn render_lines(&self) -> Vec<String> {
        self.events
            .iter()
            .map(PacketTraceEvent::summary_line)
            .collect()
    }
}

impl TraceSink for InMemoryTrace {
    fn record(&mut self, event: PacketTraceEvent) {
        self.events.push(event);
    }
}

pub struct LineWriterTraceSink<W> {
    writer: W,
    first_error: Option<io::Error>,
}

impl<W: Write> LineWriterTraceSink<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            first_error: None,
        }
    }

    pub fn finish(mut self) -> io::Result<W> {
        if let Some(error) = self.first_error.take() {
            return Err(error);
        }

        self.writer.flush()?;
        Ok(self.writer)
    }
}

impl<W: Write> TraceSink for LineWriterTraceSink<W> {
    fn record(&mut self, event: PacketTraceEvent) {
        if self.first_error.is_some() {
            return;
        }

        if let Err(error) = writeln!(self.writer, "{}", event.summary_line()) {
            self.first_error = Some(error);
        }
    }
}

pub fn build_trace_event(
    state: ProtocolState,
    direction: PacketDirection,
    compression: CompressionDisposition,
    packet_bytes: &[u8],
) -> Result<PacketTraceEvent, TraceError> {
    build_trace_event_at(
        SystemTime::now(),
        state,
        direction,
        compression,
        packet_bytes,
    )
}

pub fn build_trace_event_lossy(
    state: ProtocolState,
    direction: PacketDirection,
    compression: CompressionDisposition,
    packet_bytes: &[u8],
) -> Result<PacketTraceEvent, TraceError> {
    build_trace_event_lossy_at(
        SystemTime::now(),
        state,
        direction,
        compression,
        packet_bytes,
    )
}

pub fn build_trace_event_at(
    timestamp: SystemTime,
    state: ProtocolState,
    direction: PacketDirection,
    compression: CompressionDisposition,
    packet_bytes: &[u8],
) -> Result<PacketTraceEvent, TraceError> {
    build_trace_event_inner(
        timestamp,
        state,
        direction,
        compression,
        packet_bytes,
        false,
    )
}

pub fn build_trace_event_lossy_at(
    timestamp: SystemTime,
    state: ProtocolState,
    direction: PacketDirection,
    compression: CompressionDisposition,
    packet_bytes: &[u8],
) -> Result<PacketTraceEvent, TraceError> {
    build_trace_event_inner(timestamp, state, direction, compression, packet_bytes, true)
}

pub fn write_trace_lines<W: Write>(events: &[PacketTraceEvent], mut writer: W) -> io::Result<()> {
    for event in events {
        writeln!(writer, "{}", event.summary_line())?;
    }

    Ok(())
}

fn build_trace_event_inner(
    timestamp: SystemTime,
    state: ProtocolState,
    direction: PacketDirection,
    compression: CompressionDisposition,
    packet_bytes: &[u8],
    allow_unknown: bool,
) -> Result<PacketTraceEvent, TraceError> {
    let (packet_id, _) = decode_i32(packet_bytes)?;
    let packet_id_u8 =
        u8::try_from(packet_id).map_err(|_| TraceError::PacketIdOutOfRange(packet_id))?;
    let (packet_name, known_packet) = match find_packet_spec(state, direction, packet_id_u8) {
        Some(spec) => (spec.name.to_owned(), true),
        None if allow_unknown => (format!("Unknown 0x{packet_id_u8:02x}"), false),
        None => {
            return Err(TraceError::UnknownPacketId {
                state,
                direction,
                packet_id,
            })
        }
    };

    Ok(PacketTraceEvent {
        timestamp,
        state,
        direction,
        packet_id,
        packet_name,
        known_packet,
        compression,
        packet_len: packet_bytes.len(),
        packet_bytes: packet_bytes.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_trace_event, build_trace_event_at, build_trace_event_lossy, write_trace_lines,
        PacketTelemetryHistory,
    };
    use crate::framing::CompressionDisposition;
    use crate::protocol::{PacketDirection, ProtocolState};
    use std::time::{Duration, SystemTime};

    #[test]
    fn resolves_known_packet_name() {
        let event = build_trace_event(
            ProtocolState::Play,
            PacketDirection::Clientbound,
            CompressionDisposition::Disabled,
            &[0x00, 0x2a],
        )
        .expect("packet should resolve");

        assert_eq!(event.packet_id, 0);
        assert_eq!(event.packet_name, "Keep Alive");
        assert!(event.known_packet);
    }

    #[test]
    fn records_unknown_packet_lossily() {
        let event = build_trace_event_lossy(
            ProtocolState::Play,
            PacketDirection::Clientbound,
            CompressionDisposition::Disabled,
            &[0x7e],
        )
        .expect("packet should be traced");

        assert_eq!(event.packet_id, 0x7e);
        assert_eq!(event.packet_name, "Unknown 0x7e");
        assert!(!event.known_packet);
    }

    #[test]
    fn writes_trace_lines() {
        let event = build_trace_event(
            ProtocolState::Play,
            PacketDirection::Clientbound,
            CompressionDisposition::Disabled,
            &[0x00, 0x2a],
        )
        .expect("packet should resolve");
        let mut bytes = Vec::new();

        write_trace_lines(&[event], &mut bytes).expect("trace lines should write");

        let rendered = String::from_utf8(bytes).expect("trace output should be utf8");
        assert!(rendered.contains("Keep Alive"));
    }

    #[test]
    fn summarizes_packet_telemetry() {
        let base = SystemTime::UNIX_EPOCH;
        let first = build_trace_event_at(
            base,
            ProtocolState::Play,
            PacketDirection::Clientbound,
            CompressionDisposition::Disabled,
            &[0x00, 0x2a],
        )
        .expect("packet should resolve");
        let second = build_trace_event_at(
            base + Duration::from_millis(50),
            ProtocolState::Play,
            PacketDirection::Serverbound,
            CompressionDisposition::Disabled,
            &[0x00, 0x2a],
        )
        .expect("packet should resolve");

        let mut telemetry = PacketTelemetryHistory::default();
        telemetry.extend([&first, &second]);
        let snapshot = telemetry.snapshot();

        assert_eq!(snapshot.total_packets, 2);
        assert_eq!(snapshot.inbound_packets, 1);
        assert_eq!(snapshot.outbound_packets, 1);
        assert!(snapshot.average_packet_len > 0.0);
        assert_eq!(snapshot.average_inter_arrival_ms, 50.0);
        assert_eq!(snapshot.max_inter_arrival_ms, 50.0);
    }
}
