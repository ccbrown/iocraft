//! # iocraft
//!
//! `iocraft` is a library for crafting beautiful text output and interfaces for the terminal or logs.

#![warn(missing_docs)]

// # Organization
//
// Code is organized into modules primarily for the benefit of the maintainers. Types will be
// re-exported in the root so that users of the library have a flat namespace to work with.
//
// The exception is the models that represent collections of types, namely hooks and components.
// Those types will remain in their modules for the public API.

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

/// Components for crafting your UI.
pub mod components;

/// Hooks for adding behavior to your components.
pub mod hooks;

/// By importing this module, you'll bring all of the crate's commonly used types into scope.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::flattened_exports::*;
    pub use crate::hooks::*;
}

// So we can use our own macros.
extern crate self as iocraft;
