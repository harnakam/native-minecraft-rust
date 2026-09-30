use rmc_net::codec::play::{EntityEquipmentPacket, ItemStack, PlayClientboundPacket};

#[test]
fn equipment_packet_round_trips_slot_item_and_null() {
    for item in [None, Some(ItemStack::simple(310, 1, 17))] {
        let packet = PlayClientboundPacket::EntityEquipment(EntityEquipmentPacket {
            entity_id: 300,
            slot: 4,
            item,
        });
        let encoded = packet.encode_packet().unwrap();
        assert_eq!(encoded.packet_id, 4);
        assert_eq!(&encoded.body[..4], &[172, 2, 0, 4]);
        assert_eq!(
            PlayClientboundPacket::decode_body(4, &encoded.body).unwrap(),
            packet
        );
    }
    assert!(PlayClientboundPacket::decode_body(4, &[1, 0]).is_err());
}
