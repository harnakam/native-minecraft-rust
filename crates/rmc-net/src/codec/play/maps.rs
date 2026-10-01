//! Protocol 47 S34 map updates.
use crate::buffer::{BufferError, PacketReader, PacketWriter};
use crate::codec::CodecError;
#[derive(Clone, Debug, PartialEq)]
pub struct MapIcon {
    pub kind: u8,
    pub rotation: u8,
    pub x: i8,
    pub y: i8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MapPatch {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
    pub colors: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MapsPacket {
    pub map_id: i32,
    pub scale: i8,
    pub icons: Vec<MapIcon>,
    pub patch: Option<MapPatch>,
}
impl MapsPacket {
    pub fn validate(&self) -> Result<(), CodecError> {
        if self.icons.len() > 16384 {
            return Err(BufferError::ByteArrayTooLong {
                max_len: 16384,
                actual: self.icons.len(),
            }
            .into());
        }
        if let Some(p) = &self.patch {
            if p.width == 0
                || usize::from(p.x) + usize::from(p.width) > 128
                || usize::from(p.y) + usize::from(p.height) > 128
                || p.colors.len() != usize::from(p.width) * usize::from(p.height)
            {
                return Err(CodecError::InvalidEnumValue {
                    enum_name: "map_patch",
                    actual: p.colors.len() as i32,
                });
            }
        }
        Ok(())
    }
    pub(super) fn read(r: &mut PacketReader<'_>) -> Result<Self, CodecError> {
        let map_id = r.read_var_i32()?;
        let scale = r.read_i8()?;
        let count = r.read_var_i32()?;
        if !(0..=16384).contains(&count) {
            return Err(CodecError::InvalidEnumValue {
                enum_name: "map_icon_count",
                actual: count,
            });
        }
        let mut icons = Vec::new();
        for _ in 0..count {
            let packed = r.read_u8()?;
            icons.push(MapIcon {
                kind: packed >> 4,
                rotation: packed & 15,
                x: r.read_i8()?,
                y: r.read_i8()?,
            });
        }
        let width = r.read_u8()?;
        let patch = if width == 0 {
            None
        } else {
            let height = r.read_u8()?;
            let x = r.read_u8()?;
            let y = r.read_u8()?;
            Some(MapPatch {
                x,
                y,
                width,
                height,
                colors: r.read_byte_array(16384)?,
            })
        };
        let packet = Self {
            map_id,
            scale,
            icons,
            patch,
        };
        packet.validate()?;
        Ok(packet)
    }
    pub(super) fn write(&self, w: &mut PacketWriter) -> Result<(), CodecError> {
        self.validate()?;
        w.write_var_i32(self.map_id);
        w.write_i8(self.scale);
        w.write_var_i32(self.icons.len() as i32);
        for icon in &self.icons {
            w.write_u8((icon.kind & 15) << 4 | icon.rotation & 15);
            w.write_i8(icon.x);
            w.write_i8(icon.y);
        }
        match &self.patch {
            None => w.write_u8(0),
            Some(p) => {
                w.write_u8(p.width);
                w.write_u8(p.height);
                w.write_u8(p.x);
                w.write_u8(p.y);
                w.write_byte_array(&p.colors)?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::super::PlayClientboundPacket;
    use super::*;
    #[test]
    #[ignore = "requires locally executed MCP919 MapPacketProbe fixtures"]
    fn local_java_map_packet_fixtures_match() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/maps-java-oracle.log"),
        )
        .unwrap();
        let mut count = 0;
        for line in text.lines().filter(|l| l.starts_with("MAP|")) {
            let fields: Vec<_> = line.split('|').collect();
            let n = |i: usize| fields[i].parse::<i32>().unwrap();
            let x = n(4) as u8;
            let y = n(5) as u8;
            let width = n(6) as u8;
            let height = n(7) as u8;
            let patch = if width == 0 {
                None
            } else {
                let mut colors = vec![];
                for row in 0..height as usize {
                    for col in 0..width as usize {
                        colors.push(((x as usize + col + (y as usize + row) * 128) * 31) as u8);
                    }
                }
                Some(MapPatch {
                    x,
                    y,
                    width,
                    height,
                    colors,
                })
            };
            let expected = PlayClientboundPacket::Maps(MapsPacket {
                map_id: n(1),
                scale: n(2) as i8,
                icons: (0..n(3))
                    .map(|i| MapIcon {
                        kind: (i * 5) as u8,
                        rotation: (i * 7) as u8,
                        x: (-128 + i * 32) as i8,
                        y: (127 - i * 16) as i8,
                    })
                    .collect(),
                patch,
            });
            let hex = fields[8];
            let bytes: Vec<_> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            assert_eq!(
                PlayClientboundPacket::decode_packet(&bytes).unwrap(),
                expected
            );
            assert_eq!(expected.encode_packet().unwrap().packet_bytes(), bytes);
            count += 1;
        }
        assert_eq!(count, 60);
        println!("60 MCP919 S34 map packet fixtures match byte-for-byte");
    }
    #[test]
    fn exact_wire_preserves_signed_coordinates_and_scale_only_update() {
        let bytes = vec![
            0x34, 0xac, 2, 4, 1, 0xf3, 0x80, 0x7f, 2, 1, 126, 127, 2, 10, 255,
        ];
        let packet = PlayClientboundPacket::decode_packet(&bytes).unwrap();
        assert_eq!(packet.encode_packet().unwrap().packet_bytes(), bytes);
        let PlayClientboundPacket::Maps(p) = packet else {
            panic!()
        };
        assert_eq!(p.map_id, 300);
        assert_eq!(p.icons[0].x, -128);
        assert_eq!(p.icons[0].kind, 15);
        assert_eq!(
            PlayClientboundPacket::decode_packet(&[0x34, 1, 0xff, 0, 0]).unwrap(),
            PlayClientboundPacket::Maps(MapsPacket {
                map_id: 1,
                scale: -1,
                icons: vec![],
                patch: None
            })
        );
    }
    #[test]
    fn malformed_regions_and_counts_are_rejected() {
        for bytes in [
            vec![0x34, 0, 0, 0, 2, 1, 127, 0, 2, 0, 0],
            vec![0x34, 0, 0, 0, 2, 1, 0, 0, 1, 0],
            vec![0x34, 0, 0, 0xff, 0xff, 0xff, 0xff, 0x0f, 0],
        ] {
            assert!(PlayClientboundPacket::decode_packet(&bytes).is_err());
        }
    }
}
