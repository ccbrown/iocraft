use crate::{ComponentDrawer, ComponentUpdater, ContextStack};
use std::{
    any::Any,
    cell::{Ref, RefMut},
    pin::Pin,
    task::{Context, Poll},
};

/// A hook is a way to add behavior to a component. Hooks are called at various points in the
/// update and draw cycle.
///
/// Hooks are created by implementing this trait. All methods have default implementations, so
/// you only need to implement the ones you care about.
pub trait Hook: Unpin {
    /// Called to determine if the hook has caused a change which requires its component to be
    /// redrawn.
    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<()> {
        Poll::Pending
    }

    /// Called before the component is updated.
    fn pre_component_update(&mut self, _updater: &mut ComponentUpdater) {}

    /// Called after the component is updated.
    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {}

    /// Called before the component is drawn.
    fn pre_component_draw(&mut self, _drawer: &mut ComponentDrawer) {}

    /// Called after the component is drawn.
    fn post_component_draw(&mut self, _drawer: &mut ComponentDrawer) {}
}

pub(crate) trait AnyHook: Hook {
    fn any_self_mut(&mut self) -> &mut dyn Any;
}

impl<T: Hook + 'static> AnyHook for T {
    fn any_self_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Hook for Vec<Box<dyn AnyHook>> {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let mut is_ready = false;
        for hook in self.iter_mut() {
            if let Poll::Ready(()) = Pin::new(&mut **hook).poll_change(cx) {
                is_ready = true;
            }
        }

        if is_ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    fn pre_component_update(&mut self, updater: &mut ComponentUpdater) {
        for hook in self.iter_mut() {
            hook.pre_component_update(updater);
        }
    }

    fn post_component_update(&mut self, updater: &mut ComponentUpdater) {
        for hook in self.iter_mut() {
            hook.post_component_update(updater);
        }
    }

    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        for hook in self.iter_mut() {
            hook.pre_component_draw(drawer);
        }
    }

    fn post_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        for hook in self.iter_mut() {
            hook.post_component_draw(drawer);
        }
    }
}

/// A collection of hooks attached to a component.
///
/// Custom hooks can be defined by creating a trait with additional methods and implementing it for
/// `Hooks<'_, '_>`.
pub struct Hooks<'a, 'b: 'a> {
    hooks: &'a mut Vec<Box<dyn AnyHook>>,
    first_update: bool,
    hook_index: usize,
    context_stack: Option<&'a ContextStack<'b>>,
}

impl<'a, 'b> Hooks<'a, 'b> {
    pub(crate) fn new(hooks: &'a mut Vec<Box<dyn AnyHook>>, first_update: bool) -> Self {
        Self {
            hooks,
            first_update,
            hook_index: 0,
            context_stack: None,
        }
    }

    #[doc(hidden)]
    pub fn with_context_stack<'c, 'd>(
        &'c mut self,
        context_stack: &'c ContextStack<'d>,
    ) -> Hooks<'c, 'd> {
        Hooks {
            hooks: self.hooks,
            first_update: self.first_update,
            hook_index: self.hook_index,
            context_stack: Some(context_stack),
        }
    }

    /// Returns a reference to the context of the given type.
    ///
    /// # Panics
    ///
    /// Panics if the context is not available.
    pub fn use_context<T: Any>(&self) -> Ref<'a, T> {
        self.context_stack
            .expect("context not available")
            .get_context::<T>()
            .expect("context not found")
    }

    /// Returns a mutable reference to the context of the given type.
    ///
    /// # Panics
    ///
    /// Panics if the context is not available or is not mutable.
    pub fn use_context_mut<T: Any>(&self) -> RefMut<'a, T> {
        self.context_stack
            .expect("context not available")
            .get_context_mut::<T>()
            .expect("context not found")
    }

    /// Returns a reference to the context of the given type, if it is available.
    pub fn try_use_context<T: Any>(&self) -> Option<Ref<'a, T>> {
        self.context_stack
            .and_then(|stack| stack.get_context::<T>())
    }

    /// Returns a mutable reference to the context of the given type, if it is available and mutable.
    pub fn try_use_context_mut<T: Any>(&self) -> Option<RefMut<'a, T>> {
        self.context_stack
            .and_then(|stack| stack.get_context_mut::<T>())
    }

    /// If this is the component's first render, this function adds a new hook to the component and
    /// returns it.
    ///
    /// If it is a subsequent render, this function does nothing and returns the hook that was
    /// added during the first render.
    pub fn use_hook<H, F>(&mut self, f: F) -> &mut H
    where
        F: FnOnce() -> H,
        H: Hook + Unpin + 'static,
    {
        if self.first_update {
            self.hooks.push(Box::new(f()));
        }

        let idx = self.hook_index;
        self.hook_index += 1;
        self.hooks.get_mut(idx).and_then(|hook| hook.any_self_mut().downcast_mut::<H>()).expect("Unexpected hook type! Most likely you've violated the rules of hooks and called this hook in a different order than the previous render.")
    }
}
