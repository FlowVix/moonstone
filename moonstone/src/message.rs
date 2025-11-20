use std::{collections::VecDeque, sync::Arc};

use parking_lot::Mutex;

pub struct Messenger<T> {
    pub(crate) queue: Arc<Mutex<VecDeque<T>>>,
}
impl<T> Clone for Messenger<T> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
        }
    }
}
impl<T> Messenger<T> {
    pub fn send(&self, msg: T) {
        self.queue.lock().push_back(msg);
    }
}
