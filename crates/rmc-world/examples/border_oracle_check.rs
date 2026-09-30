use rmc_net::codec::play::WorldBorderPacket;
use rmc_world::border::WorldBorder;
use std::io::{self, BufRead};
fn main() {
    let mut count = 0;
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let fields: Vec<_> = line.split_whitespace().collect();
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
    println!("border oracle: {count} diameter observations matched captured Java clock windows");
}
