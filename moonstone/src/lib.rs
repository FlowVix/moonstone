mod view;

pub use moonstone_macro::viewtype;
pub use view::{Anchor, BeforeAnchor, ChildAnchor, View, ViewValue};

#[doc(hidden)]
pub use paste as __paste;
