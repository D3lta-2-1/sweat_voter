use std::ops::{Deref, DerefMut};

/// A simple capsule to track whenever a data get mutated
pub struct MutationTracker<T> {
    inner: T,
    dirty: bool,
}

impl<T> MutationTracker<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            dirty: false,
        }
    }

    pub fn dirty(inner: T, dirty: bool) -> Self {
        Self { inner, dirty }
    }

    pub fn clear_dirty(&mut self) -> bool {
        if self.dirty {
            self.dirty = false;
            true
        } else {
            false
        }
    }
}

impl<T: Default> Default for MutationTracker<T> {
    fn default() -> Self {
        Self {
            inner: T::default(),
            dirty: false,
        }
    }
}

impl<T> Deref for MutationTracker<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// whenever this get mutated, we mark it...
// it safer to let the compiler tag when this happened rather than doing it by hand
impl<T> DerefMut for MutationTracker<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.inner
    }
}
