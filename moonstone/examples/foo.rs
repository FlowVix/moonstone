use godot::classes::{Button, PanelContainer};
use moonstone::view;

#[view(base = PanelContainer, msg = (), cb = |_, _| {})]
struct Bar {
    pub a: i32,
    #[view]
    b: godot::obj::Gd<Button>,
}

fn main() {}
