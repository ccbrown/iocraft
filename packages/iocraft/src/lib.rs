//! # iocraft
//!
//! `iocraft` is a library for crafting beautiful text output and interfaces for the terminal or
//! logs. It allows you to easily build complex layouts and interactive elements using a
//! declarative API.
//!
//! ## Features
//!
//! - Define your UI using a clean, highly readable syntax.
//! - Organize your UI using flexbox layouts powered by [`taffy`](https://docs.rs/taffy/).
//! - Output colored and styled UIs to the terminal or ASCII output anywhere else.
//! - Create animated or interactive elements with event handling and hooks.
//! - Build fullscreen terminal applications with ease.
//! - Pass [props](crate::Props) and [context](crate::components::ContextProvider) by reference to avoid unnecessary cloning.
//!
//! ## Getting Started
//!
//! If you're familiar with React, you'll feel right at home with `iocraft`. It uses all the same
//! concepts, but is text-focused and made for Rust.
//!
//! Here's your first `iocraft` program:
//!
//! ```
//! use iocraft::prelude::*;
//!
//! fn main() {
//!     element! {
//!         View(
//!             border_style: BorderStyle::Round,
//!             border_color: Color::Blue,
//!         ) {
//!             Text(content: "Hello, world!")
//!         }
//!     }
//!     .print();
//! }
//! ```
//!
//! Your UI is composed primarily via the [`element!`] macro, which allows you to declare your UI
//! elements in a React/SwiftUI-like syntax.
//!
//! `iocraft` provides a few built-in components in the [`components`] module, such as
//! [`View`](crate::components::View), [`Text`](crate::components::Text), and
//! [`TextInput`](crate::components::TextInput), but you can also create your own using the
//! [`macro@component`] macro.
//!
//! For example, here's a custom component that uses a [hook](crate::hooks) to display a counter
//! which increments every 100ms:
//!
//! ```no_run
//! # use iocraft::prelude::*;
//! # use std::time::Duration;
//! #[component]
//! fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
//!     let mut count = hooks.use_state(|| 0);
//!
//!     hooks.use_future(async move {
//!         loop {
//!             smol::Timer::after(Duration::from_millis(100)).await;
//!             count += 1;
//!         }
//!     });
//!
//!     element! {
//!         Text(color: Color::Blue, content: format!("counter: {}", count))
//!     }
//! }
//! ```
//!
//! ## More Examples
//!
//! There are many [examples on GitHub](https://github.com/ccbrown/iocraft/tree/main/examples)
//! which demonstrate various concepts such as tables, progress bars, full screen apps, forms, and
//! more!.
//!
//! ## Shoutouts
//!
//! `iocraft` was inspired by [Dioxus](https://github.com/DioxusLabs/dioxus) and
//! [Ink](https://github.com/vadimdemedes/ink), which you should also check out, especially if
//! you're building graphical interfaces or interested in using JavaScript/TypeScript.
//!
//! You may also want to check out [Ratatui](https://github.com/ratatui/ratatui), which serves a
//! similar purpose with a less declarative API.

#![allow(clippy::needless_doctest_main)]
#![warn(missing_docs)]

// # Organization
//
// Code is organized into modules primarily for the benefit of the maintainers. Types will be
// re-exported in the root so that users of the library have a flat namespace to work with.
//
// The exception is the modules that represent collections of types, namely hooks and components.
// Those types will remain in their modules for the public API.

mod canvas;
mod component;
mod context;
mod element;
mod handler;
mod hook;
mod multimap;
mod props;
mod render;
mod style;
mod terminal;

mod flattened_exports {
    pub use crate::canvas::*;
    pub use crate::component::*;
    pub use crate::context::*;
    pub use crate::element::*;
    pub use crate::handler::*;
    pub use crate::hook::*;
    pub use crate::props::*;
    pub use crate::render::*;
    pub use crate::style::*;
    pub use crate::terminal::*;

    pub use iocraft_macros::*;
}

pub use flattened_exports::*;

/// Components for crafting your UI.
pub mod components;

pub mod hooks;

/// By importing this module, you'll bring all of the crate's commonly used types into scope.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::flattened_exports::*;
    pub use crate::hooks::*;
}

// So we can use our own macros.
extern crate self as iocraft;
