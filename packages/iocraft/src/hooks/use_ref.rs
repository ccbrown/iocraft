use crate::{Hook, Hooks};
use core::{
    cmp,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops,
};
use generational_box::{
    AnyStorage, BorrowError, BorrowMutError, GenerationalBox, Owner, SyncStorage,
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `Ref` is a copyable wrapper for a value that is owned by a component but does not cause
/// re-renders.
///
/// # Panics
///
/// Attempts to read a ref after its owner has been dropped will panic.
pub struct Ref<T: Send + Sync + 'static> {
    inner: GenerationalBox<T, SyncStorage>,
}

/// A reference to the value of a [`Ref`].
pub struct RefRef<'a, T: 'static> {
    inner: <SyncStorage as AnyStorage>::Ref<'a, T>,
}

impl<T: 'static> ops::Deref for RefRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A mutable reference to the value of a [`Ref`].
pub struct RefMutRef<'a, T: 'static> {
    inner: <SyncStorage as AnyStorage>::Mut<'a, T>,
}

impl<T: 'static> ops::Deref for RefMutRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: 'static> ops::DerefMut for RefMutRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Copy + Sync + Send + 'static> Ref<T> {
    /// Gets a copy of the current value of the ref.
    ///
    /// # Panics
    ///
    /// Panics if the owner of the ref has been dropped.
    pub fn get(&self) -> T {
        *self.read()
    }

    /// Gets a copy of the current value of the ref, if its owner has not been dropped.
    pub fn try_get(&self) -> Option<T> {
        self.try_read().map(|v| *v)
    }
}

impl<T: Sync + Send + 'static> Ref<T> {
    /// Sets the value of the ref.
    pub fn set(&mut self, value: T) {
        if let Some(mut v) = self.try_write() {
            *v = value;
        }
    }

    /// Returns a reference to the ref's value.
    ///
    /// <div class="warning">It is possible to create a deadlock using this method. If you have
    /// multiple copies of the same ref, writes to one will be blocked for as long as any
    /// reference returned by this method exists.</div>
    ///
    /// # Panics
    ///
    /// Panics if the owner of the ref has been dropped.
    pub fn read(&self) -> RefRef<T> {
        self.try_read()
            .expect("attempt to read ref after owner was dropped")
    }

    /// Returns a reference to the ref's value, if its owner has not been dropped.
    ///
    /// Most applications should not need to use this method. If you only read the ref's value
    /// from your component and its hooks, you should use [`read`](Ref::read) instead.
    ///
    /// <div class="warning">It is possible to create a deadlock using this method. If you have
    /// multiple copies of the same ref, writes to one will be blocked for as long as any
    /// reference returned by this method exists.</div>
    pub fn try_read(&self) -> Option<RefRef<T>> {
        loop {
            match self.inner.try_read() {
                Ok(inner) => break Some(RefRef { inner }),
                Err(BorrowError::AlreadyBorrowedMut(_)) => match self.inner.try_write() {
                    Err(BorrowMutError::Dropped(_)) => break None,
                    _ => continue,
                },
                Err(BorrowError::Dropped(_)) => break None,
            };
        }
    }

    /// Returns a mutable reference to the ref's value.
    ///
    /// <div class="warning">It is possible to create a deadlock using this method. If you have
    /// multiple copies of the same ref, operations on one will be blocked for as long as any
    /// reference returned by this method exists.</div>
    ///
    /// # Panics
    ///
    /// Panics if the owner of the ref has been dropped.
    pub fn write(&mut self) -> RefMutRef<T> {
        self.try_write()
            .expect("attempt to write ref after owner was dropped")
    }

    /// Returns a mutable reference to the ref's value, if its owner has not been dropped.
    ///
    /// Most applications should not need to use this method. If you only write the ref's value
    /// from your component and its hooks, you should use [`write`](Ref::write) instead.
    ///
    /// <div class="warning">It is possible to create a deadlock using this method. If you have
    /// multiple copies of the same ref, operations on one will be blocked for as long as any
    /// reference returned by this method exists.</div>
    pub fn try_write(&mut self) -> Option<RefMutRef<T>> {
        self.inner.try_write().ok().map(|inner| RefMutRef { inner })
    }
}

impl<T: Sync + Send + 'static> Clone for Ref<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Sync + Send + 'static> Copy for Ref<T> {}

impl<T: Debug + Sync + Send + 'static> Debug for Ref<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: Display + Sync + Send + 'static> Display for Ref<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: ops::Add<Output = T> + Copy + Sync + Send + 'static> ops::Add<T> for Ref<T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.get() + rhs
    }
}

impl<T: ops::AddAssign<T> + Copy + Sync + Send + 'static> ops::AddAssign<T> for Ref<T> {
    fn add_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v += rhs;
        }
    }
}

impl<T: ops::Sub<Output = T> + Copy + Sync + Send + 'static> ops::Sub<T> for Ref<T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.get() - rhs
    }
}

impl<T: ops::SubAssign<T> + Copy + Sync + Send + 'static> ops::SubAssign<T> for Ref<T> {
    fn sub_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v -= rhs;
        }
    }
}

impl<T: ops::Mul<Output = T> + Copy + Sync + Send + 'static> ops::Mul<T> for Ref<T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.get() * rhs
    }
}

impl<T: ops::MulAssign<T> + Copy + Sync + Send + 'static> ops::MulAssign<T> for Ref<T> {
    fn mul_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v *= rhs;
        }
    }
}

impl<T: ops::Div<Output = T> + Copy + Sync + Send + 'static> ops::Div<T> for Ref<T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.get() / rhs
    }
}

impl<T: ops::DivAssign<T> + Copy + Sync + Send + 'static> ops::DivAssign<T> for Ref<T> {
    fn div_assign(&mut self, rhs: T) {
        if let Some(mut v) = self.try_write() {
            *v /= rhs;
        }
    }
}

impl<T: Hash + Sync + Send> Hash for Ref<T> {
    fn hash<H: Hasher>(&self, hash: &mut H) {
        self.read().hash(hash)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<T> for Ref<T> {
    fn eq(&self, other: &T) -> bool {
        *self.read() == *other
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<T> for Ref<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.read().partial_cmp(other)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<Ref<T>> for Ref<T> {
    fn eq(&self, other: &Ref<T>) -> bool {
        *self.read() == *other.read()
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<Ref<T>> for Ref<T> {
    fn partial_cmp(&self, other: &Ref<T>) -> Option<cmp::Ordering> {
        self.read().partial_cmp(&other.read())
    }
}

impl<T: cmp::Eq + Sync + Send + 'static> cmp::Eq for Ref<T> {}

/// `UseRef` is a hook that allows you to store a value which can be modified but doesn't impact
/// rendering.
///
/// It is almost identical to [`UseState`](crate::hooks::UseState) but doesn't cause re-renders
/// when the value is changed.
///
/// Refs can be used for anything, but one common use for them is imperative control of components
/// via handles.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # #[component]
/// # fn FormField(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
/// # let mut value = hooks.use_state(|| "".to_string());
/// # let initial_cursor_position = 0;
/// let mut handle = hooks.use_ref_default::<TextInputHandle>();
///
/// hooks.use_effect(
///     move || handle.write().set_cursor_offset(initial_cursor_position),
///     (),
/// );
///
/// element! {
///     View(
///         background_color: Color::DarkGrey,
///         width: 30,
///     ) {
///         TextInput(
///             has_focus: true,
///             value: value.to_string(),
///             on_change: move |new_value| value.set(new_value),
///             handle,
///         )
///     }
/// }
/// # }
/// ```
pub trait UseRef: private::Sealed {
    /// Creates a new ref with its initial value computed by the given function.
    fn use_ref<F, T>(&mut self, f: F) -> Ref<T>
    where
        F: FnOnce() -> T,
        T: Send + Sync + Unpin + 'static;

    /// Creates a new ref with its initial value default constructed.
    fn use_ref_default<T>(&mut self) -> Ref<T>
    where
        T: Default + Send + Sync + Unpin + 'static;
}

impl UseRef for Hooks<'_, '_> {
    fn use_ref<F, T>(&mut self, f: F) -> Ref<T>
    where
        F: FnOnce() -> T,
        T: Send + Sync + Unpin + 'static,
    {
        let hook = self.use_hook(move || UseRefImpl::new(f()));
        hook.value
    }

    fn use_ref_default<T>(&mut self) -> Ref<T>
    where
        T: Default + Send + Sync + Unpin + 'static,
    {
        self.use_ref(T::default)
    }
}

struct UseRefImpl<T: Unpin + Send + Sync + 'static> {
    _storage: Owner<SyncStorage>,
    value: Ref<T>,
}

impl<T: Unpin + Send + Sync + 'static> UseRefImpl<T> {
    pub fn new(initial_value: T) -> Self {
        let storage = Owner::default();
        UseRefImpl {
            value: Ref {
                inner: storage.insert(initial_value),
            },
            _storage: storage,
        }
    }
}

impl<T: Send + Sync + Unpin> Hook for UseRefImpl<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{
        pin::Pin,
        task::{Context, Poll},
    };
    use futures::task::noop_waker;

    #[test]
    fn test_ref() {
        let mut hook = UseRefImpl::new(42);
        let mut value = hook.value;
        assert_eq!(value.get(), 42);

        value.set(43);
        assert_eq!(value, 43);
        assert_eq!(
            Pin::new(&mut hook).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Pending
        );

        assert_eq!(value.to_string(), "43");

        assert_eq!(value + 1, 44);
        value += 1;
        assert_eq!(value, 44);

        assert_eq!(value - 1, 43);
        value -= 1;
        assert_eq!(value, 43);

        assert_eq!(value * 2, 86);
        value *= 2;
        assert_eq!(value, 86);

        assert_eq!(value / 2, 43);
        value /= 2;
        assert_eq!(value, 43);

        assert!(value > 42);
        assert!(value >= 43);
        assert!(value < 44);

        assert_eq!(*value.write(), 43);

        let ref_copy = value;
        assert_eq!(*value.read(), *ref_copy.read());
    }

    #[test]
    fn test_dropped_ref() {
        let hook = UseRefImpl::new(42);

        let mut value = hook.value;
        assert_eq!(value.get(), 42);

        drop(hook);

        assert!(value.try_read().is_none());
        assert!(value.try_write().is_none());

        // these should be no-ops
        value.set(43);
        value += 1;
        value -= 1;
        value *= 2;
        value /= 2;
    }
}
