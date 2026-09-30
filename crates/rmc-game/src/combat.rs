//! Combat-critical client rules and local feedback handling.

use crate::player::Vec3;
use crate::simulation::{KnockbackImpulse, SimulationEvent};
use rmc_net::codec::play::{
    AnimationPacket, BlockPosition, DiggingAction, EntityActionKind, EntityActionPacket,
    EntityVelocityPacket, PlayClientboundPacket, PlayServerboundPacket, PlayerBlockPlacementPacket,
    PlayerDiggingPacket, RespawnPacket, Slot, UpdateHealthPacket, UseEntityAction, UseEntityPacket,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CombatConfig {
    pub hurt_ticks_on_damage: u8,
    pub swing_before_attack: bool,
}

impl CombatConfig {
    pub fn vanilla() -> Self {
        Self {
            hurt_ticks_on_damage: 10,
            swing_before_attack: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsingItemState {
    pub stack: Slot,
    pub use_ticks: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatSnapshot {
    pub server_sprint_state: bool,
    pub server_sneak_state: bool,
    pub using_item: Option<UsingItemState>,
    pub health: f32,
    pub food_level: i32,
    pub saturation: f32,
    pub hurt_ticks: u8,
    pub last_velocity: Vec3,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CombatUpdate {
    pub outbound_packets: Vec<PlayServerboundPacket>,
    pub simulation_events: Vec<SimulationEvent>,
    pub health_changed: bool,
    pub hurt_feedback: bool,
}

pub struct CombatState {
    config: CombatConfig,
    server_sprint_state: bool,
    server_sneak_state: bool,
    using_item: Option<UsingItemState>,
    health: f32,
    food_level: i32,
    saturation: f32,
    hurt_ticks: u8,
    last_velocity: Vec3,
}

impl CombatState {
    pub fn new(config: CombatConfig) -> Self {
        Self {
            config,
            server_sprint_state: false,
            server_sneak_state: false,
            using_item: None,
            health: 20.0,
            food_level: 20,
            saturation: 5.0,
            hurt_ticks: 0,
            last_velocity: Vec3::ZERO,
        }
    }

    pub fn snapshot(&self) -> CombatSnapshot {
        CombatSnapshot {
            server_sprint_state: self.server_sprint_state,
            server_sneak_state: self.server_sneak_state,
            using_item: self.using_item.clone(),
            health: self.health,
            food_level: self.food_level,
            saturation: self.saturation,
            hurt_ticks: self.hurt_ticks,
            last_velocity: self.last_velocity,
        }
    }

    pub fn tick_feedback(&mut self) {
        if let Some(using_item) = &mut self.using_item {
            using_item.use_ticks += 1;
        }

        if self.hurt_ticks > 0 {
            self.hurt_ticks -= 1;
        }
    }

    pub fn sync_action_state(
        &mut self,
        player_entity_id: i32,
        sprinting: bool,
        sneaking: bool,
    ) -> Vec<PlayServerboundPacket> {
        let mut packets = Vec::new();

        if sprinting != self.server_sprint_state {
            packets.push(PlayServerboundPacket::EntityAction(EntityActionPacket {
                entity_id: player_entity_id,
                action: if sprinting {
                    EntityActionKind::StartSprinting
                } else {
                    EntityActionKind::StopSprinting
                },
                aux_data: 0,
            }));
            self.server_sprint_state = sprinting;
        }

        if sneaking != self.server_sneak_state {
            packets.push(PlayServerboundPacket::EntityAction(EntityActionPacket {
                entity_id: player_entity_id,
                action: if sneaking {
                    EntityActionKind::StartSneaking
                } else {
                    EntityActionKind::StopSneaking
                },
                aux_data: 0,
            }));
            self.server_sneak_state = sneaking;
        }

        packets
    }

    pub fn attack_entity(&self, target_entity_id: i32) -> Vec<PlayServerboundPacket> {
        let mut packets = Vec::new();

        if self.config.swing_before_attack {
            packets.push(PlayServerboundPacket::Animation(AnimationPacket));
        }

        packets.push(PlayServerboundPacket::UseEntity(UseEntityPacket {
            entity_id: target_entity_id,
            action: UseEntityAction::Attack,
            target: None,
        }));

        packets
    }

    pub fn interact_entity(&self, target_entity_id: i32) -> Vec<PlayServerboundPacket> {
        vec![PlayServerboundPacket::UseEntity(UseEntityPacket {
            entity_id: target_entity_id,
            action: UseEntityAction::Interact,
            target: None,
        })]
    }

    pub fn interact_at_entity(
        &self,
        target_entity_id: i32,
        hit: [f32; 3],
    ) -> Vec<PlayServerboundPacket> {
        vec![PlayServerboundPacket::UseEntity(UseEntityPacket {
            entity_id: target_entity_id,
            action: UseEntityAction::InteractAt,
            target: Some(hit),
        })]
    }

    pub fn start_using_item(&mut self, held_item: Slot) -> Vec<PlayServerboundPacket> {
        self.using_item = Some(UsingItemState {
            stack: held_item.clone(),
            use_ticks: 0,
        });

        vec![PlayServerboundPacket::PlayerBlockPlacement(
            PlayerBlockPlacementPacket::use_item(held_item),
        )]
    }

    pub fn release_using_item(&mut self) -> Vec<PlayServerboundPacket> {
        if self.using_item.is_none() {
            return Vec::new();
        }

        self.using_item = None;

        vec![PlayServerboundPacket::PlayerDigging(PlayerDiggingPacket {
            action: DiggingAction::ReleaseUseItem,
            position: BlockPosition::ORIGIN,
            face: 0,
        })]
    }

    pub fn apply_health_update(&mut self, packet: &UpdateHealthPacket) -> CombatUpdate {
        let took_damage = packet.health < self.health;
        self.health = packet.health;
        self.food_level = packet.food_level;
        self.saturation = packet.saturation;

        if took_damage {
            self.hurt_ticks = self.config.hurt_ticks_on_damage;
        }

        CombatUpdate {
            health_changed: true,
            hurt_feedback: took_damage,
            ..CombatUpdate::default()
        }
    }

    pub fn apply_respawn(&mut self, _packet: &RespawnPacket) -> CombatUpdate {
        self.server_sprint_state = false;
        self.server_sneak_state = false;
        self.using_item = None;
        self.hurt_ticks = 0;
        self.last_velocity = Vec3::ZERO;
        CombatUpdate::default()
    }

    pub fn apply_entity_velocity(
        &mut self,
        packet: &EntityVelocityPacket,
        player_entity_id: Option<i32>,
    ) -> CombatUpdate {
        let velocity = Vec3::new(packet.motion_x(), packet.motion_y(), packet.motion_z());
        let mut update = CombatUpdate::default();

        if Some(packet.entity_id) == player_entity_id {
            self.last_velocity = velocity;
            update
                .simulation_events
                .push(SimulationEvent::Knockback(KnockbackImpulse {
                    velocity,
                    resets_sprint: false,
                }));
        }

        update
    }

    pub fn apply_play_packet(
        &mut self,
        packet: &PlayClientboundPacket,
        player_entity_id: Option<i32>,
    ) -> CombatUpdate {
        match packet {
            PlayClientboundPacket::UpdateHealth(packet) => self.apply_health_update(packet),
            PlayClientboundPacket::Respawn(packet) => self.apply_respawn(packet),
            PlayClientboundPacket::EntityVelocity(packet) => {
                self.apply_entity_velocity(packet, player_entity_id)
            }
            _ => CombatUpdate::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CombatConfig, CombatState};
    use crate::simulation::SimulationEvent;
    use rmc_net::codec::play::{
        AnimationPacket, EntityActionKind, EntityVelocityPacket, PlayServerboundPacket,
        PlayerDiggingPacket, UpdateHealthPacket, UseEntityAction,
    };

    #[test]
    fn syncs_sprint_and_sneak_state_in_vanilla_order() {
        let mut combat = CombatState::new(CombatConfig::vanilla());
        let packets = combat.sync_action_state(12, true, true);

        assert_eq!(packets.len(), 2);
        assert!(matches!(
            &packets[0],
            PlayServerboundPacket::EntityAction(packet)
                if packet.action == EntityActionKind::StartSprinting
        ));
        assert!(matches!(
            &packets[1],
            PlayServerboundPacket::EntityAction(packet)
                if packet.action == EntityActionKind::StartSneaking
        ));
    }

    #[test]
    fn attacks_with_animation_before_use_entity() {
        let combat = CombatState::new(CombatConfig::vanilla());
        let packets = combat.attack_entity(44);

        assert_eq!(packets.len(), 2);
        assert_eq!(
            packets[0],
            PlayServerboundPacket::Animation(AnimationPacket)
        );
        assert!(matches!(
            &packets[1],
            PlayServerboundPacket::UseEntity(packet)
                if packet.entity_id == 44 && packet.action == UseEntityAction::Attack
        ));
    }

    #[test]
    fn tracks_using_item_and_release_packet() {
        let mut combat = CombatState::new(CombatConfig::vanilla());
        let start = combat.start_using_item(None);
        assert_eq!(start.len(), 1);
        assert!(combat.snapshot().using_item.is_some());

        let release = combat.release_using_item();
        assert_eq!(release.len(), 1);
        assert_eq!(combat.snapshot().using_item, None);
        assert!(matches!(
            &release[0],
            PlayServerboundPacket::PlayerDigging(PlayerDiggingPacket { .. })
        ));
    }

    #[test]
    fn damage_update_sets_hurt_feedback() {
        let mut combat = CombatState::new(CombatConfig::vanilla());
        let update = combat.apply_health_update(&UpdateHealthPacket {
            health: 17.0,
            food_level: 19,
            saturation: 4.0,
        });

        assert!(update.health_changed);
        assert!(update.hurt_feedback);
        assert_eq!(combat.snapshot().hurt_ticks, 10);
    }

    #[test]
    fn local_velocity_packet_becomes_knockback_event() {
        let mut combat = CombatState::new(CombatConfig::vanilla());
        let update = combat.apply_entity_velocity(
            &EntityVelocityPacket {
                entity_id: 9,
                velocity_x: 1600,
                velocity_y: 800,
                velocity_z: -400,
            },
            Some(9),
        );

        assert_eq!(update.simulation_events.len(), 1);
        assert!(matches!(
            update.simulation_events[0],
            SimulationEvent::Knockback(_)
        ));
    }
}
