use std::{
    collections::HashMap,
    hash::Hash,
    ops::{Deref, DerefMut},
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

pub trait View<M>: Sized {
    type State;

    fn build(&self, parent_anchor: Gd<Node>, parent_anchor_type: AnchorType) -> ViewState<M, Self>;
    fn rebuild(&self, state: &mut ViewState<M, Self>);
    fn teardown(state: &mut ViewState<M, Self>);
    fn collect_nodes(state: &ViewState<M, Self>, nodes: &mut Vec<Gd<Node>>);
    fn message(&mut self, msg: &M);

    // fn get(state: &Self::State) -> &Self;
    // fn get_mut(state: &mut Self::State) -> &mut Self;
}

pub trait CustomView<M>: Sized {
    fn message(&mut self, msg: &M);
    fn sync(&mut self);
}

pub struct ViewState<M, T: View<M>> {
    #[doc(hidden)]
    pub __state: T::State,
    #[doc(hidden)]
    pub __parent_anchor: Gd<Node>,
    #[doc(hidden)]
    pub __parent_anchor_type: AnchorType,
}
pub struct ViewValue<M, T: View<M>> {
    #[doc(hidden)]
    pub __value: T,
    #[doc(hidden)]
    pub __state: ViewState<M, T>,
}
impl<M, T: View<M>> Deref for ViewValue<M, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.__value
    }
}
impl<M, T: View<M>> DerefMut for ViewValue<M, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.__value
    }
}

impl<M, T: Inherits<Node>> View<M> for Gd<T> {
    type State = Gd<T>;

    fn build(
        &self,
        mut parent_anchor: Gd<Node>,
        parent_anchor_type: AnchorType,
    ) -> ViewState<M, Self> {
        parent_anchor_type.add(&mut parent_anchor, &self.clone().upcast());
        ViewState {
            __state: self.clone(),
            __parent_anchor: parent_anchor,
            __parent_anchor_type: parent_anchor_type,
        }
    }

    fn rebuild(&self, state: &mut ViewState<M, Self>) {
        View::teardown(state);
        state
            .__parent_anchor_type
            .add(&mut state.__parent_anchor, &self.clone().upcast());
        state.__state = self.clone();
    }

    fn teardown(state: &mut ViewState<M, Self>) {
        state
            .__parent_anchor_type
            .remove(&mut state.__parent_anchor, &state.__state.clone().upcast());
    }

    fn collect_nodes(state: &ViewState<M, Self>, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.__state.clone().upcast());
    }

    fn message(&mut self, msg: &M) {}
}

pub struct OptionViewState<InnerState> {
    anchor: Gd<Node>,
    inner_state: Option<InnerState>,
}
impl<M, T: View<M>> View<M> for Option<T> {
    type State = OptionViewState<ViewState<M, T>>;

    fn build(
        &self,
        mut parent_anchor: Gd<Node>,
        parent_anchor_type: AnchorType,
    ) -> ViewState<M, Self> {
        let opt_anchor = Node::new_alloc();
        parent_anchor_type.add(&mut parent_anchor, &opt_anchor);

        let inner_state = self
            .as_ref()
            .map(|v| v.build(opt_anchor.clone(), AnchorType::Before));

        ViewState {
            __state: OptionViewState {
                anchor: opt_anchor,
                inner_state,
            },
            __parent_anchor: parent_anchor,
            __parent_anchor_type: parent_anchor_type,
        }
    }

    fn rebuild(&self, state: &mut ViewState<M, Self>) {
        let opt_anchor = state.__state.anchor.clone();
        match (self, state.__state.inner_state.as_mut()) {
            (None, None) => {}
            (None, Some(inner_state)) => {
                View::teardown(inner_state);
                state.__state.inner_state = None;
            }
            (Some(new), None) => {
                state.__state.inner_state = Some(new.build(opt_anchor, AnchorType::Before));
            }
            (Some(new), Some(inner_state)) => {
                new.rebuild(inner_state);
            }
        }
    }

    fn teardown(state: &mut ViewState<M, Self>) {
        state
            .__parent_anchor_type
            .remove(&mut state.__parent_anchor, &state.__state.anchor);
        if let Some(is) = &mut state.__state.inner_state {
            View::teardown(is);
        }
    }

    fn collect_nodes(state: &ViewState<M, Self>, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.__state.anchor.clone());
        if let Some(is) = &state.__state.inner_state {
            View::collect_nodes(is, nodes);
        }
    }

    fn message(&mut self, msg: &M) {
        if let Some(s) = self {
            s.message(msg);
        }
    }
}

pub struct VecViewState<K, InnerState> {
    anchor: Gd<Node>,
    inner_state: Vec<(K, InnerState)>,
}
impl<M, K: Hash + Eq + Clone, T: View<M>> View<M> for Vec<(K, T)> {
    type State = VecViewState<K, ViewState<M, T>>;

    fn build(
        &self,
        mut parent_anchor: Gd<Node>,
        parent_anchor_type: AnchorType,
    ) -> ViewState<M, Self> {
        let vec_anchor = Node::new_alloc();
        parent_anchor_type.add(&mut parent_anchor, &vec_anchor);

        let inner_state = self
            .iter()
            .map(|(k, v)| (k.clone(), v.build(vec_anchor.clone(), AnchorType::Before)))
            .collect::<Vec<_>>();

        ViewState {
            __state: VecViewState {
                anchor: vec_anchor,
                inner_state,
            },
            __parent_anchor: parent_anchor,
            __parent_anchor_type: parent_anchor_type,
        }
    }

    fn rebuild(&self, state: &mut ViewState<M, Self>) {
        let vec_anchor = state.__state.anchor.clone();

        let mut total_nodes = 0;

        let mut prev_map = state
            .__state
            .inner_state
            .drain(..)
            .enumerate()
            .map(|(idx, mut is)| {
                let mut nodes = vec![];
                View::collect_nodes(&mut is.1, &mut nodes);
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
                state.__state.inner_state.push((k, is));
            } else {
                let is = v.build(vec_anchor.clone(), AnchorType::Before);
                let mut nodes = vec![];
                View::collect_nodes(&is, &mut nodes);
                for node in &nodes {
                    node.get_parent().unwrap().move_child(node, move_idx as i32);
                    move_idx += 1;
                }
                state.__state.inner_state.push((k.clone(), is));
            }
        }

        for (_, (mut inner, _)) in prev_map.drain() {
            View::teardown(&mut inner);
        }
    }

    fn teardown(state: &mut ViewState<M, Self>) {
        state
            .__parent_anchor_type
            .remove(&mut state.__parent_anchor, &state.__state.anchor);
        for (_, is) in &mut state.__state.inner_state {
            View::teardown(is);
        }
    }

    fn collect_nodes(state: &ViewState<M, Self>, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.__state.anchor.clone());
        for (_, is) in &state.__state.inner_state {
            View::collect_nodes(is, nodes);
        }
    }

    fn message(&mut self, msg: &M) {
        for i in self {
            i.1.message(msg);
        }
    }
}
