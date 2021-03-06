// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data for view widgets

use super::{ListData, ListDataMut, SingleData, SingleDataMut};
#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Wrapper for single-thread shared data
#[derive(Clone, Debug)]
pub struct SharedRc<T: Debug> {
    handle: UpdateHandle,
    data: Rc<RefCell<T>>,
}

impl<T: Default + Debug> Default for SharedRc<T> {
    fn default() -> Self {
        SharedRc {
            handle: UpdateHandle::new(),
            data: Default::default(),
        }
    }
}

impl<T: Debug> SharedRc<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        SharedRc {
            handle: UpdateHandle::new(),
            data: Rc::new(RefCell::new(data)),
        }
    }
}

impl<T: Clone + Debug> SingleData for SharedRc<T> {
    type Item = T;

    fn get_cloned(&self) -> Self::Item {
        self.data.borrow().to_owned()
    }

    fn update(&self, value: Self::Item) -> Option<UpdateHandle> {
        *self.data.borrow_mut() = value;
        Some(self.handle)
    }

    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.handle)
    }
}
impl<T: Clone + Debug> SingleDataMut for SharedRc<T> {
    fn set(&mut self, value: Self::Item) {
        *self.data.borrow_mut() = value;
    }
}

impl<T: ListDataMut> ListData for SharedRc<T> {
    type Key = T::Key;
    type Item = T::Item;

    fn len(&self) -> usize {
        self.data.borrow().len()
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        self.data.borrow().get_cloned(key)
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
        self.data.borrow_mut().set(key, value);
        Some(self.handle)
    }

    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.data.borrow().iter_vec(limit)
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.data.borrow().iter_vec_from(start, limit)
    }

    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.handle)
    }
}
impl<T: ListDataMut> ListDataMut for SharedRc<T> {
    fn set(&mut self, key: &Self::Key, item: Self::Item) {
        self.data.borrow_mut().set(key, item);
    }
}
