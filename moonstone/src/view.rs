use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
};

use godot::{classes::Button, prelude::*};

pub struct ChildAnchor {
    node: Gd<Node>,
}
pub struct BeforeAnchor {
    node: Gd<Node>,
    anchored: HashSet<Gd<Node>>,
}
pub trait Anchor {
    fn new(node: Gd<Node>) -> Self
    where
        Self: Sized;
    fn add(&mut self, node: &Gd<Node>);
    fn remove(&mut self, node: &Gd<Node>);
    fn node(&self) -> Gd<Node>;
}

impl Anchor for ChildAnchor {
    fn add(&mut self, node: &Gd<Node>) {
        self.node.add_child(node);
    }

    fn remove(&mut self, node: &Gd<Node>) {
        self.node.remove_child(node);
    }

    fn new(node: Gd<Node>) -> Self
    where
        Self: Sized,
    {
        Self { node }
    }

    fn node(&self) -> Gd<Node> {
        self.node.clone()
    }
}
impl Anchor for BeforeAnchor {
    fn add(&mut self, node: &Gd<Node>) {
        let idx = self.node.get_index();
        let mut parent = self.node.get_parent().unwrap();
        parent.add_child(node);
        parent.move_child(node, idx);
    }

    fn remove(&mut self, node: &Gd<Node>) {
        if self.anchored.remove(node) {
            self.node.get_parent().unwrap().remove_child(node);
        }
    }

    fn new(node: Gd<Node>) -> Self
    where
        Self: Sized,
    {
        Self {
            node,
            anchored: HashSet::new(),
        }
    }

    fn node(&self) -> Gd<Node> {
        self.node.clone()
    }
}

pub trait View: Sized {
    type State;
    type Access<'a>
    where
        Self: 'a;

    fn build(&self, parent_anchor: &mut dyn Anchor) -> Self::State;
    fn rebuild(&self, state: &mut Self::State);
    fn teardown(state: &mut Self::State, parent_anchor: &mut dyn Anchor);
    fn collect_nodes(state: &Self::State, nodes: &mut Vec<Gd<Node>>);
    fn access<'a>(&'a self) -> Self::Access<'a>;
}

pub trait CustomView: Sized {
    fn init(&mut self);
}

pub struct ViewValue<T: View> {
    pub(crate) value: T,
    pub(crate) state: T::State,
}
impl<T: View> ViewValue<T> {
    #[doc(hidden)]
    pub fn __create(value: T, state: T::State) -> Self {
        Self { value, state }
    }
    #[doc(hidden)]
    pub fn __value(&self) -> &T {
        &self.value
    }
    #[doc(hidden)]
    pub fn __value_mut(&mut self) -> &mut T {
        &mut self.value
    }
    #[doc(hidden)]
    pub fn __rebuild(&mut self) {
        self.value.rebuild(&mut self.state);
    }
}

pub struct GdViewState<T: Inherits<Node>> {
    anchor: BeforeAnchor,
    node: Gd<T>,
}

impl<T: Inherits<Node>> View for Gd<T> {
    type State = GdViewState<T>;
    type Access<'a> = Self;

    fn build(&self, parent_anchor: &mut dyn Anchor) -> Self::State {
        let mut gd_anchor = BeforeAnchor::new(Node::new_alloc());
        parent_anchor.add(&gd_anchor.node());

        gd_anchor.add(&self.clone().upcast());

        GdViewState {
            anchor: gd_anchor,
            node: self.clone(),
        }
    }

    fn rebuild(&self, state: &mut Self::State) {
        if self != &state.node {
            state.anchor.remove(&state.node.clone().upcast());
            if let Some(mut parent) = self.upcast_ref().get_parent() {
                parent.remove_child(self);
            }
            state.anchor.add(&self.clone().upcast());
            state.node = self.clone();
        }
    }

    fn teardown(state: &mut Self::State, parent_anchor: &mut dyn Anchor) {
        state.anchor.remove(&state.node.clone().upcast());
        parent_anchor.remove(&state.anchor.node());
        state.anchor.node().queue_free();
        state.node.upcast_mut().queue_free();
        // state.state.upcast_mut().queue_free();
    }

    fn collect_nodes(state: &Self::State, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.node.clone().upcast());
        nodes.push(state.anchor.node());
    }

    fn access<'a>(&'a self) -> Self::Access<'a> {
        self.clone()
    }
}

pub struct OptionViewState<InnerState> {
    anchor: BeforeAnchor,
    inner_state: Option<InnerState>,
}
impl<T: View> View for Option<T> {
    type State = OptionViewState<T::State>;
    type Access<'a>
        = &'a Self
    where
        T: 'a;

    fn build(&self, parent_anchor: &mut dyn Anchor) -> Self::State {
        let mut opt_anchor = BeforeAnchor::new(Node::new_alloc());
        parent_anchor.add(&opt_anchor.node());

        let inner_state = self.as_ref().map(|v| v.build(&mut opt_anchor));

        OptionViewState {
            anchor: opt_anchor,
            inner_state,
        }
    }

    fn rebuild(&self, state: &mut Self::State) {
        match (self, state.inner_state.as_mut()) {
            (None, None) => {}
            (None, Some(inner_state)) => {
                <T as View>::teardown(inner_state, &mut state.anchor);
                state.inner_state = None;
            }
            (Some(new), None) => {
                state.inner_state = Some(new.build(&mut state.anchor));
            }
            (Some(new), Some(inner_state)) => {
                new.rebuild(inner_state);
            }
        }
    }

    fn teardown(state: &mut Self::State, parent_anchor: &mut dyn Anchor) {
        if let Some(is) = &mut state.inner_state {
            <T as View>::teardown(is, &mut state.anchor);
        }
        parent_anchor.remove(&state.anchor.node());
        state.anchor.node().clone().queue_free();
    }

    fn collect_nodes(state: &Self::State, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.anchor.node());
        if let Some(is) = &state.inner_state {
            <T as View>::collect_nodes(is, nodes);
        }
    }

    fn access<'a>(&'a self) -> Self::Access<'a> {
        self
    }
}

pub struct VecViewState<K, InnerState> {
    anchor: BeforeAnchor,
    inner_state: Vec<(K, InnerState)>,
}
impl<K: Hash + Eq + Clone, T: View> View for Vec<(K, T)> {
    type State = VecViewState<K, T::State>;
    type Access<'a>
        = &'a Self
    where
        T: 'a,
        K: 'a;

    fn build(&self, parent_anchor: &mut dyn Anchor) -> Self::State {
        let mut vec_anchor = BeforeAnchor::new(Node::new_alloc());
        parent_anchor.add(&vec_anchor.node());

        let inner_state = self
            .iter()
            .map(|(k, v)| (k.clone(), v.build(&mut vec_anchor)))
            .collect::<Vec<_>>();

        VecViewState {
            anchor: vec_anchor,
            inner_state,
        }
    }

    fn rebuild(&self, state: &mut Self::State) {
        let mut total_nodes = 0;

        let mut prev_map = state
            .inner_state
            .drain(..)
            .enumerate()
            .map(|(idx, is)| {
                let mut nodes = vec![];
                <T as View>::collect_nodes(&is.1, &mut nodes);
                total_nodes += nodes.len();
                (is.0, (is.1, nodes))
            })
            .collect::<HashMap<_, _>>();

        let mut move_idx = state.anchor.node().get_index() as usize - total_nodes;
        for (k, v) in self {
            if let Some((k, (mut is, nodes))) = prev_map.remove_entry(k) {
                for node in &nodes {
                    node.get_parent().unwrap().move_child(node, move_idx as i32);
                    move_idx += 1;
                }
                v.rebuild(&mut is);
                let mut new_nodes = vec![];
                <T as View>::collect_nodes(&is, &mut new_nodes);
                move_idx =
                    (move_idx as isize + new_nodes.len() as isize - nodes.len() as isize) as usize;
                state.inner_state.push((k, is));
            } else {
                let is = v.build(&mut state.anchor);
                let mut nodes = vec![];
                <T as View>::collect_nodes(&is, &mut nodes);
                for node in &nodes {
                    node.get_parent().unwrap().move_child(node, move_idx as i32);
                    move_idx += 1;
                }
                state.inner_state.push((k.clone(), is));
            }
        }

        for (_, (mut inner, _)) in prev_map.drain() {
            <T as View>::teardown(&mut inner, &mut state.anchor);
        }
    }

    fn teardown(state: &mut Self::State, parent_anchor: &mut dyn Anchor) {
        for (_, is) in &mut state.inner_state {
            <T as View>::teardown(is, &mut state.anchor);
        }
        parent_anchor.remove(&state.anchor.node());
        state.anchor.node().queue_free();
    }

    fn collect_nodes(state: &Self::State, nodes: &mut Vec<Gd<Node>>) {
        nodes.push(state.anchor.node());
        for (_, is) in &state.inner_state {
            <T as View>::collect_nodes(is, nodes);
        }
    }

    fn access<'a>(&'a self) -> Self::Access<'a> {
        self
    }
}

#[macro_export]
macro_rules! mutate {
    ($obj:ident{$($field:ident),* $(,)?}, {
        $($body:tt)*
    }) => {
        {
            $crate::__paste::paste! {
                $(
                    let $field = $obj.[< __DONT_USE_THIS_DIRECTLY_ $field >].__value_mut();
                )*
            }
            let out = {
                $($body)*
            };
            $crate::__paste::paste! {
                $(
                    $obj.[< __DONT_USE_THIS_DIRECTLY_ $field >].__rebuild();
                )*
            }
            out
        }
    };
}
