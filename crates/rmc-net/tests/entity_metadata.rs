use rmc_net::codec::play::metadata::{decode, Value};
use rmc_net::codec::play::{EntityMetadataPacket, PlayClientboundPacket};
#[test]
fn metadata_packet_and_typed_flags_follow_data_watcher_layout() {
    let data = vec![0, 34, 0x46, 0, 0, 0, 9, 127];
    let packet = PlayClientboundPacket::EntityMetadata(EntityMetadataPacket {
        entity_id: 300,
        metadata: data.clone(),
    });
    let encoded = packet.encode_packet().unwrap();
    assert_eq!(encoded.packet_id, 0x1c);
    assert_eq!(
        PlayClientboundPacket::decode_body(0x1c, &encoded.body).unwrap(),
        packet
    );
    assert_eq!(decode(&data).unwrap()[0], (0, Value::Byte(34)));
    assert_eq!(decode(&data).unwrap()[1], (6, Value::Int(9)));
    assert!(decode(&[0, 1]).is_err());
    assert!(decode(&[127, 0]).is_err());
}

#[test]
fn all_eight_data_watcher_types_are_decoded() {
    use rmc_net::buffer::PacketWriter;
    let mut w = PacketWriter::new();
    w.write_u8(0);
    w.write_i8(-1);
    w.write_u8(33);
    w.write_i16(-2);
    w.write_u8(66);
    w.write_i32(-3);
    w.write_u8(99);
    w.write_f32(0.5);
    w.write_u8(132);
    w.write_string("hello", 32767).unwrap();
    w.write_u8(165);
    w.write_i16(-1);
    w.write_u8(198);
    for value in [1, -2, 3] {
        w.write_i32(value);
    }
    w.write_u8(231);
    for value in [1.0, 2.0, 3.0] {
        w.write_f32(value);
    }
    w.write_u8(127);
    let values = decode(&w.into_inner()).unwrap();
    assert_eq!(
        values,
        vec![
            (0, Value::Byte(-1)),
            (1, Value::Short(-2)),
            (2, Value::Int(-3)),
            (3, Value::Float(0.5)),
            (4, Value::String("hello".into())),
            (5, Value::Item(None)),
            (6, Value::Position([1, -2, 3])),
            (7, Value::Rotation([1.0, 2.0, 3.0]))
        ]
    );
}
