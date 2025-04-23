use crate::{Hook, Hooks};
use core::hash::{Hash, Hasher};
use std::hash::DefaultHasher;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseMemo` is a hook that allows you to memoize a value, recomputing it only if any of its
/// listed dependencies change.
///
/// It can also be used to simply execute a function in response to changes, or execute a function
/// exactly once by providing `()` as the dependency.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # #[derive(Default, Props)]
/// # struct MyProps {
/// #     n: u64,
/// # }
/// # fn factor(n: u64) -> Vec<u64> {
/// #     unimplemented!()
/// # }
/// #[component]
/// fn MyComponent(mut hooks: Hooks, props: &MyProps) -> impl Into<AnyElement<'static>> {
///     let factors = hooks.use_memo(|| factor(props.n), props.n);
///     let factors_csv = factors.iter().map(u64::to_string).collect::<Vec<_>>().join(", ");
///
///     element! {
///         Text(content: format!("factors: {}", factors_csv))
///     }
/// }
/// ```
pub trait UseMemo: private::Sealed {
    /// Returns a memoized value, recomputing it only if any of the dependency argument changes.
    ///
    /// Changes to the dependencies are detected solely via the [`Hash`](std::hash::Hash) trait, so this
    /// function will hash them but not store them.
    ///
    /// To provide multiple dependencies, place your dependencies in a tuple.
    ///
    /// The returned value is cloned, so if cloning is expensive, consider using a reference type
    /// such as [`Arc`](std::sync::Arc).
    fn use_memo<F, D, T>(&mut self, f: F, deps: D) -> T
    where
        F: FnOnce() -> T,
        D: Hash,
        T: Clone + Send + Unpin + 'static;
}

fn hash_deps<D: Hash>(deps: D) -> u64 {
    let mut hasher = DefaultHasher::new();
    deps.hash(&mut hasher);
    hasher.finish()
}

impl UseMemo for Hooks<'_, '_> {
    fn use_memo<F, D, T>(&mut self, f: F, deps: D) -> T
    where
        F: FnOnce() -> T,
        D: Hash,
        T: Clone + Send + Unpin + 'static,
    {
        let deps_hash = hash_deps(deps);
        let hook = self.use_hook(UseMemoImpl::<T>::default);
        if hook.memoized_value.is_none() || hook.deps_hash != deps_hash {
            hook.memoized_value = Some(f());
            hook.deps_hash = deps_hash;
        }
        hook.memoized_value.clone().unwrap()
    }
}

struct UseMemoImpl<T> {
    deps_hash: u64,
    memoized_value: Option<T>,
}

impl<T> Default for UseMemoImpl<T> {
    fn default() -> Self {
        Self {
            deps_hash: 0,
            memoized_value: None,
        }
    }
}

impl<T: Send + Unpin> Hook for UseMemoImpl<T> {}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[derive(Clone)]
    struct MyStruct(u64);

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let s = "foo bar";
        let counter = hooks.use_state(|| 0);

        let a = hooks.use_memo(|| 42, (counter, s));
        let b = hooks.use_memo(|| MyStruct(1), (counter, s));
        let c = hooks.use_memo(|| "hello!", (counter, s));
        hooks.use_memo(|| {}, (counter, s));
        assert_eq!(a, 42);
        assert_eq!(b.0, 1);
        assert_eq!(c, "hello!");

        element!(View)
    }

    #[test]
    fn test_use_memo() {
        let s = element!(MyComponent).to_string();
        assert_eq!(s, "");
    }
}
