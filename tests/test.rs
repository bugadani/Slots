use slots::Slots;
use slots::consts::*;

#[test]
fn key_can_be_used_to_read_value() {
    let mut slots: Slots<_, U8> = Slots::new();
    let k1 = slots.store(5).unwrap();

    assert_eq!(5, slots.read(&k1, |&w| w));
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
