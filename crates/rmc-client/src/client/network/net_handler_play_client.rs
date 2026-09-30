//! Main-thread play packet effects, corresponding to MCP919 NetHandlerPlayClient.
//! Transport decoding remains in rmc-net; simulation consumes ordered effects.
use rmc_game::combat::CombatState;
use rmc_game::player::Vec3;
use rmc_game::simulation::SimulationEvent;
use rmc_net::codec::play::PlayClientboundPacket;
use rmc_net::session::PlayerPose;
use rmc_render::ChunkMeshPipeline;
use rmc_world::WorldSnapshot;

pub(crate) fn apply_inbound_play_packet(
    packet: &PlayClientboundPacket,
    _local_pose: PlayerPose,
    player_entity_id: Option<i32>,
    world: &mut WorldSnapshot,
    mesh_pipeline: &mut ChunkMeshPipeline,
    combat: &mut CombatState,
    pending_simulation_events: &mut Vec<SimulationEvent>,
) -> Result<(), String> {
    if let Some(changes) = world
        .apply_play_packet(packet)
        .map_err(|error| format!("failed to apply world packet: {error:?}"))?
    {
        mesh_pipeline.apply_world_changes(&changes);
    }

    if let PlayClientboundPacket::PlayerPositionAndLook(packet) = packet {
        pending_simulation_events.push(SimulationEvent::Teleport {
            position: Vec3::new(packet.x, packet.y, packet.z),
            yaw: packet.yaw,
            pitch: packet.pitch,
            flags: packet.flags.bits(),
        });
    }

    if let PlayClientboundPacket::Explosion(packet) = packet {
        if packet.motion.iter().all(|v| v.is_finite()) {
            pending_simulation_events.push(SimulationEvent::AddVelocity(Vec3::new(
                f64::from(packet.motion[0]),
                f64::from(packet.motion[1]),
                f64::from(packet.motion[2]),
            )));
        }
    }
    let combat_update = combat.apply_play_packet(packet, player_entity_id);
    pending_simulation_events.extend(combat_update.simulation_events);
    Ok(())
}
