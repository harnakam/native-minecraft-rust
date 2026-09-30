use rmc_game::mining::*;
use rmc_net::codec::play::*;

fn target(id: u16, x: i32) -> MiningTarget {
    MiningTarget {
        position: BlockPosition::new(x, 64, 0),
        face: 1,
        block_state: id << 4,
    }
}
fn context(tool: i16) -> MiningContext {
    MiningContext {
        held_item: Some(ItemStack::simple(tool, 1, 0)),
        on_ground: true,
        ..MiningContext::default()
    }
}
fn actions(update: &MiningUpdate) -> Vec<DiggingAction> {
    update
        .packets
        .iter()
        .filter_map(|packet| {
            if let PlayServerboundPacket::PlayerDigging(packet) = packet {
                Some(packet.action)
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn diamond_pick_stone_finishes_after_six_damage_ticks() {
    let mut mining = MiningState::default();
    let ctx = context(278);
    let first = mining.update(true, true, 1, Some(target(1, 0)), &ctx);
    assert_eq!(actions(&first), vec![DiggingAction::StartDestroyBlock]);
    assert!(first.destroyed.is_empty());
    for _ in 0..4 {
        assert!(mining
            .update(false, true, 1, Some(target(1, 0)), &ctx)
            .destroyed
            .is_empty());
    }
    let last = mining.update(false, true, 1, Some(target(1, 0)), &ctx);
    assert_eq!(actions(&last), vec![DiggingAction::StopDestroyBlock]);
    assert_eq!(last.destroyed, vec![BlockPosition::new(0, 64, 0)]);
    assert!(mining.progress().is_none());
}
#[test]
fn target_change_aborts_old_face_then_starts_new_and_release_uses_down() {
    let mut mining = MiningState::default();
    let ctx = context(278);
    mining.update(true, true, 1, Some(target(1, 0)), &ctx);
    let switched = mining.update(false, true, 1, Some(target(1, 1)), &ctx);
    assert_eq!(
        actions(&switched),
        vec![
            DiggingAction::AbortDestroyBlock,
            DiggingAction::StartDestroyBlock
        ]
    );
    let released = mining.update(false, false, 1, None, &ctx);
    assert_eq!(actions(&released), vec![DiggingAction::AbortDestroyBlock]);
    assert!(
        matches!(&released.packets[0],PlayServerboundPacket::PlayerDigging(packet) if packet.face==0 && packet.position.x==1)
    );
}
#[test]
fn unbreakable_block_never_sends_finish_and_instant_block_only_sends_start() {
    let mut mining = MiningState::default();
    let ctx = context(278);
    let unbreakable = mining.update(true, true, 1000, Some(target(7, 0)), &ctx);
    assert_eq!(
        actions(&unbreakable),
        vec![DiggingAction::StartDestroyBlock]
    );
    assert!(unbreakable.destroyed.is_empty());
    mining.reset();
    let instant = mining.update(true, true, 1, Some(target(165, 0)), &ctx);
    assert_eq!(actions(&instant), vec![DiggingAction::StartDestroyBlock]);
    assert_eq!(instant.destroyed.len(), 1);
    assert_eq!(instant.packets.len(), 1);
}
#[test]
fn harvesting_tool_tier_and_environment_change_relative_hardness() {
    assert_eq!(
        relative_hardness(56 << 4, &context(285)),
        12.0_f32 / 3.0 / 100.0
    );
    assert_eq!(
        relative_hardness(56 << 4, &context(278)),
        8.0_f32 / 3.0 / 30.0
    );
    let mut slow = context(278);
    slow.fatigue = Some(1);
    slow.underwater = true;
    slow.on_ground = false;
    assert_eq!(
        relative_hardness(1 << 4, &slow),
        8.0_f32 * 0.09 / 5.0 / 5.0 / 1.5 / 30.0
    );
}
#[test]
fn efficiency_is_read_from_real_item_nbt() {
    let mut ctx = context(278);
    ctx.held_item.as_mut().unwrap().nbt = Some(vec![
        10, 0, 0, 9, 0, 4, b'e', b'n', b'c', b'h', 10, 0, 0, 0, 1, 2, 0, 2, b'i', b'd', 0, 32, 2,
        0, 3, b'l', b'v', b'l', 0, 2, 0, 0,
    ]);
    assert_eq!(relative_hardness(1 << 4, &ctx), 13.0_f32 / 1.5 / 30.0);
}
#[test]
fn creative_sword_and_spectator_do_not_predict_block_destruction() {
    let mut mining = MiningState::default();
    let mut ctx = context(276);
    ctx.game_mode = 1;
    let sword = mining.update(true, true, 1, Some(target(1, 0)), &ctx);
    assert!(sword.destroyed.is_empty());
    ctx.game_mode = 3;
    assert!(mining
        .update(true, true, 1, Some(target(1, 0)), &ctx)
        .packets
        .is_empty());
}
