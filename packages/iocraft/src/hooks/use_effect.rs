use crate::{ComponentUpdater, Hook, Hooks};
use core::hash::{Hash, Hasher};
use std::hash::DefaultHasher;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseEffect` is a hook that allows you to execute a function after each update pass.
///
/// It will execute after each pass if the dependency has changed. If you want to execute it
/// exactly once, after the first pass, you can provide `()` as the dependency.
pub trait UseEffect: private::Sealed {
    /// Executes the given function after each update pass, if the dependency argument has changed.
    ///
    /// If you want to execute the function exactly once, after the first pass, you can provide
    /// `()` as the dependency.
    ///
    /// Changes to the dependencies are detected solely via the [`Hash`](std::hash::Hash) trait, so this
    /// function will hash them but not store them.
    ///
    /// To provide multiple dependencies, place your dependencies in a tuple.
    fn use_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: FnOnce() + Send + Unpin + 'static,
        D: Hash;
}

fn hash_deps<D: Hash>(deps: D) -> u64 {
    let mut hasher = DefaultHasher::new();
    deps.hash(&mut hasher);
    hasher.finish()
}

impl UseEffect for Hooks<'_, '_> {
    fn use_effect<F, D>(&mut self, f: F, deps: D)
    where
        F: FnOnce() + Send + Unpin + 'static,
        D: Hash,
    {
        let deps_hash = hash_deps(deps);
        let hook = self.use_hook(UseEffectImpl::<F>::default);
        if hook.deps_hash != deps_hash {
            hook.f = Some(f);
            hook.deps_hash = deps_hash;
        } else {
            hook.f = None;
        }
    }
}

struct UseEffectImpl<F> {
    deps_hash: u64,
    f: Option<F>,
}

impl<F> Default for UseEffectImpl<F> {
    fn default() -> Self {
        Self {
            deps_hash: 0,
            f: None,
        }
    }
}

impl<F: FnOnce() + Send + Unpin> Hook for UseEffectImpl<F> {
    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {
        if let Some(f) = self.f.take() {
            f();
        }
    }
}
