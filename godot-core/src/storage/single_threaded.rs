/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::any::type_name;
use std::backtrace::Backtrace;
use std::cell;
use std::sync::Mutex;

#[cfg(not(feature = "experimental-threads"))]
use godot_cell::panicking::{GdCell, InaccessibleGuard, MutGuard, RefGuard};

#[cfg(feature = "experimental-threads")]
use godot_cell::blocking::{GdCell, InaccessibleGuard, MutGuard, RefGuard};

use crate::obj::{Base, GodotClass};
use crate::out;
use crate::storage::{Lifecycle, Storage, StorageRefCounted};

pub struct InstanceStorage<T: GodotClass> {
    user_instance: GdCell<T>,
    pub(super) base: Base<T::Base>,

    // Declared after `user_instance`, is dropped last
    pub(super) lifecycle: cell::Cell<Lifecycle>,
    godot_ref_count: cell::Cell<u32>,

    mut_binder: Mutex<Option<Backtrace>>,
    const_binders: Mutex<Vec<Backtrace>>,
}

// SAFETY:
// The only way to get a reference to the user instance is by going through the `GdCell` in `user_instance`.
// If this `GdCell` has returned any references, then `self.user_instance.as_ref().is_currently_bound()` will
// return true. So `is_bound` will return true when a reference to the user instance exists.
//
// If `is_bound` is false, then there are no references to the user instance in this storage. And if a `&mut`
// reference to the storage exists then no other references to data in this storage can exist. So we can
// safely drop it.
unsafe impl<T: GodotClass> Storage for InstanceStorage<T> {
    type Instance = T;

    fn construct(
        user_instance: Self::Instance,
        base: Base<<Self::Instance as GodotClass>::Base>,
    ) -> Self {
        out!("    Storage::construct             <{}>", type_name::<T>());
        Self {
            user_instance: GdCell::new(user_instance),
            base,
            lifecycle: cell::Cell::new(Lifecycle::Alive),
            godot_ref_count: cell::Cell::new(1),
            mut_binder: Mutex::new(None),
            const_binders: Mutex::new(vec![]),
        }
    }

    fn is_bound(&self) -> bool {
        self.user_instance.is_currently_bound()
    }

    fn base(&self) -> &Base<<Self::Instance as GodotClass>::Base> {
        &self.base
    }

    fn get(&self) -> RefGuard<'_, T> {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let value = self.user_instance.borrow().unwrap_or_else(|err| {
            panic!(
                "\
                    Gd<T>::bind() failed, already bound; T = {}.\n  \
                    Make sure to use `self.base_mut()` or `self.base()` instead of `self.to_gd()` when possible.\n  \
                    Details: single-threaded, {err}.\n  \
                    Backtrace: {}\n  \
                    Mutable binder: {:?}\n  \
                ",
                type_name::<T>(),
                backtrace,
                self.mut_binder.lock().unwrap(),
            )
        });
        self.const_binders.lock().unwrap().push(backtrace);
        value
    }

    fn get_mut(&self) -> MutGuard<'_, T> {
        let backtrace = std::backtrace::Backtrace::force_capture();
        self.const_binders.lock().unwrap().retain(|_| {
            self.is_bound()
        });
        let value = self.user_instance.borrow_mut().unwrap_or_else(|err| {
            panic!(
                "\
                    Gd<T>::bind_mut() failed, already bound; T = {}.\n  \
                    Make sure to use `self.base_mut()` instead of `self.to_gd()` when possible.\n  \
                    Details: single-threaded, {err}.\n  \
                    Backtrace: {}\n  \
                    Constant binders: {:?}\n  \
                    Mutable binder: {:?}\n  \
                ",
                type_name::<T>(),
                backtrace,
                self.const_binders.lock().unwrap(),
                self.mut_binder.lock().unwrap(),
            )
        });
        self.mut_binder.lock().unwrap().replace(backtrace);
        value
    }

    fn get_inaccessible<'a: 'b, 'b>(
        &'a self,
        value: &'b mut Self::Instance,
    ) -> InaccessibleGuard<'b, T> {
        self.user_instance
            .make_inaccessible(value)
            .unwrap_or_else(|err| {
                // We should never hit this, except maybe in extreme cases like having more than
                // `usize::MAX` borrows.
                panic!(
                    "\
                        `base_mut()` failed for type T = {}.\n  \
                        This is most likely a bug, please report it.\n  \
                        Details: {err}.\
                    ",
                    type_name::<T>()
                )
            })
    }

    fn get_lifecycle(&self) -> Lifecycle {
        self.lifecycle.get()
    }

    fn set_lifecycle(&self, lifecycle: Lifecycle) {
        self.lifecycle.set(lifecycle)
    }
}

impl<T: GodotClass> StorageRefCounted for InstanceStorage<T> {
    fn godot_ref_count(&self) -> u32 {
        self.godot_ref_count.get()
    }

    fn on_inc_ref(&self) {
        let refc = self.godot_ref_count.get() + 1;
        self.godot_ref_count.set(refc);

        out!(
            "    Storage::on_inc_ref (rc={})     <{}>", // -- {:?}",
            refc,
            type_name::<T>(),
            //self.user_instance
        );
    }

    fn on_dec_ref(&self) {
        let refc = self.godot_ref_count.get() - 1;
        self.godot_ref_count.set(refc);

        out!(
            "  | Storage::on_dec_ref (rc={})     <{}>", // -- {:?}",
            refc,
            type_name::<T>(),
            //self.user_instance
        );
    }
}

impl<T: GodotClass> Drop for InstanceStorage<T> {
    fn drop(&mut self) {
        out!(
            "    Storage::drop (rc={})           <{:?}>",
            self.godot_ref_count(),
            self.base(),
        );
    }
}
