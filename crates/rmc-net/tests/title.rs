use rmc_net::codec::play::{PlayClientboundPacket, TitlePacket};
#[test]
fn title_actions_roundtrip_reject_truncation_and_use_fixed_int_times() {
    let actions = [
        TitlePacket::Title("{}".into()),
        TitlePacket::Subtitle("{}".into()),
        TitlePacket::Times {
            fade_in: 10,
            stay: 70,
            fade_out: 20,
        },
        TitlePacket::Clear,
        TitlePacket::Reset,
    ];
    for (id, action) in actions.into_iter().enumerate() {
        let packet = PlayClientboundPacket::Title(action);
        let bytes = packet.encode_packet().unwrap().packet_bytes();
        assert_eq!(&bytes[..2], &[0x45, id as u8]);
        assert_eq!(
            PlayClientboundPacket::decode_packet(&bytes).unwrap(),
            packet
        );
        for end in 1..bytes.len() {
            assert!(PlayClientboundPacket::decode_packet(&bytes[..end]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(PlayClientboundPacket::decode_packet(&trailing).is_err());
        if id == 2 {
            assert_eq!(bytes, vec![0x45, 2, 0, 0, 0, 10, 0, 0, 0, 70, 0, 0, 0, 20]);
        }
    }
    assert!(PlayClientboundPacket::decode_packet(&[0x45, 5]).is_err());
}
