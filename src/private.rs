//! **Ignore me!** This file contains implementation details
//! that are conceptually private but must be technically public.

#[doc(hide)]
pub enum Entry<IT> {
    Used(IT),
    EmptyNext(usize),
    EmptyLast
}
