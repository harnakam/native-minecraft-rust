//! Client map storage corresponding to S34PacketMaps.setMapdataTo.
use rmc_net::codec::play::{MapIcon, MapsPacket};
#[derive(Clone, Debug, PartialEq)]
pub struct MapData {
    pub scale: i8,
    pub icons: Vec<MapIcon>,
    pub colors: Vec<u8>,
}
impl Default for MapData {
    fn default() -> Self {
        Self {
            scale: 0,
            icons: vec![],
            colors: vec![0; 128 * 128],
        }
    }
}
impl MapData {
    pub fn receive(&mut self, packet: &MapsPacket) -> Result<(), rmc_net::codec::CodecError> {
        packet.validate()?;
        self.scale = packet.scale;
        self.icons = packet.icons.clone();
        if let Some(p) = &packet.patch {
            for row in 0..usize::from(p.height) {
                let start = usize::from(p.x) + (usize::from(p.y) + row) * 128;
                let source = row * usize::from(p.width);
                self.colors[start..start + usize::from(p.width)]
                    .copy_from_slice(&p.colors[source..source + usize::from(p.width)]);
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use rmc_net::codec::play::{MapPatch, PlayClientboundPacket};
    #[test]
    fn updates_replace_icons_and_scale_but_preserve_unchanged_pixels() {
        let mut world = crate::WorldSnapshot::new(crate::WorldConfig::overworld());
        let packet = MapsPacket {
            map_id: 7,
            scale: 2,
            icons: vec![MapIcon {
                kind: 1,
                rotation: 3,
                x: -5,
                y: 8,
            }],
            patch: Some(MapPatch {
                x: 126,
                y: 127,
                width: 2,
                height: 1,
                colors: vec![9, 255],
            }),
        };
        world
            .apply_play_packet(&PlayClientboundPacket::Maps(packet))
            .unwrap();
        let m = world.map(7).unwrap();
        assert_eq!(&m.colors[16382..], &[9, 255]);
        assert_eq!(m.colors[0], 0);
        assert_eq!(m.icons.len(), 1);
        world
            .apply_play_packet(&PlayClientboundPacket::Maps(MapsPacket {
                map_id: 7,
                scale: 4,
                icons: vec![],
                patch: None,
            }))
            .unwrap();
        let m = world.map(7).unwrap();
        assert_eq!(m.scale, 4);
        assert!(m.icons.is_empty());
        assert_eq!(&m.colors[16382..], &[9, 255]);
    }
}
