use godot::{
    classes::{Button, PanelContainer, VBoxContainer},
    obj::Gd,
};
use moonstone::{CustomView, ViewValue, viewtype};

// #[view(base = PanelContainer, msg = ())]
// struct Bar {
//     pub a: i32,
//     #[view]
//     #[enter(lol: VBoxContainer)]
//     pub b: godot::obj::Gd<Button>,
// }

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
