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

mod use_context;
pub use use_context::*;
mod use_future;
pub use use_future::*;
mod use_output;
pub use use_output::*;
mod use_state;
pub use use_state::*;
mod use_terminal_events;
pub use use_terminal_events::*;
mod use_terminal_size;
pub use use_terminal_size::*;
