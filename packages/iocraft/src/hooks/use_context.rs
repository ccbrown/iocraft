use crate::Hooks;
use std::{
    any::Any,
    cell::{Ref, RefMut},
};

/// `UseContext` provides methods for accessing context from a component.
pub trait UseContext<'a> {
    /// Returns a reference to the context of the given type.
    ///
    /// # Panics
    ///
    /// Panics if the context is not available.
    fn use_context<T: Any>(&self) -> Ref<'a, T>;

    /// Returns a mutable reference to the context of the given type.
    ///
    /// # Panics
    ///
    /// Panics if the context is not available or is not mutable.
    fn use_context_mut<T: Any>(&self) -> RefMut<'a, T>;

    /// Returns a reference to the context of the given type, if it is available.
    fn try_use_context<T: Any>(&self) -> Option<Ref<'a, T>>;

    /// Returns a mutable reference to the context of the given type, if it is available and mutable.
    fn try_use_context_mut<T: Any>(&self) -> Option<RefMut<'a, T>>;
}

impl<'a> UseContext<'a> for Hooks<'a, '_> {
    fn use_context<T: Any>(&self) -> Ref<'a, T> {
        self.context_stack
            .expect("context not available")
            .get_context::<T>()
            .expect("context not found")
    }

    fn use_context_mut<T: Any>(&self) -> RefMut<'a, T> {
        self.context_stack
            .expect("context not available")
            .get_context_mut::<T>()
            .expect("context not found")
    }

    fn try_use_context<T: Any>(&self) -> Option<Ref<'a, T>> {
        self.context_stack
            .and_then(|stack| stack.get_context::<T>())
    }

    fn try_use_context_mut<T: Any>(&self) -> Option<RefMut<'a, T>> {
        self.context_stack
            .and_then(|stack| stack.get_context_mut::<T>())
    }
}
