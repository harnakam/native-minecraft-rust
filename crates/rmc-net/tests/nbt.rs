use rmc_net::{
    buffer::PacketReader,
    nbt::{self, Tag},
};

#[test]
fn item_tags_compare_structure_and_preserve_numeric_types() {
    use rmc_net::codec::play::ItemStack;
    let mut a = ItemStack::simple(1, 1, 0);
    let mut b = a.clone();
    a.nbt = Some(vec![10, 0, 0, 1, 0, 1, b'a', 1, 2, 0, 1, b'b', 0, 2, 0]);
    b.nbt = Some(vec![10, 0, 0, 2, 0, 1, b'b', 0, 2, 1, 0, 1, b'a', 1, 0]);
    assert!(a.tags_equal(&b));
    a.nbt = Some(vec![10, 0, 0, 1, 0, 1, b'a', 1, 0]);
    b.nbt = Some(vec![10, 0, 0, 2, 0, 1, b'a', 0, 1, 0]);
    assert!(!a.tags_equal(&b));
}

#[test]
fn empty_list_type_and_unpaired_utf16_units_remain_distinct() {
    assert_ne!(
        nbt::parse(&[9, 0, 0, 1, 0, 0, 0, 0]).unwrap(),
        nbt::parse(&[9, 0, 0, 2, 0, 0, 0, 0]).unwrap()
    );
    assert_ne!(
        nbt::parse(&[8, 0, 0, 0, 3, 0xed, 0xa0, 0x80]).unwrap(),
        nbt::parse(&[8, 0, 0, 0, 3, 0xed, 0xa0, 0x81]).unwrap()
    );
}

#[test]
fn item_wire_nbt_requires_a_compound_and_valid_modified_utf8() {
    assert!(PacketReader::new(&[8, 0, 0, 0, 0]).read_nbt_blob().is_err());
    assert!(PacketReader::new(&[10, 0, 0, 8, 0, 1, b'a', 0, 1, 0xff, 0])
        .read_nbt_blob()
        .is_err());
}

#[test]
fn modified_utf8_decodes_null_and_a_surrogate_pair() {
    let bytes = [
        8, 0, 0, 0, 8, 0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80,
    ];
    assert_eq!(
        nbt::parse(&bytes).unwrap(),
        Tag::String("\0😀".encode_utf16().collect())
    );
}
#[test]
fn truncated_and_oversized_lists_fail_before_allocating() {
    assert!(nbt::parse(&[9, 0, 0, 1, 0x7f, 0xff, 0xff, 0xff]).is_err());
    assert!(nbt::parse(&[9, 0, 0, 2, 0, 0, 0, 1, 0]).is_err());
    assert!(PacketReader::new(&[9, 0, 0, 0, 0, 0, 0, 1])
        .read_nbt_blob()
        .is_err());
}
#[test]
fn both_wire_scanner_and_inspector_bound_nested_compounds() {
    let mut bytes = vec![10, 0, 0];
    for _ in 0..514 {
        bytes.extend([10, 0, 0]);
    }
    bytes.extend(std::iter::repeat_n(0, 515));
    assert!(PacketReader::new(&bytes).read_nbt_blob().is_err());
    assert!(nbt::parse(&bytes).is_err());
}
