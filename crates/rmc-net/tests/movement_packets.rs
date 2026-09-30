use rmc_net::codec::play::*;

#[test]
fn server_game_mode_change_uses_unsigned_reason_and_float_value() {
    let bytes = [0x2b, 3, 0x3f, 0x80, 0, 0];
    let packet = PlayClientboundPacket::ChangeGameState(ChangeGameStatePacket {
        reason: 3,
        value: 1.0,
    });
    assert_eq!(
        PlayClientboundPacket::decode_packet(&bytes).unwrap(),
        packet
    );
    assert_eq!(packet.encode_packet().unwrap().packet_bytes(), bytes);
}

#[test]
fn server_hotbar_change_is_a_signed_byte_not_a_short() {
    for slot in [-1, 0, 8, 9] {
        let bytes = [0x09, slot as u8];
        let packet =
            PlayClientboundPacket::HeldItemChange(HeldItemChangeClientboundPacket { slot });
        assert_eq!(
            PlayClientboundPacket::decode_packet(&bytes).unwrap(),
            packet
        );
        assert_eq!(packet.encode_packet().unwrap().packet_bytes(), bytes);
    }
}

#[test]
fn abilities_decode_wire_flags_and_ieee_speeds() {
    let bytes = [0x39, 0x06, 0x3d, 0x4c, 0xcc, 0xcd, 0x3d, 0xcc, 0xcc, 0xcd];
    let packet = PlayClientboundPacket::decode_packet(&bytes).unwrap();
    assert_eq!(
        packet,
        PlayClientboundPacket::PlayerAbilities(PlayerAbilitiesPacket {
            flags: 6,
            flying_speed: 0.05,
            walking_speed: 0.1
        })
    );
    assert_eq!(packet.encode_packet().unwrap().packet_bytes(), bytes);
}
#[test]
fn attribute_collection_lengths_are_bounded_before_allocation() {
    assert!(PlayClientboundPacket::decode_packet(&[0x20, 1, 0xff, 0xff, 0xff, 0xff]).is_err());
    assert!(PlayClientboundPacket::decode_packet(&[0x20, 1, 0, 0, 1, 1]).is_err());
}
#[test]
fn effects_preserve_duration_amplifier_and_particle_flags() {
    let bytes = [0x1d, 12, 8, 1, 0xac, 2, 1];
    assert_eq!(
        PlayClientboundPacket::decode_packet(&bytes).unwrap(),
        PlayClientboundPacket::EntityEffect(EntityEffectPacket {
            entity_id: 12,
            effect_id: 8,
            amplifier: 1,
            duration: 300,
            hide_particles: 1
        })
    );
}
#[test]
fn respawn_and_inventory_status_have_vanilla_wire_actions() {
    for action in 0..=2 {
        let bytes = [0x16, action as u8];
        assert_eq!(
            PlayServerboundPacket::decode_packet(&bytes).unwrap(),
            PlayServerboundPacket::ClientStatus(action)
        );
        assert_eq!(
            PlayServerboundPacket::ClientStatus(action)
                .encode_packet()
                .unwrap()
                .packet_bytes(),
            bytes
        );
    }
}

#[test]
fn flight_change_serverbound_has_c13_flags_and_speeds() {
    let bytes = [0x13, 0x0f, 0x3d, 0x4c, 0xcc, 0xcd, 0x3d, 0xcc, 0xcc, 0xcd];
    let expected = PlayServerboundPacket::PlayerAbilities(PlayerAbilitiesPacket {
        flags: 15,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    assert_eq!(
        PlayServerboundPacket::decode_packet(&bytes).unwrap(),
        expected
    );
    assert_eq!(expected.encode_packet().unwrap().packet_bytes(), bytes);
}
