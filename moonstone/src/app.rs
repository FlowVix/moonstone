use std::{collections::VecDeque, sync::Arc};

use godot::{
    classes::Node,
    obj::{Gd, Inherits},
};
use parking_lot::Mutex;

use crate::{AnchorType, View, ViewState, ViewValue, message::Messenger};

pub struct App<M, T: View<M>> {
    state: T,
    view_state: ViewState<M, T>,
    messenger: Messenger<M>,
}
impl<M, T: View<M>> App<M, T> {
    pub fn new<N: Inherits<Node>>(mount: Gd<N>, state: impl FnOnce(Messenger<M>) -> T) -> Self {
        let messenger = Messenger {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        };
        let state = state(messenger.clone());
        let view_state = state.build(mount.upcast(), AnchorType::ChildOf);
        Self {
            state,
            view_state,
            messenger,
        }
    }
    pub fn handle_messages(&mut self) {
        let msgs = { self.messenger.queue.lock().drain(..).collect::<Vec<_>>() };
        for msg in &msgs {
            self.state.message(msg);
        }
        if !msgs.is_empty() {
            self.state.rebuild(&mut self.view_state);
        }
    }
    pub fn destroy(mut self) {
        View::teardown(&mut self.view_state);
    }
}
