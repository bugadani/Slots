use crate::private::Entry;

/// Read-only iterator to access all occupied slots.
pub struct Iter<'a, IT> {
    inner: core::slice::Iter<'a, Entry<IT>>,
}

impl<'a, IT> Iter<'a, IT> {
    pub fn from_iter(inner: &'a [Entry<IT>]) -> Self {
        Self {
            inner: inner.into_iter(),
        }
    }
}

impl<'a, IT> Iterator for Iter<'a, IT> {
    type Item = &'a IT;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(slot) = self.inner.next() {
            if let Entry::Used(ref item) = slot {
                return Some(item);
            }
        }
        None
    }
}

/// Read-write iterator to access all occupied slots.
pub struct IterMut<'a, IT> {
    inner: core::slice::IterMut<'a, Entry<IT>>,
}

impl<'a, IT> IterMut<'a, IT> {
    pub fn from_iter(inner: &'a mut [Entry<IT>]) -> Self {
        Self {
            inner: inner.iter_mut(),
        }
    }
}

impl<'a, IT> Iterator for IterMut<'a, IT> {
    type Item = &'a mut IT;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(slot) = self.inner.next() {
            if let Entry::Used(ref mut item) = slot {
                return Some(item);
            }
        }
        None
    }
}

#[cfg(test)]
mod iter_test {
    use crate::{consts::U3, slots::Slots, unrestricted::UnrestrictedSlots};

    #[test]
    fn sanity_check() {
        let mut slots: Slots<_, U3> = Slots::new();

        let _k1 = slots.store(1).unwrap();
        let k2 = slots.store(2).unwrap();
        let _k3 = slots.store(3).unwrap();

        slots.take(k2);

        let mut iter = slots.iter();
        // iterator does not return elements in order of store
        assert_eq!(Some(&3), iter.next());
        assert_eq!(Some(&1), iter.next());
        assert_eq!(None, iter.next());

        for &_ in slots.iter() {}
    }

    #[test]
    fn test_mut() {
        let mut slots: UnrestrictedSlots<_, U3> = UnrestrictedSlots::new();

        let _k1 = slots.store(1).unwrap();
        let k2 = slots.store(2).unwrap();
        let _k3 = slots.store(3).unwrap();

        for k in slots.iter_mut() {
            *k *= 2;
        }

        assert_eq!(Some(4), slots.take(k2));
    }
}