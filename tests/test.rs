use slots::Slots;
use slots::consts::*;

#[test]
fn key_can_be_used_to_read_value() {
    let mut slots: Slots<_, U8> = Slots::new();
    let k1 = slots.store(5).unwrap();

    assert_eq!(5, slots.read(&k1, |&w| w));
}

#[test]
fn size_can_be_1() {
    let mut slots: Slots<_, U1> = Slots::new();
    let k1 = slots.store(5).unwrap();

    assert_eq!(5, slots.read(&k1, |&w| w));

    assert_eq!(1, slots.count());
    slots.take(k1);
    assert_eq!(0, slots.count());

    // test that we can fill the storage again
    slots.store(6);
    assert_eq!(1, slots.count());
}

#[test]
fn index_can_be_used_to_read_value() {
    let mut slots: Slots<_, U8> = Slots::new();

    slots.store(5).unwrap();
    slots.store(6).unwrap();
    slots.store(7).unwrap();

    assert_eq!(5, slots.try_read(7, |&w| w).unwrap());
    assert_eq!(6, slots.try_read(6, |&w| w).unwrap());
    assert_eq!(7, slots.try_read(5, |&w| w).unwrap());
}

#[test]
fn trying_to_read_missing_element_returns_none() {
    let slots: Slots<u8, U8> = Slots::new();

    assert_eq!(None, slots.try_read(0, |&w| w));
}

#[test]
fn trying_to_read_deleted_element_returns_none() {
    let mut slots: Slots<u8, U8> = Slots::new();

    slots.store(5).unwrap();
    let k = slots.store(6).unwrap();
    slots.store(7).unwrap();

    let idx = k.index(); //k will be consumed

    slots.take(k);

    assert_eq!(None, slots.try_read(idx, |&w| w));
}

#[test]
fn elements_can_be_modified_using_key() {
    let mut slots: Slots<u8, U8> = Slots::new();

    let k = slots.store(5).unwrap();

    assert_eq!(7, slots.modify(&k, |w| {
        *w = *w + 2;
        *w
    }));
    assert_eq!(7, slots.read(&k, |&w| w));
}

#[test]
fn store_returns_err_when_full() {
    let mut slots: Slots<u8, U1> = Slots::new();

    slots.store(5);

    let k2 = slots.store(5);

    assert!(k2.is_err());
}

#[test]
#[cfg(feature = "verify_owner")]
#[should_panic(expected = "Key used in wrong instance")]
fn use_across_slots_verify() {
    let mut a: Slots<u8, U4> = Slots::new();
    let mut b: Slots<u8, U4> = Slots::new();

    let k = a.store(5).expect("There should be room");
    // Store an element in b so we don't get a different panic
    let _ = b.store(6).expect("There should be room");

    b.take(k);
}

#[test]
#[cfg(not(feature = "verify_owner"))]
fn use_across_slots_no_verify() {
    let mut a: Slots<u8, U4> = Slots::new();
    let mut b: Slots<u8, U4> = Slots::new();

    let k = a.store(5).expect("There should be room");
    // Store an element in b so we don't get a different panic
    let _ = b.store(6).expect("There should be room");

    assert_eq!(6, b.take(k));
}

#[should_panic(expected = "Compiled size does not match expected")]
#[test]
/// Verify some size bounds: an N long array over IT is not larger than 3 usize + N * IT (as long
/// as IT is larger than two usize and has two niches)
//
// Fails until https://github.com/rust-lang/rust/issues/46213 is resolved (possibly,
// https://github.com/rust-lang/rust/pull/70477 is sufficient). When this starts not failing any
// more, be happy, remove the panic, and figure out how to skip the test on older Rust versions.
// (If left just goes down but does not reach right, that should be investigated further, as it
// indicates that the optimization was implemented incompletely, or it turns out it is not possible
// for some reasons and needs fixing in the code).
fn is_compact() {
    #[allow(unused)]
    struct TwoNichesIn16Byte {
        n1: u64,
        n2: u32,
        n3: u16,
        n4: u8,
        b: bool,
    }

    assert_eq!(core::mem::size_of::<TwoNichesIn16Byte>(), 16);

    let mut expected_size = 32 * 16 + 3 * core::mem::size_of::<usize>();
    if cfg!(feature = "verify_owner") {
        expected_size += core::mem::size_of::<usize>(); // an extra usize for object id
    }
    assert_eq!(core::mem::size_of::<Slots<TwoNichesIn16Byte, U32>>(), expected_size, "Compiled size does not match expected");
}

#[test]
fn capacity_and_count() {
    let mut slots: Slots<u8, U4> = Slots::new();

    assert_eq!(slots.capacity(), 4);
    assert_eq!(slots.count(), 0);

    let k1 = slots.store(1).unwrap();
    let k2 = slots.store(2).unwrap();

    assert_eq!(slots.count(), 2);

    let k3 = slots.store(3).unwrap();
    let k4 = slots.store(4).unwrap();

    assert_eq!(slots.count(), 4);

    slots.take(k1);
    slots.take(k2);
    slots.take(k3);
    slots.take(k4);

    assert_eq!(slots.count(), 0);
}
