use godot::classes::{Button, PanelContainer, VBoxContainer};
use moonstone::{CustomView, ViewValue, view};

#[view(base = PanelContainer, msg = ())]
struct Bar {
    pub a: i32,
    #[view]
    #[enter(lol: VBoxContainer)]
    pub b: godot::obj::Gd<Button>,
}
impl CustomView for Bar {
    fn init(&mut self) {}
    fn sync(&mut self) {
        todo!()
    }
}

fn main() {}
