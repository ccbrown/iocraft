// # Organization
//
// Code is organized into modules primarily for the purpose of organizing code. Types will be
// re-exported in the root so that users of the library have a flat namespace to work with.
//
// The exception is the models that represent collections of types, namely hooks and components.
// Those types will remain in their modules for the publix API.

mod canvas;
mod component;
mod context;
mod element;
mod hook;
mod props;
mod render;
mod signal;
mod style;
mod terminal;

mod flattened_exports {
    pub use crate::canvas::*;
    pub use crate::component::*;
    pub use crate::context::*;
    pub use crate::element::*;
    pub use crate::hook::*;
    pub use crate::props::*;
    pub use crate::render::*;
    pub use crate::signal::*;
    pub use crate::style::*;
    pub use crate::terminal::*;

    pub use iocraft_macros::*;
}

pub use flattened_exports::*;
pub mod components;
pub mod hooks;

pub mod prelude {
    pub use crate::components::*;
    pub use crate::flattened_exports::*;
    pub use crate::hooks::*;
}

// So we can use our own macros.
extern crate self as iocraft;
