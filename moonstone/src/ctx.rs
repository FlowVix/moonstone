use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use godot::{classes::Node, obj::Gd};

use crate::{AnchorType, View, ViewValue};

#[doc(hidden)]
pub struct ViewRef<M> {
    #[doc(hidden)]
    pub __check: Box<dyn FnMut() -> bool>,
    #[doc(hidden)]
    pub __message: Box<dyn FnMut(&M)>,
}

pub struct AppCtx<M> {
    pub(crate) msg_queue: Rc<RefCell<VecDeque<M>>>,
    // pub(crate) view_refs: Rc<RefCell<Vec<ViewRef<M>>>>,
}

impl<M> Clone for AppCtx<M> {
    fn clone(&self) -> Self {
        Self {
            msg_queue: self.msg_queue.clone(),
            // view_refs: self.view_refs.clone(),
        }
    }
}
impl<M> AppCtx<M> {
    pub fn msg(&self, msg: M) {
        self.msg_queue.borrow_mut().push_back(msg);
    }
    // #[doc(hidden)]
    // pub fn __view_refs(&self) -> &Rc<RefCell<Vec<ViewRef<M>>>> {
    //     &self.view_refs
    // }
}
