//! Client-side mining prediction and protocol 47 start/abort/finish transitions.
use rmc_net::codec::play::{
    BlockPosition, DiggingAction, PlayServerboundPacket, PlayerDiggingPacket, Slot,
};

#[derive(Clone, Debug, Default)]
pub struct MiningContext {
    pub held_item: Slot,
    pub game_mode: u8,
    pub on_ground: bool,
    pub underwater: bool,
    pub aqua_affinity: bool,
    pub haste: Option<u8>,
    pub fatigue: Option<u8>,
    pub adventure_can_destroy: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiningTarget {
    pub position: BlockPosition,
    pub face: u8,
    pub block_state: u16,
}
#[derive(Default, Debug)]
pub struct MiningUpdate {
    pub packets: Vec<PlayServerboundPacket>,
    pub destroyed: Vec<BlockPosition>,
}

pub fn relative_hardness(state: u16, context: &MiningContext) -> f32 {
    let Some((hardness, hand)) = crate::mining_properties::block_properties(state >> 4) else {
        return 0.0;
    };
    if hardness < 0.0 {
        return 0.0;
    }
    let (mut speed, harvest) = crate::mining_properties::tool_properties(
        state >> 4,
        context.held_item.as_ref().map_or(-1, |item| item.item_id),
        hand,
    );
    if speed > 1.0 {
        let level = context
            .held_item
            .as_ref()
            .map_or(0, |item| item.enchantment_level(32));
        if level > 0 {
            speed += (level.wrapping_mul(level).wrapping_add(1)) as f32;
        }
    }
    if let Some(amplifier) = context.haste {
        speed *= 1.0 + (amplifier as f32 + 1.0) * 0.2;
    }
    if let Some(amplifier) = context.fatigue {
        speed *= match amplifier {
            0 => 0.3,
            1 => 0.09,
            2 => 0.0027,
            _ => 0.00081,
        };
    }
    if context.underwater && !context.aqua_affinity {
        speed /= 5.0;
    }
    if !context.on_ground {
        speed /= 5.0;
    }
    speed / hardness / if harvest { 30.0 } else { 100.0 }
}

pub fn adventure_can_destroy(state: u16, item: Option<&rmc_net::codec::play::ItemStack>) -> bool {
    let Some(name) = crate::mining_properties::block_name(state >> 4) else {
        return false;
    };
    let Some(bytes) = item.and_then(|item| item.nbt.as_deref()) else {
        return false;
    };
    let Ok(tag) = rmc_net::nbt::parse(bytes) else {
        return false;
    };
    let Some(list) = tag.get("CanDestroy").and_then(rmc_net::nbt::Tag::list) else {
        return false;
    };
    list.iter().any(|value| {
        value.string_equals(name)
            || name
                .strip_prefix("minecraft:")
                .is_some_and(|name| value.string_equals(name))
            || value.string_equals(&(state >> 4).to_string())
    })
}

#[derive(Default)]
pub struct MiningState {
    target: Option<MiningTarget>,
    item: Slot,
    damage: f32,
    delay: u8,
}
impl MiningState {
    pub fn progress(&self) -> Option<(BlockPosition, f32)> {
        self.target.map(|target| (target.position, self.damage))
    }
    pub fn reset(&mut self) {
        self.target = None;
        self.damage = 0.0;
        self.delay = 0;
        self.item = None;
    }
    pub fn update(
        &mut self,
        pressed: bool,
        held: bool,
        ticks: usize,
        target: Option<MiningTarget>,
        context: &MiningContext,
    ) -> MiningUpdate {
        let mut result = MiningUpdate::default();
        if context.game_mode == 3 || (context.game_mode == 2 && !context.adventure_can_destroy) {
            self.abort(0, &mut result);
            return result;
        }
        if pressed {
            if let Some(target) = target {
                self.start(target, context, &mut result);
            }
        }
        for _ in 0..ticks {
            if !held {
                self.abort(0, &mut result);
                continue;
            }
            let Some(target) = target.filter(|target| {
                target.block_state >> 4 != 0 && !result.destroyed.contains(&target.position)
            }) else {
                self.abort(0, &mut result);
                continue;
            };
            if self.delay > 0 {
                self.delay -= 1;
                result.packets.push(PlayServerboundPacket::Animation(
                    rmc_net::codec::play::AnimationPacket,
                ));
                continue;
            }
            if context.game_mode == 1 {
                self.start(target, context, &mut result);
                result.packets.push(PlayServerboundPacket::Animation(
                    rmc_net::codec::play::AnimationPacket,
                ));
                continue;
            }
            if !self.same_target(target, context) {
                self.start(target, context, &mut result);
            } else {
                self.damage += relative_hardness(target.block_state, context);
                if self.damage >= 1.0 {
                    result.packets.push(dig(
                        DiggingAction::StopDestroyBlock,
                        target.position,
                        target.face,
                    ));
                    result.destroyed.push(target.position);
                    self.target = None;
                    self.damage = 0.0;
                    self.delay = 5;
                }
            }
            result.packets.push(PlayServerboundPacket::Animation(
                rmc_net::codec::play::AnimationPacket,
            ));
        }
        result
    }
    fn same_target(&self, target: MiningTarget, context: &MiningContext) -> bool {
        let item_same = match (&self.item, &context.held_item) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.item_id == b.item_id
                    && a.tags_equal(b)
                    && (a.damage == b.damage
                        || matches!(a.item_id,256..=259|261|267..=279|283..=286|290..=294|298..=317|346|359|398))
            }
            _ => false,
        };
        self.target
            .is_some_and(|previous| previous.position == target.position)
            && item_same
    }
    fn start(&mut self, target: MiningTarget, context: &MiningContext, result: &mut MiningUpdate) {
        if target.block_state >> 4 == 0 {
            return;
        }
        if context.game_mode != 1 && self.same_target(target, context) {
            return;
        }
        self.abort(target.face, result);
        result.packets.push(dig(
            DiggingAction::StartDestroyBlock,
            target.position,
            target.face,
        ));
        if context.game_mode == 1 {
            self.delay = 5;
            if !context
                .held_item
                .as_ref()
                .is_some_and(|item| matches!(item.item_id, 267 | 268 | 272 | 276 | 283))
            {
                result.destroyed.push(target.position);
            }
        } else if relative_hardness(target.block_state, context) >= 1.0 {
            result.destroyed.push(target.position);
        } else {
            self.target = Some(target);
            self.item = context.held_item.clone();
            self.damage = 0.0;
        }
    }
    fn abort(&mut self, face: u8, result: &mut MiningUpdate) {
        if let Some(target) = self.target.take() {
            result
                .packets
                .push(dig(DiggingAction::AbortDestroyBlock, target.position, face));
        }
        self.damage = 0.0;
    }
}
fn dig(action: DiggingAction, position: BlockPosition, face: u8) -> PlayServerboundPacket {
    PlayServerboundPacket::PlayerDigging(PlayerDiggingPacket {
        action,
        position,
        face,
    })
}
