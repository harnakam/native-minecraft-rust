//! RenderGlobal destroy progress ownership and cloud-tick expiry.
use rmc_net::codec::play::{BlockBreakAnimationPacket, BlockPosition};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq)]
pub struct DestroyProgress {
    pub position: BlockPosition,
    pub stage: u8,
    last_update: i32,
}
#[derive(Clone, Debug, Default)]
pub struct BlockDamage {
    tick: i32,
    entries: BTreeMap<i32, DestroyProgress>,
}
impl BlockDamage {
    pub fn entries(&self) -> &BTreeMap<i32, DestroyProgress> {
        &self.entries
    }
    pub fn receive(&mut self, p: &BlockBreakAnimationPacket) {
        if p.progress < 10 {
            self.entries.insert(
                p.breaker_id,
                DestroyProgress {
                    position: p.position,
                    stage: p.progress,
                    last_update: self.tick,
                },
            );
        } else {
            self.entries.remove(&p.breaker_id);
        }
    }
    pub fn advance(&mut self, ticks: usize) {
        for _ in 0..ticks {
            self.tick = self.tick.wrapping_add(1);
            if self.tick % 20 == 0 {
                let tick = self.tick;
                self.entries
                    .retain(|_, p| tick.wrapping_sub(p.last_update) <= 400);
            }
        }
    }
}
