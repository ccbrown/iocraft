mod components;
mod render;
mod style;
mod traits;

extern crate self as flashy_io;

pub use flashy_element::ElementType;

#[derive(Clone, Hash, PartialEq, Eq, Debug, derive_more::Display)]
pub struct ElementKey(uuid::Uuid);

impl ElementKey {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

pub type Element<T> = flashy_element::Element<ElementKey, T>;

pub use flashy_macros::flashy;

pub mod prelude;
pub use prelude::*;
