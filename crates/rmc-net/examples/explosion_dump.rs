use rmc_net::codec::play::{ExplosionPacket, PlayClientboundPacket};
fn main() {
    for (i, x) in [-1.75, -0.75, 0.75, 16777216.0].into_iter().enumerate() {
        let packet = PlayClientboundPacket::Explosion(ExplosionPacket {
            x,
            y: 64.25,
            z: 8.5,
            strength: 2.0,
            records: vec![[-2, 1, 0], [127, -128, -128]],
            motion: [0.25, 0.5, -0.25],
        });
        let bytes = packet.encode_packet().unwrap().packet_bytes();
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        println!("explosion {i} wire {hex}");
        let PlayClientboundPacket::Explosion(decoded) =
            PlayClientboundPacket::decode_packet(&bytes).unwrap()
        else {
            unreachable!()
        };
        for (k, pos) in decoded.affected_positions().enumerate() {
            println!("explosion {i} pos{k} {},{},{}", pos.x, pos.y, pos.z);
        }
    }
}
