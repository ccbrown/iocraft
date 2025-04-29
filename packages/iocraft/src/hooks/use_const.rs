use crate::{Hook, Hooks};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseConst` is a hook that allows you to store a value which doesn't change.
///
/// It can be used for complex values that are constant across renders or it can be used to create
/// controllers, caches, channels, or other objects which don't directly impact the component's
/// output.
///
/// It is similar to [`UseMemo`](crate::hooks::UseMemo), but it never recomputes the value. If the
/// value is dependent on state, props, or other dynamic values, use
/// [`UseMemo`](crate::hooks::UseMemo) instead.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # use std::sync::Arc;
/// # #[derive(Clone, Default, Props)]
/// # struct Data;
/// # #[derive(Clone, Default, Props)]
/// # struct DataViewProps {
/// #     data: Arc<Data>,
/// # }
/// # #[component]
/// # fn DataView(props: &DataViewProps) -> impl Into<AnyElement<'static>> {
/// #     element!(View)
/// # }
/// #[component]
/// fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let data: Arc<Data> = hooks.use_const_default();
///
///     element! {
///         DataView(data)
///     }
/// }
/// ```
pub trait UseConst: private::Sealed {
    /// Returns a constant value, initialized by the provided function.
    ///
    /// The returned value is cloned, so if cloning is expensive, consider using a reference type
    /// such as [`Arc`](std::sync::Arc).
    fn use_const<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> T,
        T: Clone + Send + Unpin + 'static;

    /// Returns a constant value, initialized by its default value.
    ///
    /// The returned value is cloned, so if cloning is expensive, consider using a reference type
    /// such as [`Arc`](std::sync::Arc).
    fn use_const_default<T>(&mut self) -> T
    where
        T: Clone + Default + Send + Unpin + 'static;
}

impl UseConst for Hooks<'_, '_> {
    fn use_const<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> T,
        T: Clone + Send + Unpin + 'static,
    {
        let hook = self.use_hook(move || UseConstImpl { value: f() });
        hook.value.clone()
    }

    fn use_const_default<T>(&mut self) -> T
    where
        T: Clone + Default + Send + Unpin + 'static,
    {
        self.use_const(T::default)
    }
}

struct UseConstImpl<T> {
    value: T,
}

impl<T: Send + Unpin> Hook for UseConstImpl<T> {}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[derive(Clone, Default)]
    struct MyStruct(u64);

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let a = hooks.use_const(|| 42);
        let b: MyStruct = hooks.use_const_default();
        let c = hooks.use_const(|| "hello!");

        assert_eq!(a, 42);
        assert_eq!(b.0, 0);
        assert_eq!(c, "hello!");

        element!(View)
    }

    #[test]
    fn test_use_const() {
        let s = element!(MyComponent).to_string();
        assert_eq!(s, "");
    }
}
