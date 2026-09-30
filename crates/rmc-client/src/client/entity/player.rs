//! Received remote player state corresponding to client world entities.
use rmc_game::player::Vec3;
use rmc_net::codec::play::{
    EntityHeadLookPacket, EntityLookMovePacket, EntityLookPacket, EntityRelativeMovePacket,
    EntityTeleportPacket, PlayClientboundPacket, SpawnPlayerPacket,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct TrackedPlayerEntity {
    pub entity_id: i32,
    pub uuid: [u8; 16],
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub on_ground: bool,
    pub held_item: i16,
    pub equipment: [rmc_net::codec::play::Slot; 5],
    pub metadata: BTreeMap<u8, rmc_net::codec::play::metadata::Value>,
}

impl TrackedPlayerEntity {
    pub fn flag(&self, bit: u8) -> bool {
        match self.metadata.get(&0) {
            Some(rmc_net::codec::play::metadata::Value::Byte(value)) if bit < 8 => {
                (*value as u8 & (1 << bit)) != 0
            }
            _ => false,
        }
    }
}

#[derive(Default)]
pub struct EntityTracker {
    players: BTreeMap<i32, TrackedPlayerEntity>,
}

impl EntityTracker {
    pub fn clear(&mut self) {
        self.players.clear();
    }

    pub fn players(&self) -> impl Iterator<Item = &TrackedPlayerEntity> {
        self.players.values()
    }

    pub fn apply_packet(&mut self, packet: &PlayClientboundPacket) {
        match packet {
            PlayClientboundPacket::EntityMetadata(packet) => {
                if let (Some(entity), Ok(values)) = (
                    self.players.get_mut(&packet.entity_id),
                    rmc_net::codec::play::metadata::decode(&packet.metadata),
                ) {
                    entity.metadata.extend(values);
                }
            }
            PlayClientboundPacket::EntityEquipment(packet) => {
                if let (Some(entity), Ok(slot)) = (
                    self.players.get_mut(&packet.entity_id),
                    usize::try_from(packet.slot),
                ) {
                    if let Some(equipment) = entity.equipment.get_mut(slot) {
                        *equipment = packet.item.clone();
                        if slot == 0 {
                            entity.held_item = packet.item.as_ref().map_or(0, |item| item.item_id);
                        }
                    }
                }
            }
            PlayClientboundPacket::SpawnPlayer(packet) => self.apply_spawn_player(packet),
            PlayClientboundPacket::DestroyEntities(packet) => {
                for entity_id in &packet.entity_ids {
                    self.players.remove(entity_id);
                }
            }
            PlayClientboundPacket::EntityRelativeMove(packet) => self.apply_relative_move(packet),
            PlayClientboundPacket::EntityLook(packet) => self.apply_entity_look(packet),
            PlayClientboundPacket::EntityLookMove(packet) => self.apply_entity_look_move(packet),
            PlayClientboundPacket::EntityTeleport(packet) => self.apply_entity_teleport(packet),
            PlayClientboundPacket::EntityHeadLook(packet) => self.apply_head_look(packet),
            _ => {}
        }
    }

    pub(crate) fn apply_spawn_player(&mut self, packet: &SpawnPlayerPacket) {
        let Ok(metadata) = rmc_net::codec::play::metadata::decode(&packet.metadata) else {
            return;
        };
        let yaw = angle_to_degrees(packet.yaw);
        self.players.insert(
            packet.entity_id,
            TrackedPlayerEntity {
                entity_id: packet.entity_id,
                uuid: packet.player_uuid,
                position: Vec3::new(
                    f64::from(packet.x) / 32.0,
                    f64::from(packet.y) / 32.0,
                    f64::from(packet.z) / 32.0,
                ),
                yaw,
                pitch: angle_to_degrees(packet.pitch),
                head_yaw: yaw,
                on_ground: false,
                held_item: packet.held_item,
                metadata: metadata.into_iter().collect(),
                equipment: [
                    (packet.held_item > 0)
                        .then(|| rmc_net::codec::play::ItemStack::simple(packet.held_item, 1, 0)),
                    None,
                    None,
                    None,
                    None,
                ],
            },
        );
    }

    fn apply_relative_move(&mut self, packet: &EntityRelativeMovePacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.position.x += f64::from(packet.delta_x) / 32.0;
            entity.position.y += f64::from(packet.delta_y) / 32.0;
            entity.position.z += f64::from(packet.delta_z) / 32.0;
            entity.on_ground = packet.on_ground;
        }
    }

    fn apply_entity_look(&mut self, packet: &EntityLookPacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.yaw = angle_to_degrees(packet.yaw);
            entity.pitch = angle_to_degrees(packet.pitch);
            entity.on_ground = packet.on_ground;
        }
    }

    fn apply_entity_look_move(&mut self, packet: &EntityLookMovePacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.position.x += f64::from(packet.delta_x) / 32.0;
            entity.position.y += f64::from(packet.delta_y) / 32.0;
            entity.position.z += f64::from(packet.delta_z) / 32.0;
            entity.yaw = angle_to_degrees(packet.yaw);
            entity.pitch = angle_to_degrees(packet.pitch);
            entity.on_ground = packet.on_ground;
        }
    }

    fn apply_entity_teleport(&mut self, packet: &EntityTeleportPacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.position = Vec3::new(
                f64::from(packet.x) / 32.0,
                f64::from(packet.y) / 32.0,
                f64::from(packet.z) / 32.0,
            );
            entity.yaw = angle_to_degrees(packet.yaw);
            entity.pitch = angle_to_degrees(packet.pitch);
            entity.on_ground = packet.on_ground;
        }
    }

    fn apply_head_look(&mut self, packet: &EntityHeadLookPacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.head_yaw = angle_to_degrees(packet.head_yaw);
        }
    }
}

fn angle_to_degrees(value: u8) -> f32 {
    (f32::from(value) * 360.0) / 256.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmc_net::codec::play::{DestroyEntitiesPacket, EntityEquipmentPacket, ItemStack};
    #[test]
    fn equipment_updates_existing_player_and_destroy_discards_it() {
        let mut entities = EntityTracker::default();
        let equipment = PlayClientboundPacket::EntityEquipment(EntityEquipmentPacket {
            entity_id: 7,
            slot: 4,
            item: Some(ItemStack::simple(310, 1, 17)),
        });
        entities.apply_packet(&equipment);
        assert_eq!(entities.players().count(), 0);
        entities.apply_spawn_player(&SpawnPlayerPacket {
            entity_id: 7,
            player_uuid: [7; 16],
            x: 0,
            y: 0,
            z: 0,
            yaw: 0,
            pitch: 0,
            held_item: 0,
            metadata: vec![127],
        });
        entities.apply_packet(&equipment);
        assert_eq!(
            entities.players().next().unwrap().equipment[4],
            Some(ItemStack::simple(310, 1, 17))
        );
        entities.apply_packet(&PlayClientboundPacket::EntityEquipment(
            EntityEquipmentPacket {
                entity_id: 7,
                slot: 0,
                item: Some(ItemStack::simple(276, 1, 0)),
            },
        ));
        assert_eq!(entities.players().next().unwrap().held_item, 276);
        entities.apply_packet(&PlayClientboundPacket::EntityEquipment(
            EntityEquipmentPacket {
                entity_id: 7,
                slot: 4,
                item: None,
            },
        ));
        assert!(entities.players().next().unwrap().equipment[4].is_none());
        entities.apply_packet(&PlayClientboundPacket::DestroyEntities(
            DestroyEntitiesPacket {
                entity_ids: vec![7],
            },
        ));
        assert_eq!(entities.players().count(), 0);
    }

    #[test]
    fn spawn_flags_and_metadata_updates_preserve_other_indices() {
        let mut entities = EntityTracker::default();
        entities.apply_spawn_player(&SpawnPlayerPacket {
            entity_id: 7,
            player_uuid: [7; 16],
            x: 0,
            y: 0,
            z: 0,
            yaw: 0,
            pitch: 0,
            held_item: 0,
            metadata: vec![0, 2, 0x26, 0, 9, 127],
        });
        assert!(entities.players().next().unwrap().flag(1));
        entities.apply_packet(&PlayClientboundPacket::EntityMetadata(
            rmc_net::codec::play::EntityMetadataPacket {
                entity_id: 7,
                metadata: vec![0, 32, 127],
            },
        ));
        let player = entities.players().next().unwrap();
        assert!(!player.flag(1));
        assert!(player.flag(5));
        assert_eq!(
            player.metadata.get(&6),
            Some(&rmc_net::codec::play::metadata::Value::Short(9))
        );
    }
}
