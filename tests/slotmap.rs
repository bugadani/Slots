#![warn(soft_unstable)]
#![feature(test)]

extern crate test;

use slots::*;
use slots::consts::*;

use test::Bencher;

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

struct Thing {
    x: [usize; 20],
    y: Option<u8>,
}

impl Thing {
    fn new() -> Self {
        Self { x: [10; 20], y: Some(20) }
    }
}

#[bench]
fn bench_slotmap(b: &mut Bencher) {
    b.iter(|| {
        let mut s: slots::SlotMap<_, U128> = SlotMap::new();

        for _ in 0..128 {
            let h = s.store(Thing::new()).ok().unwrap();
            s.read(h, |i| test::black_box(i.y)).unwrap();
        }
    })
}

// This is identical to bench_slotmap except for the type.
// That this becomes *much* faster than slotmap on larger Thing sizes indicates that somewhere, the
// slotmap implementation is flawed in that it moves data around where it should not.
#[bench]
fn bench_rawslots(b: &mut Bencher) {
    b.iter(|| {
        let mut s: slots::RawSlots<_, U128> = RawSlots::new();

        for _ in 0..128 {
            let h = s.store(Thing::new()).ok().unwrap();
            s.read(h, |i| test::black_box(i.y)).unwrap();
        }
    })
}
