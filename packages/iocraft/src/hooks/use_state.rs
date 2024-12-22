use crate::{Hook, Hooks};
use core::{
    cmp,
    fmt::{self, Debug, Display, Formatter},
    ops,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use generational_box::{
    AnyStorage, BorrowError, BorrowMutError, GenerationalBox, Owner, SyncStorage,
};

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Hooks<'_, '_> {}
}

/// `UseState` is a hook that allows you to store state in a component.
///
/// When the state changes, the component will be re-rendered.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # use std::time::Duration;
/// #[component]
/// fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let mut count = hooks.use_state(|| 0);
///
///     hooks.use_future(async move {
///         loop {
///             smol::Timer::after(Duration::from_millis(100)).await;
///             count += 1;
///         }
///     });
///
///     element! {
///         Text(color: Color::Blue, content: format!("counter: {}", count))
///     }
/// }
/// ```
pub trait UseState: private::Sealed {
    /// Creates a new state with its initial value computed by the given function.
    ///
    /// When the state changes, the component will be re-rendered.
    fn use_state<T, F>(&mut self, initial_value: F) -> State<T>
    where
        T: Unpin + Sync + Send + 'static,
        F: FnOnce() -> T;
}

impl UseState for Hooks<'_, '_> {
    fn use_state<T, F>(&mut self, initial_value: F) -> State<T>
    where
        T: Unpin + Sync + Send + 'static,
        F: FnOnce() -> T,
    {
        self.use_hook(move || UseStateImpl::new(initial_value()))
            .state
    }
}

struct UseStateImpl<T: Unpin + Send + Sync + 'static> {
    _storage: Owner<SyncStorage>,
    state: State<T>,
}

impl<T: Unpin + Send + Sync + 'static> UseStateImpl<T> {
    pub fn new(initial_value: T) -> Self {
        let storage = Owner::default();
        UseStateImpl {
            state: State {
                inner: storage.insert(StateValue {
                    did_change: false,
                    waker: None,
                    value: initial_value,
                }),
            },
            _storage: storage,
        }
    }
}

impl<T: Unpin + Send + Sync + 'static> Hook for UseStateImpl<T> {
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Ok(mut value) = self.state.inner.try_write() {
            if value.did_change {
                value.did_change = false;
                Poll::Ready(())
            } else {
                value.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        } else {
            Poll::Pending
        }
    }
}

struct StateValue<T> {
    did_change: bool,
    waker: Option<Waker>,
    value: T,
}

/// A reference to the value of a [`State`].
pub struct StateRef<'a, T: 'static> {
    inner: <SyncStorage as AnyStorage>::Ref<'a, StateValue<T>>,
}

impl<T: 'static> ops::Deref for StateRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

/// A mutable reference to the value of a [`State`].
pub struct StateMutRef<'a, T: 'static> {
    inner: <SyncStorage as AnyStorage>::Mut<'a, StateValue<T>>,
    did_deref_mut: bool,
}

impl<T: 'static> ops::Deref for StateMutRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

impl<T: 'static> ops::DerefMut for StateMutRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.did_deref_mut = true;
        &mut self.inner.value
    }
}

impl<T: 'static> Drop for StateMutRef<'_, T> {
    fn drop(&mut self) {
        if self.did_deref_mut {
            self.inner.did_change = true;
            if let Some(waker) = self.inner.waker.take() {
                waker.wake();
            }
        }
    }
}

/// `State` is a copyable wrapper for a value that can be observed for changes. States used by a
/// component will cause the component to be re-rendered when its value changes.
///
/// # Panics
///
/// Attempts to read a state after its owner has been dropped will panic.
pub struct State<T: Send + Sync + 'static> {
    inner: GenerationalBox<StateValue<T>, SyncStorage>,
}

impl<T: Sync + Send + 'static> Clone for State<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Sync + Send + 'static> Copy for State<T> {}

impl<T: Copy + Sync + Send + 'static> State<T> {
    /// Gets a copy of the current value of the state.
    pub fn get(&self) -> T {
        *self.read()
    }
}

impl<T: Sync + Send + 'static> State<T> {
    /// Sets the value of the state.
    pub fn set(&mut self, value: T) {
        if let Some(v) = self.try_write() {
            *v = value;
        }
    }

    /// Returns a reference to the state's value.
    ///
    /// <div class="warning">It is possible to create a deadlock using this method. If you have
    /// multiple copies of the same state, writes to one will be blocked for as long as any
    /// reference returned by this method exists.</div>
    ///
    /// # Panics
    ///
    /// Panics if the owner of the state has been dropped.
    pub fn read(&self) -> StateRef<T> {
        self.try_read()
            .expect("attempt to read state after owner was dropped")
    }

    /// Returns a reference to the state's value, if its owner has not been dropped.
    ///
    /// <div class="warning">It is possible to create a deadlock using this method. If you have
    /// multiple copies of the same state, writes to one will be blocked for as long as any
    /// reference returned by this method exists.</div>
    pub fn try_read(&self) -> Option<StateRef<T>> {
        loop {
            match self.inner.try_read() {
                Ok(inner) => break Some(StateRef { inner }),
                Err(BorrowError::AlreadyBorrowedMut(_)) => match self.inner.try_write() {
                    Err(BorrowMutError::Dropped(_)) => break None,
                    _ => continue,
                },
                Err(BorrowError::Dropped(_)) => break None,
            };
        }
    }

    /// Returns a mutable reference to the state's value.
    ///
    /// <div class="warning">It is possible to create a deadlock using this method. If you have
    /// multiple copies of the same state, operations on one will be blocked for as long as any
    /// reference returned by this method exists.</div>
    ///
    /// # Panics
    ///
    /// Panics if the owner of the state has been dropped.
    pub fn write(&mut self) -> StateMutRef<T> {
        self.try_write()
            .expect("attempt to write state after owner was dropped")
    }

    /// Returns a mutable reference to the state's value, if its owner has not been dropped.
    ///
    /// <div class="warning">It is possible to create a deadlock using this method. If you have
    /// multiple copies of the same state, operations on one will be blocked for as long as any
    /// reference returned by this method exists.</div>
    pub fn try_write(&mut self) -> Option<StateMutRef<T>> {
        self.inner.try_write().ok().map(|inner| StateMutRef {
            inner,
            did_deref_mut: false,
        })
    }
}

impl<T: Debug + Sync + Send + 'static> Debug for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: Display + Sync + Send + 'static> Display for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.read().fmt(f)
    }
}

impl<T: ops::Add<Output = T> + Copy + Sync + Send + 'static> ops::Add<T> for State<T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.get() + rhs
    }
}

impl<T: ops::AddAssign<T> + Copy + Sync + Send + 'static> ops::AddAssign<T> for State<T> {
    fn add_assign(&mut self, rhs: T) {
        if let Some(v) = self.try_write() {
            *v += rhs;
        }
    }
}

impl<T: ops::Sub<Output = T> + Copy + Sync + Send + 'static> ops::Sub<T> for State<T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.get() - rhs
    }
}

impl<T: ops::SubAssign<T> + Copy + Sync + Send + 'static> ops::SubAssign<T> for State<T> {
    fn sub_assign(&mut self, rhs: T) {
        if let Some(v) = self.try_write() {
            *v -= rhs;
        }
    }
}

impl<T: ops::Mul<Output = T> + Copy + Sync + Send + 'static> ops::Mul<T> for State<T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.get() * rhs
    }
}

impl<T: ops::MulAssign<T> + Copy + Sync + Send + 'static> ops::MulAssign<T> for State<T> {
    fn mul_assign(&mut self, rhs: T) {
        if let Some(v) = self.try_write() {
            *v *= rhs;
        }
    }
}

impl<T: ops::Div<Output = T> + Copy + Sync + Send + 'static> ops::Div<T> for State<T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.get() / rhs
    }
}

impl<T: ops::DivAssign<T> + Copy + Sync + Send + 'static> ops::DivAssign<T> for State<T> {
    fn div_assign(&mut self, rhs: T) {
        if let Some(v) = self.try_write() {
            *v /= rhs;
        }
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<T> for State<T> {
    fn eq(&self, other: &T) -> bool {
        *self.read() == *other
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<T> for State<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.read().partial_cmp(other)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<State<T>> for State<T> {
    fn eq(&self, other: &State<T>) -> bool {
        *self.read() == *other.read()
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<State<T>> for State<T> {
    fn partial_cmp(&self, other: &State<T>) -> Option<cmp::Ordering> {
        self.read().partial_cmp(&other.read())
    }
}

impl<T: cmp::Eq + Sync + Send + 'static> cmp::Eq for State<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::pin::Pin;
    use futures::task::noop_waker;

    #[test]
    fn test_state() {
        let mut hook = UseStateImpl::new(42);
        let mut state = hook.state;
        assert_eq!(state.get(), 42);

        state.set(43);
        assert_eq!(state, 43);
        assert_eq!(
            Pin::new(&mut hook).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Ready(())
        );
        assert_eq!(
            Pin::new(&mut hook).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Pending
        );

        assert_eq!(state.to_string(), "43");

        assert_eq!(state.clone() + 1, 44);
        state += 1;
        assert_eq!(state, 44);

        assert_eq!(state.clone() - 1, 43);
        state -= 1;
        assert_eq!(state, 43);

        assert_eq!(state.clone() * 2, 86);
        state *= 2;
        assert_eq!(state, 86);

        assert_eq!(state.clone() / 2, 43);
        state /= 2;
        assert_eq!(state, 43);

        assert!(state > 42);
        assert!(state >= 43);
        assert!(state < 44);

        assert_eq!(*state.write(), 43);

        let state_copy = state.clone();
        assert_eq!(*state.read(), *state_copy.read());
    }
}
