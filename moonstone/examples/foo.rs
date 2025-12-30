use std::mem::{self, swap};

use godot::{
    classes::{Button, LineEdit, Node, PanelContainer, VBoxContainer},
    obj::{Gd, WithBaseField},
};
use moonstone::{CustomView, ViewValue, mutate, viewtype};

viewtype! {
    enum Guy {
        Foo(Gd<Button>),
        Bar(Gd<LineEdit>),
    }
}

viewtype! {
    struct Bar {
        pub view a: Gd<Button>,
        pub view b: Gd<Button>,
    } VBoxContainer
}

impl CustomView for Bar {
    fn init(&mut self) {
        mutate!(self { a, b }, {
            swap(a, b);
        })
        // self.peeenis()
        // self.p
        // mutate(self).a().b().
    }
}

// struct Foo {
//     v: String,
// }
// macro_rules! gog {
//     ({
//         $($t:tt)*
//     }) => {
//         {
//             $($t)*
//         }
//     };
// }
// impl Foo {
//     fn lol(&mut self, f: impl FnOnce(&mut String, &mut Self)) {
//         // f(&mut self.v, self);
//         let lol = gog!({
//             let a = 3;
//         });
//     }
// }

fn main() {}
