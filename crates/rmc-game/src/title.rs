//! Server title state matching MCP919 GuiIngame and NetHandlerPlayClient.
use rmc_net::codec::play::TitlePacket;
#[derive(Clone, Debug, PartialEq)]
pub struct TitleState {
    pub title_json: String,
    pub subtitle_json: String,
    pub remaining_ticks: i32,
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}
impl Default for TitleState {
    fn default() -> Self {
        Self {
            title_json: String::new(),
            subtitle_json: String::new(),
            remaining_ticks: 0,
            fade_in: 10,
            stay: 70,
            fade_out: 20,
        }
    }
}
impl TitleState {
    fn duration(&self) -> i32 {
        self.fade_in
            .wrapping_add(self.stay)
            .wrapping_add(self.fade_out)
    }
    pub fn receive(&mut self, packet: &TitlePacket) {
        match packet {
            TitlePacket::Title(text) => {
                self.title_json.clone_from(text);
                self.remaining_ticks = self.duration();
            }
            TitlePacket::Subtitle(text) => self.subtitle_json.clone_from(text),
            TitlePacket::Times {
                fade_in,
                stay,
                fade_out,
            } => {
                if *fade_in < 0 && *stay < 0 && *fade_out < 0 {
                    self.clear();
                    return;
                }
                if *fade_in >= 0 {
                    self.fade_in = *fade_in;
                }
                if *stay >= 0 {
                    self.stay = *stay;
                }
                if *fade_out >= 0 {
                    self.fade_out = *fade_out;
                }
                if self.remaining_ticks > 0 {
                    self.remaining_ticks = self.duration();
                }
            }
            TitlePacket::Clear => self.clear(),
            // The MCP handler calls displayTitle("", "", ...), whose title
            // branch leaves the subtitle intact, then restores default times.
            TitlePacket::Reset => {
                self.title_json.clear();
                self.remaining_ticks = self.duration();
                self.fade_in = 10;
                self.stay = 70;
                self.fade_out = 20;
            }
        }
    }
    fn clear(&mut self) {
        self.title_json.clear();
        self.subtitle_json.clear();
        self.remaining_ticks = 0;
    }
    pub fn advance(&mut self, ticks: usize) {
        if self.remaining_ticks > 0 {
            self.remaining_ticks = self
                .remaining_ticks
                .saturating_sub(ticks.min(i32::MAX as usize) as i32);
            if self.remaining_ticks <= 0 {
                self.clear();
            }
        }
    }
    pub fn alpha(&self, partial_ticks: f32) -> u8 {
        if self.remaining_ticks <= 0 {
            return 0;
        }
        let remaining = self.remaining_ticks as f32 - partial_ticks;
        let mut alpha = 255;
        if self.remaining_ticks > self.fade_out.wrapping_add(self.stay) {
            alpha = ((self.duration() as f32 - remaining) * 255.0 / self.fade_in as f32) as i32;
        }
        if self.remaining_ticks <= self.fade_out {
            alpha = (remaining * 255.0 / self.fade_out as f32) as i32;
        }
        alpha.clamp(0, 255) as u8
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subtitle_waits_for_title_and_timing_changes_restart_active_display() {
        let mut state = TitleState::default();
        state.receive(&TitlePacket::Subtitle("sub".into()));
        assert_eq!(state.remaining_ticks, 0);
        state.receive(&TitlePacket::Title("title".into()));
        assert_eq!(state.remaining_ticks, 100);
        assert_eq!(state.alpha(0.0), 0);
        assert_eq!(state.alpha(0.5), 12);
        state.advance(10);
        assert_eq!(state.alpha(0.0), 255);
        state.advance(80);
        assert_eq!(state.alpha(0.0), 127);
        state.receive(&TitlePacket::Times {
            fade_in: -1,
            stay: 5,
            fade_out: 3,
        });
        assert_eq!(state.remaining_ticks, 18);
        state.advance(18);
        assert_eq!(state.subtitle_json, "");
        assert_eq!(state.alpha(0.0), 0);
    }
    #[test]
    fn clear_and_reset_follow_mcp_handler_order() {
        let mut state = TitleState::default();
        state.receive(&TitlePacket::Times {
            fade_in: 1,
            stay: 2,
            fade_out: 3,
        });
        state.receive(&TitlePacket::Subtitle("sub".into()));
        state.receive(&TitlePacket::Reset);
        assert_eq!(state.remaining_ticks, 6);
        assert_eq!(state.subtitle_json, "sub");
        assert_eq!((state.fade_in, state.stay, state.fade_out), (10, 70, 20));
        state.receive(&TitlePacket::Clear);
        assert_eq!(state.remaining_ticks, 0);
        assert!(state.subtitle_json.is_empty());
    }
}
