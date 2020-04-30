use slots::SlotMap;
use slots::consts::*;

#[test]
fn key_reuse_triggers() {
    let mut slots: SlotMap<_, U8> = SlotMap::new();

    let k1 = slots.store(5).unwrap();
    let ptr1 = slots.read(k1, |w| w as *const _).unwrap();

    assert_eq!(slots.take(k1).unwrap(), 5);

    let k2 = slots.store(6).unwrap();
    let ptr2 = slots.read(k2, |w| w as *const _).unwrap();

    // This test relies on the undocumented but present property that after freeing someting, the
    // next allocation goes there right again.
    assert_eq!(ptr1, ptr2);

    assert_ne!(k1, k2);

    assert_eq!(None, slots.read(k1, |w| *w));

    // This, in addition, relies on the current behavior of RawSlots to in the last slot first, and
    // on it starting with generation 1.
    assert_eq!(k1, 8 | 7);
    assert_eq!(k2, 16 | 7);
}

