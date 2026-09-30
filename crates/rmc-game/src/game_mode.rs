//! Game-state reason 3 conversion from NetHandlerPlayClient.
pub fn from_game_state(value: f32) -> u8 {
    let rounded = value + 0.5;
    let integer = rounded as i32;
    // MathHelper.floor_float uses Java's saturating float cast followed by
    // a wrapping subtraction, including nonfinite and out-of-range inputs.
    let id = if rounded < integer as f32 {
        integer.wrapping_sub(1)
    } else {
        integer
    };
    match id {
        1..=3 => id as u8,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_rounding_matches_java_handler_boundaries() {
        for (value, expected) in [
            (-0.5, 0),
            (0.499, 0),
            (0.5, 1),
            (1.499, 1),
            (1.5, 2),
            (2.499, 2),
            (2.5, 3),
            (3.499, 3),
            (3.5, 0),
            (f32::NAN, 0),
            (f32::INFINITY, 0),
            (f32::NEG_INFINITY, 0),
        ] {
            assert_eq!(from_game_state(value), expected, "value={value}");
        }
    }
}
