use std::mem::{self, swap};

use godot::{
    classes::{Button, LineEdit, Node, PanelContainer, VBoxContainer},
    obj::{Gd, WithBaseField},
};
use moonstone::{ViewValue, mutate, viewtype};

viewtype! {
    enum Guy {
        Foo(Gd<Button>),
        Bar(Gd<LineEdit>),
    }
}

viewtype! {
    struct Bar: VBoxContainer {
        pub view a: Gd<Button>,
        view lol: Button {
            pub view b: Gd<Button>,
        }
    }
}

// impl CustomView for Bar {
//     fn init(&mut self) {
//         mutate!(self { a, b }, {
//             swap(a, b);
//         })
//         // self.peeenis()
//         // self.p
//         // mutate(self).a().b().
//     }
// }
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node, no_init)]
struct Gang {
    base: Base<Node>,
}
fn lol() {
    Bar_Init {
        a: todo!(),
        lol: todo!(),
        b: todo!(),
    }
    .build(|b| {});
    // Gd::fro
    // Bar_Init {
    //     a: Button::new_alloc(),
    //     b: Button::new_alloc(),
    //     lol: |this|
    // }
    // Bar::build(|this| Bar_Init {
    //     a: Button::new_alloc(),
    //     b: Button::new_alloc(),
    // });
    // Gd::fro
}
// fn lol(v: Gang) {
// }

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
