use crate::{Hook, Hooks};
use generational_box::{AnyStorage, GenerationalBox, Owner, SyncStorage};
use std::{
    cmp,
    fmt::{self, Debug, Display, Formatter},
    ops,
    pin::Pin,
    task::{Context, Poll, Waker},
};

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
pub trait UseState {
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
pub struct StateRef<T: 'static> {
    inner: <SyncStorage as AnyStorage>::Ref<'static, StateValue<T>>,
}

impl<T: 'static> ops::Deref for StateRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

/// `State` is a copyable wrapper for a value that can be observed for changes. States used by a
/// component will cause the component to be re-rendered when its value changes.
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
        self.inner.read().value
    }
}

impl<T: Sync + Send + 'static> State<T> {
    /// Sets the value of the state.
    pub fn set(&self, value: T) {
        self.modify(|v| *v = value);
    }

    /// Returns a reference to the state's value.
    pub fn read(&self) -> StateRef<T> {
        StateRef {
            inner: self.inner.read(),
        }
    }

    fn modify<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut inner = self.inner.write();
        f(&mut inner.value);
        inner.did_change = true;
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

impl<T: Debug + Sync + Send + 'static> Debug for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.read().value.fmt(f)
    }
}

impl<T: Display + Sync + Send + 'static> Display for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.read().value.fmt(f)
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
        self.modify(|v| *v += rhs);
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
        self.modify(|v| *v -= rhs);
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
        self.modify(|v| *v *= rhs);
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
        self.modify(|v| *v /= rhs);
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<T> for State<T> {
    fn eq(&self, other: &T) -> bool {
        self.inner.read().value == *other
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<T> for State<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.inner.read().value.partial_cmp(other)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<State<T>> for State<T> {
    fn eq(&self, other: &State<T>) -> bool {
        self.inner.read().value == other.inner.read().value
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<State<T>> for State<T> {
    fn partial_cmp(&self, other: &State<T>) -> Option<cmp::Ordering> {
        self.inner
            .read()
            .value
            .partial_cmp(&other.inner.read().value)
    }
}

impl<T: cmp::Eq + Sync + Send + 'static> cmp::Eq for State<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::noop_waker;
    use std::pin::Pin;

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
    }
}
