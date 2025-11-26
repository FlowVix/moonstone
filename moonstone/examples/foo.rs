use godot::{
    classes::{Button, LineEdit, PanelContainer, VBoxContainer},
    obj::{Gd, WithBaseField},
};
use moonstone::{CustomView, ViewValue, viewtype};

viewtype! {
    enum Guy {
        Foo(Gd<Button>),
        Bar(Gd<LineEdit>),
    }
}

viewtype! {
    struct Bar: VBoxContainer {
        view switch: Gd<Button>,
        view guy: Guy,
    }
}

impl CustomView for Bar {
    fn init(&mut self) {}
    fn sync(&mut self) {}
}

fn main() {}
