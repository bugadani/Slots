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

    assert_eq!(5, slots.try_read(0, |&w| w).unwrap());
    assert_eq!(6, slots.try_read(1, |&w| w).unwrap());
    assert_eq!(7, slots.try_read(2, |&w| w).unwrap());
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

#[should_panic()]
#[test]
/// Verify some size bounds: an N long array over IT is not larger than 3 usize + N * IT (as long
/// as IT is larger than two usize and has two niches)
//
// Fails until https://github.com/rust-lang/rust/issues/46213 is resolved (possibly,
// https://github.com/rust-lang/rust/pull/70477 is sufficient). When this starts not failing any
// more, be happy, remove the panic, and figure out how to skip the test on older Rust versions.
fn is_compact() {
    struct TwoNichesIn16Byte {
        n1: u64,
        n2: u32,
        n3: u16,
        n4: u8,
        b: bool,
    }

    assert_eq!(core::mem::size_of::<TwoNichesIn16Byte>(), 16);

    let slots: Slots<TwoNichesIn16Byte, U32> = Slots::new();
    assert_eq!(core::mem::size_of::<Slots<TwoNichesIn16Byte, U32>>(), 32 * 16 + 3 * core::mem::size_of::<usize>());
}
