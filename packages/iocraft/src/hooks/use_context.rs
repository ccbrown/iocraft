use crate::Hooks;
use core::{
    any::Any,
    cell::{Ref, RefMut},
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseContext` provides methods for accessing context from a component.
///
/// With the exception of [`SystemContext`](crate::SystemContext), which is always available,
/// contexts are provided via the [`ContextProvider`](crate::components::ContextProvider)
/// component.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// struct NumberOfTheDay(i32);
///
/// #[component]
/// fn MyContextConsumer(hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let number = hooks.use_context::<NumberOfTheDay>();
///
///     element! {
///         View(border_style: BorderStyle::Round, border_color: Color::Cyan) {
///             Text(content: "The number of the day is... ")
///             Text(color: Color::Green, weight: Weight::Bold, content: number.0.to_string())
///             Text(content: "!")
///         }
///     }
/// }
/// ```
pub trait UseContext<'a>: private::Sealed {
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
