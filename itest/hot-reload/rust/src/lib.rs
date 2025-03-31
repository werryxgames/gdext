/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

struct HotReload;

#[gdextension]
unsafe impl ExtensionLibrary for HotReload {
    fn on_level_init(_level: InitLevel) {
        println!("[Rust]      Init level {:?}", _level);
    }

    fn on_level_deinit(_level: InitLevel) {
        println!("[Rust]      Deinit level {:?}", _level);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init, base=Node)]
struct Reloadable {
    #[export]
    #[init(val = Planet::Earth)]
    favorite_planet: Planet,
}

#[godot_api]
impl Reloadable {
    #[func]
    #[rustfmt::skip]
    // DO NOT MODIFY FOLLOWING LINE -- replaced by hot-reload test. Hence #[rustfmt::skip] above.
    fn get_number(&self) -> i64 { 100 }

    #[func]
    fn from_string(s: GString) -> Gd<Self> {
        Gd::from_object(Reloadable {
            favorite_planet: Planet::from_godot(s),
        })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotConvert, Var, Export)]
#[godot(via = GString)]
enum Planet {
    Earth,
    Mars,
    Venus,
}
