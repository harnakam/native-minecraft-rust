use rmc_game::{
    camera::CameraState,
    input::MovementInput,
    player::Vec3,
    simulation::{AuthoritativePlayerState, LocalSimulationLayer, SimulationConfig},
};
use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
use rmc_world::{WorldConfig, WorldSnapshot};
fn put(world: &mut WorldSnapshot, x: i32, y: i32, z: i32, id: i32, meta: i32) {
    world
        .apply_block_change(&BlockChangePacket {
            position: BlockPosition { x, y, z },
            block_state_id: (id << 4) | meta,
        })
        .unwrap();
}
fn main() {
    for name in [
        "walk",
        "diagonal",
        "sprint",
        "jump",
        "ice",
        "soul_sand",
        "slime",
        "water",
        "lava",
        "ladder",
        "web",
        "slab",
    ] {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        let floor = match name {
            "ice" => 79,
            "soul_sand" => 88,
            "slime" => 165,
            _ => 1,
        };
        for x in -32..64 {
            for z in -32..64 {
                put(&mut world, x, 63, z, floor, 0);
                if matches!(name, "water" | "lava") {
                    for y in 64..66 {
                        put(&mut world, x, y, z, if name == "water" { 9 } else { 11 }, 0);
                    }
                }
            }
        }
        if name == "ladder" {
            for y in 64..80 {
                put(&mut world, 8, y, 8, 65, 5);
                put(&mut world, 9, y, 8, 1, 0);
            }
        }
        if name == "web" {
            for y in 64..66 {
                for z in 8..20 {
                    put(&mut world, 8, y, z, 30, 0);
                }
            }
        }
        if name == "slab" {
            for x in 0..16 {
                put(&mut world, x, 64, 9, 44, 0);
                put(&mut world, x, 64, 10, 1, 0);
            }
        }
        let mut sim = LocalSimulationLayer::new(SimulationConfig::vanilla());
        sim.apply_authoritative_state(AuthoritativePlayerState {
            position: Vec3::new(
                8.5,
                if name == "soul_sand" {
                    63.875
                } else if name == "slime" {
                    68.0
                } else {
                    64.0
                },
                8.5,
            ),
            velocity: Vec3::new(0.0, if name == "slime" { -0.4 } else { 0.0 }, 0.0),
            on_ground: name != "slime",
        });
        let camera = CameraState {
            yaw: if name == "diagonal" {
                37.25
            } else if matches!(name, "sprint" | "ladder") {
                -90.0
            } else {
                0.0
            },
            ..CameraState::default()
        };
        let movement = MovementInput {
            forward: 1.0,
            strafe: if name == "diagonal" { 1.0 } else { 0.0 },
            sprint: name == "sprint",
            jump: name == "jump",
            ..MovementInput::default()
        };
        for tick in 1..=80 {
            sim.tick_with_world(movement, camera, 0, &world);
            let p = sim.player().position;
            let v = sim.velocity();
            println!(
                "travel {name} {tick} {} {} {} {} {} {} {}",
                p.x,
                p.y,
                p.z,
                v.x,
                v.y,
                v.z,
                sim.player().on_ground
            );
        }
    }
}
