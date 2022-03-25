//! **Ignore me!** This file contains implementation details
//! that are conceptually private but must be technically public.

#[doc(hidden)]
pub enum Entry<IT> {
    Used(IT),
    EmptyNext(usize),
    EmptyLast,
}

impl<IT> Default for Entry<IT> {
    fn default() -> Self {
        Entry::EmptyLast
    }
}
