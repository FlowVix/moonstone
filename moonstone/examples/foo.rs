use godot::{
    classes::{Button, PanelContainer, VBoxContainer},
    obj::{Gd, WithBaseField},
};
use moonstone::{CustomView, ViewValue, viewtype};

viewtype! {
    struct Bar: PanelContainer {
        a: i32,
        pub(crate) b: VBoxContainer {
            pub view c: Option<Gd<Button>>,
        }
    }
}

impl CustomView for Bar {
    fn init(&mut self) {}
    fn sync(&mut self) {}
}

fn main() {}
