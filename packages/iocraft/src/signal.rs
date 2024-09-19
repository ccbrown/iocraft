use generational_box::{AnyStorage, GenerationalBox, Owner, SyncStorage};
use std::{
    cmp,
    fmt::{self, Debug, Display, Formatter},
    ops,
    pin::Pin,
    task::{Context, Poll, Waker},
};

trait SignalValueGenerationalBox {
    fn poll_change_unpin(&mut self, cx: &mut Context<'_>) -> Poll<()>;
}

impl<T: Sync + Send + 'static> SignalValueGenerationalBox
    for GenerationalBox<SignalValue<T>, SyncStorage>
{
    fn poll_change_unpin(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Ok(mut value) = self.try_write() {
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

struct SignalValue<T> {
    did_change: bool,
    waker: Option<Waker>,
    value: T,
}

#[doc(hidden)]
#[derive(Default)]
pub struct SignalOwner {
    storage: Owner<SyncStorage>,
    signals: Vec<Box<dyn SignalValueGenerationalBox>>,
}

impl SignalOwner {
    pub fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut is_ready = false;
        for signal in self.signals.iter_mut() {
            if signal.poll_change_unpin(cx).is_ready() {
                is_ready = true;
            }
        }
        if is_ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl SignalOwner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_signal<T: Send + Sync + 'static>(&mut self, value: T) -> Signal<T> {
        let key = self.storage.insert(SignalValue {
            did_change: false,
            waker: None,
            value,
        });
        self.signals.push(Box::new(key));
        Signal { inner: key }
    }

    pub fn new_signal_with_default<T: Default + Send + Sync + 'static>(&mut self) -> Signal<T> {
        self.new_signal(T::default())
    }
}

/// A reference to the value of a [`Signal`].
pub struct SignalRef<T: 'static> {
    inner: <SyncStorage as AnyStorage>::Ref<'static, SignalValue<T>>,
}

impl<T: 'static> ops::Deref for SignalRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner.value
    }
}

/// `Signal` is a clonable wrapper for a value that can be observed for changes. Signals used as
/// part of a component's state will cause the component to be re-rendered when the signal's value
/// changes.
pub struct Signal<T: Send + Sync + 'static> {
    inner: GenerationalBox<SignalValue<T>, SyncStorage>,
}

impl<T: Sync + Send + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Sync + Send + 'static> Copy for Signal<T> {}

impl<T: Copy + Sync + Send + 'static> Signal<T> {
    /// Gets a copy of the current value of the signal.
    pub fn get(&self) -> T {
        self.inner.read().value
    }
}

impl<T: Sync + Send + 'static> Signal<T> {
    /// Sets the value of the signal.
    pub fn set(&self, value: T) {
        self.modify(|v| *v = value);
    }

    /// Returns a reference to the signal's value.
    pub fn read(&self) -> SignalRef<T> {
        SignalRef {
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

impl<T: Debug + Sync + Send + 'static> Debug for Signal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.read().value.fmt(f)
    }
}

impl<T: Display + Sync + Send + 'static> Display for Signal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.read().value.fmt(f)
    }
}

impl<T: ops::Add<Output = T> + Copy + Sync + Send + 'static> ops::Add<T> for Signal<T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.get() + rhs
    }
}

impl<T: ops::AddAssign<T> + Copy + Sync + Send + 'static> ops::AddAssign<T> for Signal<T> {
    fn add_assign(&mut self, rhs: T) {
        self.modify(|v| *v += rhs);
    }
}

impl<T: ops::Sub<Output = T> + Copy + Sync + Send + 'static> ops::Sub<T> for Signal<T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.get() - rhs
    }
}

impl<T: ops::SubAssign<T> + Copy + Sync + Send + 'static> ops::SubAssign<T> for Signal<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.modify(|v| *v -= rhs);
    }
}

impl<T: ops::Mul<Output = T> + Copy + Sync + Send + 'static> ops::Mul<T> for Signal<T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.get() * rhs
    }
}

impl<T: ops::MulAssign<T> + Copy + Sync + Send + 'static> ops::MulAssign<T> for Signal<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.modify(|v| *v *= rhs);
    }
}

impl<T: ops::Div<Output = T> + Copy + Sync + Send + 'static> ops::Div<T> for Signal<T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.get() / rhs
    }
}

impl<T: ops::DivAssign<T> + Copy + Sync + Send + 'static> ops::DivAssign<T> for Signal<T> {
    fn div_assign(&mut self, rhs: T) {
        self.modify(|v| *v /= rhs);
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<T> for Signal<T> {
    fn eq(&self, other: &T) -> bool {
        self.inner.read().value == *other
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<T> for Signal<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.inner.read().value.partial_cmp(other)
    }
}

impl<T: cmp::PartialEq<T> + Sync + Send + 'static> cmp::PartialEq<Signal<T>> for Signal<T> {
    fn eq(&self, other: &Signal<T>) -> bool {
        self.inner.read().value == other.inner.read().value
    }
}

impl<T: cmp::PartialOrd<T> + Sync + Send + 'static> cmp::PartialOrd<Signal<T>> for Signal<T> {
    fn partial_cmp(&self, other: &Signal<T>) -> Option<cmp::Ordering> {
        self.inner
            .read()
            .value
            .partial_cmp(&other.inner.read().value)
    }
}

impl<T: cmp::Eq + Sync + Send + 'static> cmp::Eq for Signal<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::noop_waker;
    use std::pin::Pin;

    #[test]
    fn test_signal() {
        let mut owner = SignalOwner::new();
        assert_eq!(
            Pin::new(&mut owner).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Pending
        );

        let mut signal = owner.new_signal(42);
        assert_eq!(signal.get(), 42);

        signal.set(43);
        assert_eq!(signal, 43);
        assert_eq!(
            Pin::new(&mut owner).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Ready(())
        );
        assert_eq!(
            Pin::new(&mut owner).poll_change(&mut Context::from_waker(&noop_waker())),
            Poll::Pending
        );

        assert_eq!(signal.to_string(), "43");

        assert_eq!(signal.clone() + 1, 44);
        signal += 1;
        assert_eq!(signal, 44);

        assert_eq!(signal.clone() - 1, 43);
        signal -= 1;
        assert_eq!(signal, 43);

        assert_eq!(signal.clone() * 2, 86);
        signal *= 2;
        assert_eq!(signal, 86);

        assert_eq!(signal.clone() / 2, 43);
        signal /= 2;
        assert_eq!(signal, 43);

        assert!(signal > 42);
        assert!(signal >= 43);
        assert!(signal < 44);
        assert!(signal <= owner.new_signal(100));
    }
}
