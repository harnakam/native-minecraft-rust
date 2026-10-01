//! Protocol 47 absolute statistic updates.
use super::statistics_registry::REGISTERED_STATISTICS;
use crate::buffer::{PacketReader, PacketWriter};
use crate::codec::CodecError;
use std::collections::BTreeMap;

/// None means that StatList.getOneShotStat would return null.
pub fn statistic_is_achievement(id: &str) -> Option<bool> {
    REGISTERED_STATISTICS
        .binary_search_by_key(&id, |(name, _)| *name)
        .ok()
        .map(|index| REGISTERED_STATISTICS[index].1)
}

/// EntityPlayerSP only forwards local increments for independent statistics.
pub fn statistic_is_independent(id: &str) -> Option<bool> {
    statistic_is_achievement(id).map(|_| {
        super::statistics_registry::INDEPENDENT_STATISTICS
            .binary_search(&id)
            .is_ok()
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatisticsPacket {
    pub values: BTreeMap<String, i32>,
}

impl StatisticsPacket {
    pub(super) fn read(reader: &mut PacketReader<'_>) -> Result<Self, CodecError> {
        let count = reader.read_var_i32()?;
        // Java accepts negative counts as an empty loop. Positive counts cannot
        // consume more than a framed packet; avoid reserving from untrusted input.
        let mut values = BTreeMap::new();
        for _ in 0..count.max(0) {
            let id = reader.read_string(32767)?;
            let value = reader.read_var_i32()?;
            if statistic_is_achievement(&id).is_some() {
                values.insert(id, value);
            }
        }
        Ok(Self { values })
    }

    pub(super) fn write(&self, writer: &mut PacketWriter) -> Result<(), CodecError> {
        writer.write_var_i32(self.values.len() as i32);
        for (id, value) in &self.values {
            writer.write_string(id, 32767)?;
            writer.write_var_i32(*value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::play::PlayClientboundPacket;

    #[test]
    fn unknown_ids_are_ignored_and_duplicate_values_replace() {
        let mut writer = PacketWriter::new();
        writer.write_var_i32(3);
        for (id, value) in [
            ("stat.leaveGame", 1),
            ("unknown.stat", 9),
            ("stat.leaveGame", -2),
        ] {
            writer.write_string(id, 32767).unwrap();
            writer.write_var_i32(value);
        }
        let bytes = writer.into_inner();
        let packet = StatisticsPacket::read(&mut PacketReader::new(&bytes)).unwrap();
        assert_eq!(
            packet.values,
            BTreeMap::from([("stat.leaveGame".into(), -2)])
        );
        let encoded = PlayClientboundPacket::Statistics(packet.clone())
            .encode_packet()
            .unwrap();
        assert_eq!(
            PlayClientboundPacket::decode_packet(&encoded.packet_bytes()).unwrap(),
            PlayClientboundPacket::Statistics(packet)
        );
        assert_eq!(
            statistic_is_achievement("achievement.openInventory"),
            Some(true)
        );
        assert_eq!(statistic_is_achievement("stat.leaveGame"), Some(false));
        assert_eq!(
            statistic_is_achievement("stat.craftItem.minecraft.fake"),
            None
        );
    }

    #[test]
    fn truncated_counts_fail_without_allocating_from_count() {
        let bytes = [255, 255, 255, 255, 7];
        assert!(StatisticsPacket::read(&mut PacketReader::new(&bytes)).is_err());
        let negative = [255, 255, 255, 255, 15];
        assert!(StatisticsPacket::read(&mut PacketReader::new(&negative))
            .unwrap()
            .values
            .is_empty());
    }

    #[test]
    #[ignore = "requires local MCP919 statistics Java oracle"]
    fn local_java_statistics_fixtures_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/statistics-java-oracle.log");
        let oracle = std::fs::read_to_string(path).unwrap();
        let mut ids = 0;
        let mut wires = 0;
        for line in oracle.lines() {
            let fields: Vec<_> = line.split('|').collect();
            match fields[0] {
                "ID" => {
                    assert_eq!(
                        statistic_is_achievement(fields[1]),
                        Some(fields[2] == "true")
                    );
                    ids += 1;
                }
                "WIRE" => {
                    let bytes: Vec<_> = fields[1]
                        .as_bytes()
                        .chunks_exact(2)
                        .map(|pair| {
                            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
                        })
                        .collect();
                    let packet = StatisticsPacket::read(&mut PacketReader::new(&bytes)).unwrap();
                    assert_eq!(packet.values.len(), fields[2].parse::<usize>().unwrap());
                    assert_eq!(
                        packet.values["stat.leaveGame"],
                        fields[3].parse::<i32>().unwrap()
                    );
                    wires += 1;
                }
                _ => {}
            }
        }
        assert_eq!(ids, REGISTERED_STATISTICS.len());
        assert_eq!(wires, 7);
        println!("{ids} MCP919 statistic IDs and {wires} packet fixtures match");
    }
}
