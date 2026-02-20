//! This module contains hooks that can be used to add behavior to components.
//!
//! Hooks are implemented as traits which extend the [`Hooks`](crate::Hooks) object that gets passed to your component.
//!
//! For example, if you want to create a hook that acts as a shorthand for getting the user's
//! username from context, you might define and use it like this:
//!
//! ```
//! # use iocraft::prelude::*;
//! # struct UserInfo { name: String }
//! pub trait UseUserInfo {
//!     /// Returns the user's username.
//!     fn use_username(&mut self) -> String;
//! }
//!
//! impl UseUserInfo for Hooks<'_, '_> {
//!     fn use_username(&mut self) -> String {
//!         self.use_context::<UserInfo>().name.to_string()
//!     }
//! }
//!
//! #[component]
//! fn Greeting(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
//!     let username = hooks.use_username();
//!     element! {
//!         Text(content: format!("Hello, {}!", username))
//!     }
//! }
//! ```
//!
//! # Rules of Hooks
//!
//! Usage of hooks is subject to the same sorts of rules as [React hooks](https://react.dev/reference/rules/rules-of-hooks).
//!
//! They must be called in the same order every time, so calling them in any sort of conditional or
//! loop is not allowed. If you break the rules of hooks, you can expect a panic.
//!
//! # Note to Library Authors
//!
//! If you are writing a library that provides hooks, it's recommended that you seal your hook
//! traits so you can add new methods without breaking semver compatibility.

mod use_async_handler;
pub use use_async_handler::*;
mod use_const;
pub use use_const::*;
mod use_context;
pub use use_context::*;
mod use_effect;
pub use use_effect::*;
mod use_future;
pub use use_future::*;
mod use_memo;
pub use use_memo::*;
mod use_output;
pub use use_output::*;
mod use_ref;
pub use use_ref::*;
mod use_state;
pub use use_state::*;
mod use_terminal_events;
pub use use_terminal_events::*;
mod use_terminal_size;
pub use use_terminal_size::*;
mod use_component_rect;
pub use use_component_rect::*;
