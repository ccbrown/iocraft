use crate::ComponentUpdater;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// A hook is a way to add behavior to a component. Hooks are called at various points in the
/// update and render cycle.
///
/// Hooks are created by implementing this trait. All methods have default implementations, so
/// you only need to implement the ones you care about.
pub trait Hook: Default {
    /// Called to determine if the hook has caused a change which requires its component to be
    /// re-rendered.
    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        Poll::Pending
    }

    /// Called before the component is updated.
    fn pre_component_update(&mut self, _updater: &mut ComponentUpdater) {}

    /// Called after the component is updated.
    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {}
}
