use std::{
    cell::RefCell,
    collections::HashMap,
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
};

use godot::{classes::Button, prelude::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorType {
    ChildOf,
    Before,
}
impl AnchorType {
    pub fn add(self, anchor: &mut Node, node: &Gd<Node>) {
        match self {
            AnchorType::ChildOf => anchor.add_child(node),
            AnchorType::Before => {
                let idx = anchor.get_index();
                let mut parent = anchor.get_parent().unwrap();
                parent.add_child(node);
                parent.move_child(node, idx);
            }
        }
    }
    pub fn remove(self, anchor: &mut Node, node: &Gd<Node>) {
        match self {
            AnchorType::ChildOf => anchor.remove_child(node),
            AnchorType::Before => anchor.get_parent().unwrap().remove_child(node),
        }
    }
}

pub trait View: Sized {
    type State;

    fn build(&self, parent_anchor: Gd<Node>, parent_anchor_type: AnchorType) -> ViewState<Self>;
    fn rebuild(&self, state: &mut ViewState<Self>);
    fn teardown(state: &mut ViewState<Self>);
    fn collect_nodes(state: &ViewState<Self>, nodes: &mut Vec<Gd<Node>>);

    // fn get(state: &Self::State) -> &Self;
    // fn get_mut(state: &mut Self::State) -> &mut Self;
}

pub trait CustomView: Sized {
    // fn message(&mut self, msg: &M);
    fn init(&mut self);
    fn sync(&mut self);
}

pub struct ViewState<T: View> {
    pub state: T::State,
    pub parent_anchor: Gd<Node>,
    pub parent_anchor_type: AnchorType,
}
pub struct ViewValue<T: View> {
    pub(crate) value: T,
    pub(crate) state: ViewState<T>,
}
pub struct ViewValueMut<'a, T: View> {
    pub(crate) inner: &'a mut ViewValue<T>,
}
impl<T: View> ViewValue<T> {
    pub fn create(value: T, state: ViewState<T>) -> Self {
        Self { value, state }
    }
    pub fn get_mut(&mut self) -> ViewValueMut<'_, T> {
        ViewValueMut { inner: self }
    }
}
impl<T: View> Deref for ViewValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<'a, T: View> Drop for ViewValueMut<'a, T> {
    fn drop(&mut self) {
        self.inner.value.rebuild(&mut self.inner.state);
    }
}
impl<'a, T: View> Deref for ViewValueMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}
impl<'a, T: View> DerefMut for ViewValueMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.value
    }
}

impl<T: Inherits<Node>> View for Gd<T> {
    type State = Gd<T>;

    fn build(
        &self,
        mut parent_anchor: Gd<Node>,
        parent_anchor_type: AnchorType,
    ) -> ViewState<Self> {
        parent_anchor_type.add(&mut parent_anchor, &self.clone().upcast());
        ViewState {
            state: self.clone(),
            parent_anchor,
            parent_anchor_type,
        }
    }

    fn rebuild(&self, state: &mut ViewState<Self>) {
        if self.upcast_ref().get_parent() != state.state.upcast_ref().get_parent() {
            state.state.upcast_mut().queue_free();
            state.state.clone().upcast_mut().replace_by(self);
            state.state = self.clone();
        }
    }

    fn teardown(state: &mut ViewState<Self>) {
        state
            .parent_anchor_type
            .remove(&mut state.parent_anchor, &state.state.clone().upcast());
        state.state.upcast_mut().queue_free();
    }

    fn collect_nodes(state: &ViewState<Self>, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.state.clone().upcast());
    }
}

pub struct OptionViewState<InnerState> {
    anchor: Gd<Node>,
    inner_state: Option<InnerState>,
}
impl<T: View> View for Option<T> {
    type State = OptionViewState<ViewState<T>>;

    fn build(
        &self,
        mut parent_anchor: Gd<Node>,
        parent_anchor_type: AnchorType,
    ) -> ViewState<Self> {
        let opt_anchor = Node::new_alloc();
        parent_anchor_type.add(&mut parent_anchor, &opt_anchor);

        let inner_state = self
            .as_ref()
            .map(|v| v.build(opt_anchor.clone(), AnchorType::Before));

        ViewState {
            state: OptionViewState {
                anchor: opt_anchor,
                inner_state,
            },
            parent_anchor,
            parent_anchor_type,
        }
    }

    fn rebuild(&self, state: &mut ViewState<Self>) {
        let opt_anchor = state.state.anchor.clone();
        match (self, state.state.inner_state.as_mut()) {
            (None, None) => {}
            (None, Some(inner_state)) => {
                View::teardown(inner_state);
                state.state.inner_state = None;
            }
            (Some(new), None) => {
                state.state.inner_state = Some(new.build(opt_anchor, AnchorType::Before));
            }
            (Some(new), Some(inner_state)) => {
                new.rebuild(inner_state);
            }
        }
    }

    fn teardown(state: &mut ViewState<Self>) {
        if let Some(is) = &mut state.state.inner_state {
            View::teardown(is);
        }
        state
            .parent_anchor_type
            .remove(&mut state.parent_anchor, &state.state.anchor);
        state.state.anchor.queue_free();
    }

    fn collect_nodes(state: &ViewState<Self>, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.state.anchor.clone());
        if let Some(is) = &state.state.inner_state {
            View::collect_nodes(is, nodes);
        }
    }
}

pub struct VecViewState<K, InnerState> {
    anchor: Gd<Node>,
    inner_state: Vec<(K, InnerState)>,
}
impl<K: Hash + Eq + Clone, T: View> View for Vec<(K, T)> {
    type State = VecViewState<K, ViewState<T>>;

    fn build(
        &self,
        mut parent_anchor: Gd<Node>,
        parent_anchor_type: AnchorType,
    ) -> ViewState<Self> {
        let vec_anchor = Node::new_alloc();
        parent_anchor_type.add(&mut parent_anchor, &vec_anchor);

        let inner_state = self
            .iter()
            .map(|(k, v)| (k.clone(), v.build(vec_anchor.clone(), AnchorType::Before)))
            .collect::<Vec<_>>();

        ViewState {
            state: VecViewState {
                anchor: vec_anchor,
                inner_state,
            },
            parent_anchor,
            parent_anchor_type,
        }
    }

    fn rebuild(&self, state: &mut ViewState<Self>) {
        let vec_anchor = state.state.anchor.clone();

        let mut total_nodes = 0;

        let mut prev_map = state
            .state
            .inner_state
            .drain(..)
            .enumerate()
            .map(|(idx, mut is)| {
                let mut nodes = vec![];
                View::collect_nodes(&is.1, &mut nodes);
                total_nodes += nodes.len();
                (is.0, (is.1, nodes))
            })
            .collect::<HashMap<_, _>>();

        let mut move_idx = vec_anchor.get_index() as usize - total_nodes;
        for (k, v) in self {
            if let Some((k, (mut is, nodes))) = prev_map.remove_entry(k) {
                for node in &nodes {
                    node.get_parent().unwrap().move_child(node, move_idx as i32);
                    move_idx += 1;
                }
                v.rebuild(&mut is);
                let mut new_nodes = vec![];
                View::collect_nodes(&is, &mut new_nodes);
                move_idx =
                    (move_idx as isize + new_nodes.len() as isize - nodes.len() as isize) as usize;
                state.state.inner_state.push((k, is));
            } else {
                let is = v.build(vec_anchor.clone(), AnchorType::Before);
                let mut nodes = vec![];
                View::collect_nodes(&is, &mut nodes);
                for node in &nodes {
                    node.get_parent().unwrap().move_child(node, move_idx as i32);
                    move_idx += 1;
                }
                state.state.inner_state.push((k.clone(), is));
            }
        }

        for (_, (mut inner, _)) in prev_map.drain() {
            View::teardown(&mut inner);
        }
    }

    fn teardown(state: &mut ViewState<Self>) {
        for (_, is) in &mut state.state.inner_state {
            View::teardown(is);
        }
        state
            .parent_anchor_type
            .remove(&mut state.parent_anchor, &state.state.anchor);
        state.state.anchor.queue_free();
    }

    fn collect_nodes(state: &ViewState<Self>, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.state.anchor.clone());
        for (_, is) in &state.state.inner_state {
            View::collect_nodes(is, nodes);
        }
    }
}
