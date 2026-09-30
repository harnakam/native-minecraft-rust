use rmc_net::codec::play::WorldBorderPacket;
use rmc_world::border::WorldBorder;
use std::io::{self, BufRead};
fn main() {
    let mut count = 0;
    let mut bounds_count = 0;
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields[0] == "bounds" {
            assert_eq!(fields.len(), 10);
            let center: f64 = fields[1].parse().unwrap();
            let size = fields[2].parse().unwrap();
            let diameter = fields[3].parse().unwrap();
            let position: f64 = fields[4].parse().unwrap();
            let mut border = WorldBorder::default();
            border.receive_at(
                &WorldBorderPacket::Initialize {
                    x: center,
                    z: center - 0.5,
                    from: diameter,
                    to: diameter,
                    milliseconds: 0,
                    size,
                    warning_distance: 5,
                    warning_time: 15,
                },
                0,
            );
            let bounds = border.bounds_at(0);
            let actual = [
                bounds[0],
                bounds[1],
                bounds[2],
                bounds[3],
                border.closest_distance_at(position, position + 0.5, 0),
            ];
            for (value, expected) in actual.into_iter().zip(&fields[5..]) {
                assert_eq!(
                    value.to_bits(),
                    expected.parse::<u64>().unwrap(),
                    "Java bounds/distance mismatch: {line}"
                );
                bounds_count += 1;
            }
            continue;
        }
        assert_eq!(fields.len(), 6);
        let from = fields[0].parse().unwrap();
        let to = fields[1].parse().unwrap();
        let duration = fields[2].parse().unwrap();
        let before: i64 = fields[3].parse().unwrap();
        let after: i64 = fields[4].parse().unwrap();
        let expected: u64 = fields[5].parse().unwrap();
        assert!(
            after >= before && after - before < 1000,
            "unbounded clock window"
        );
        let matches = (before..=after).any(|elapsed| {
            let mut border = WorldBorder::default();
            border.receive_at(
                &WorldBorderPacket::LerpSize {
                    from,
                    to,
                    milliseconds: duration,
                },
                0,
            );
            border.diameter_at(elapsed).to_bits() == expected
        });
        assert!(matches, "Java diameter mismatch: {line}");
        count += 1;
    }
    assert_eq!(count, 189);
    assert_eq!(bounds_count, 600);
    println!(
        "border oracle: {bounds_count} fixed border bounds/distance observations matched exactly"
    );
    println!("border oracle: {count} diameter observations matched captured Java clock windows");
}
