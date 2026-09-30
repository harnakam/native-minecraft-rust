//! Input frames, key bindings, and movement intent for the M2 shell.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PhysicalInput {
    KeyW,
    KeyA,
    KeyS,
    KeyD,
    Space,
    LeftShift,
    LeftControl,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Escape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundAction {
    MoveForward,
    MoveLeft,
    MoveBack,
    MoveRight,
    Jump,
    Sneak,
    Sprint,
    SelectHotbar(u8),
    ToggleMouseCapture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binding {
    pub input: PhysicalInput,
    pub action: BoundAction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyBindings {
    bindings: Vec<Binding>,
}

impl KeyBindings {
    pub fn vanilla() -> Self {
        Self {
            bindings: vec![
                Binding {
                    input: PhysicalInput::KeyW,
                    action: BoundAction::MoveForward,
                },
                Binding {
                    input: PhysicalInput::KeyA,
                    action: BoundAction::MoveLeft,
                },
                Binding {
                    input: PhysicalInput::KeyS,
                    action: BoundAction::MoveBack,
                },
                Binding {
                    input: PhysicalInput::KeyD,
                    action: BoundAction::MoveRight,
                },
                Binding {
                    input: PhysicalInput::Space,
                    action: BoundAction::Jump,
                },
                Binding {
                    input: PhysicalInput::LeftShift,
                    action: BoundAction::Sneak,
                },
                Binding {
                    input: PhysicalInput::LeftControl,
                    action: BoundAction::Sprint,
                },
                Binding {
                    input: PhysicalInput::Escape,
                    action: BoundAction::ToggleMouseCapture,
                },
                Binding {
                    input: PhysicalInput::Digit1,
                    action: BoundAction::SelectHotbar(0),
                },
                Binding {
                    input: PhysicalInput::Digit2,
                    action: BoundAction::SelectHotbar(1),
                },
                Binding {
                    input: PhysicalInput::Digit3,
                    action: BoundAction::SelectHotbar(2),
                },
                Binding {
                    input: PhysicalInput::Digit4,
                    action: BoundAction::SelectHotbar(3),
                },
                Binding {
                    input: PhysicalInput::Digit5,
                    action: BoundAction::SelectHotbar(4),
                },
                Binding {
                    input: PhysicalInput::Digit6,
                    action: BoundAction::SelectHotbar(5),
                },
                Binding {
                    input: PhysicalInput::Digit7,
                    action: BoundAction::SelectHotbar(6),
                },
                Binding {
                    input: PhysicalInput::Digit8,
                    action: BoundAction::SelectHotbar(7),
                },
                Binding {
                    input: PhysicalInput::Digit9,
                    action: BoundAction::SelectHotbar(8),
                },
            ],
        }
    }

    fn actions_for(&self, input: PhysicalInput) -> impl Iterator<Item = BoundAction> + '_ {
        self.bindings
            .iter()
            .filter(move |binding| binding.input == input)
            .map(|binding| binding.action)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputFrame {
    pub pressed_inputs: Vec<PhysicalInput>,
    pub released_inputs: Vec<PhysicalInput>,
    pub mouse_delta_x: f32,
    pub mouse_delta_y: f32,
    pub hotbar_scroll: i8,
}

impl Default for InputFrame {
    fn default() -> Self {
        Self {
            pressed_inputs: Vec::new(),
            released_inputs: Vec::new(),
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            hotbar_scroll: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MovementInput {
    pub forward: f32,
    pub strafe: f32,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputSnapshot {
    active_inputs: Vec<PhysicalInput>,
    pub mouse_captured: bool,
    pub selected_hotbar_slot: u8,
}

impl Default for InputSnapshot {
    fn default() -> Self {
        Self {
            active_inputs: Vec::new(),
            mouse_captured: false,
            selected_hotbar_slot: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputUpdate {
    pub movement: MovementInput,
    pub selected_hotbar_slot: u8,
    pub mouse_captured: bool,
    pub mouse_delta_x: f32,
    pub mouse_delta_y: f32,
}

impl InputSnapshot {
    pub fn apply_frame(&mut self, bindings: &KeyBindings, frame: &InputFrame) -> InputUpdate {
        for input in &frame.released_inputs {
            self.active_inputs.retain(|active| active != input);
        }

        for input in &frame.pressed_inputs {
            if !self.active_inputs.contains(input) {
                self.active_inputs.push(*input);

                for action in bindings.actions_for(*input) {
                    match action {
                        BoundAction::ToggleMouseCapture => {
                            self.mouse_captured = !self.mouse_captured;
                        }
                        BoundAction::SelectHotbar(slot) => {
                            self.selected_hotbar_slot = slot;
                        }
                        _ => {}
                    }
                }
            }
        }

        if frame.hotbar_scroll != 0 {
            self.selected_hotbar_slot =
                scroll_hotbar_slot(self.selected_hotbar_slot, frame.hotbar_scroll);
        }

        InputUpdate {
            movement: MovementInput {
                forward: axis(
                    self.is_action_active(bindings, BoundAction::MoveForward),
                    self.is_action_active(bindings, BoundAction::MoveBack),
                ),
                strafe: axis(
                    self.is_action_active(bindings, BoundAction::MoveLeft),
                    self.is_action_active(bindings, BoundAction::MoveRight),
                ),
                jump: self.is_action_active(bindings, BoundAction::Jump),
                sneak: self.is_action_active(bindings, BoundAction::Sneak),
                sprint: self.is_action_active(bindings, BoundAction::Sprint),
            },
            selected_hotbar_slot: self.selected_hotbar_slot,
            mouse_captured: self.mouse_captured,
            mouse_delta_x: frame.mouse_delta_x,
            mouse_delta_y: frame.mouse_delta_y,
        }
    }

    fn is_action_active(&self, bindings: &KeyBindings, action: BoundAction) -> bool {
        self.active_inputs.iter().copied().any(|input| {
            bindings
                .actions_for(input)
                .any(|bound_action| bound_action == action)
        })
    }
}

fn axis(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn scroll_hotbar_slot(selected_hotbar_slot: u8, hotbar_scroll: i8) -> u8 {
    let wrapped = (i16::from(selected_hotbar_slot) - i16::from(hotbar_scroll)).rem_euclid(9);
    wrapped as u8
}

#[cfg(test)]
mod tests {
    use super::{InputFrame, InputSnapshot, KeyBindings, PhysicalInput};

    #[test]
    fn toggles_mouse_capture_and_updates_movement() {
        let bindings = KeyBindings::vanilla();
        let mut snapshot = InputSnapshot::default();
        let update = snapshot.apply_frame(
            &bindings,
            &InputFrame {
                pressed_inputs: vec![PhysicalInput::Escape, PhysicalInput::KeyW],
                ..InputFrame::default()
            },
        );

        assert!(update.mouse_captured);
        assert_eq!(update.movement.forward, 1.0);
    }

    #[test]
    fn scrolls_hotbar_with_wraparound() {
        let bindings = KeyBindings::vanilla();
        let mut snapshot = InputSnapshot::default();
        let update = snapshot.apply_frame(
            &bindings,
            &InputFrame {
                hotbar_scroll: 1,
                ..InputFrame::default()
            },
        );

        assert_eq!(update.selected_hotbar_slot, 8);
    }

    #[test]
    fn toggle_actions_are_edge_triggered() {
        let bindings = KeyBindings::vanilla();
        let mut snapshot = InputSnapshot::default();

        let first = snapshot.apply_frame(
            &bindings,
            &InputFrame {
                pressed_inputs: vec![PhysicalInput::Escape],
                ..InputFrame::default()
            },
        );
        let second = snapshot.apply_frame(
            &bindings,
            &InputFrame {
                pressed_inputs: vec![PhysicalInput::Escape],
                ..InputFrame::default()
            },
        );

        assert!(first.mouse_captured);
        assert!(second.mouse_captured);
    }
}
