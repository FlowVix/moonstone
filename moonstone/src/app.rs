use std::{cell::RefCell, collections::VecDeque, rc::Rc, sync::Arc};

use godot::{
    classes::Node,
    obj::{Gd, Inherits},
};
use parking_lot::Mutex;

use crate::{AnchorType, View, ViewState, ViewValue, ctx::AppCtx};

pub struct App<T: View> {
    state: T,
    view_state: ViewState<T>,
}
impl<T: View> App<T> {
    pub fn new<N: Inherits<Node>>(mount: Gd<N>, state: impl FnOnce() -> T) -> Self {
        let state = state();
        let view_state = state.build(mount.upcast(), AnchorType::ChildOf);
        Self { state, view_state }
    }
    pub fn destroy(mut self) {
        View::teardown(&mut self.view_state);
    }
}
