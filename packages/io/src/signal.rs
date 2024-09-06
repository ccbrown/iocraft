use futures_signals::signal::{Mutable, SignalExt};
use std::{
    cmp,
    fmt::{self, Debug, Display, Formatter},
    ops,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Default)]
pub struct SignalOwner {
    signals: Vec<Pin<Box<dyn futures_signals::signal::Signal<Item = ()> + Send>>>,
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
        let inner = Mutable::new(value);
        self.signals.push(inner.signal_ref(|_| {}).boxed());
        Signal { inner }
    }

    pub fn new_signal_with_default<T: Default + Send + Sync + 'static>(&mut self) -> Signal<T> {
        self.new_signal(T::default())
    }
}

#[derive(Clone)]
pub struct Signal<T> {
    inner: Mutable<T>,
}

impl<T: Debug> Debug for Signal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.lock_ref().fmt(f)
    }
}

impl<T: Display> Display for Signal<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.lock_ref().fmt(f)
    }
}

impl<T: ops::Add<Output = T> + Copy> ops::Add<T> for Signal<T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.inner.get() + rhs
    }
}

impl<T: ops::AddAssign<T> + Copy> ops::AddAssign<T> for Signal<T> {
    fn add_assign(&mut self, rhs: T) {
        let mut value = self.inner.lock_mut();
        *value += rhs;
    }
}

impl<T: ops::Sub<Output = T> + Copy> ops::Sub<T> for Signal<T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.inner.get() - rhs
    }
}

impl<T: ops::SubAssign<T> + Copy> ops::SubAssign<T> for Signal<T> {
    fn sub_assign(&mut self, rhs: T) {
        let mut value = self.inner.lock_mut();
        *value -= rhs;
    }
}

impl<T: ops::Mul<Output = T> + Copy> ops::Mul<T> for Signal<T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.inner.get() * rhs
    }
}

impl<T: ops::MulAssign<T> + Copy> ops::MulAssign<T> for Signal<T> {
    fn mul_assign(&mut self, rhs: T) {
        let mut value = self.inner.lock_mut();
        *value *= rhs;
    }
}

impl<T: ops::Div<Output = T> + Copy> ops::Div<T> for Signal<T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.inner.get() / rhs
    }
}

impl<T: ops::DivAssign<T> + Copy> ops::DivAssign<T> for Signal<T> {
    fn div_assign(&mut self, rhs: T) {
        let mut value = self.inner.lock_mut();
        *value /= rhs;
    }
}

impl<T: cmp::PartialEq<T>> cmp::PartialEq<T> for Signal<T> {
    fn eq(&self, other: &T) -> bool {
        *self.inner.lock_ref() == *other
    }
}

impl<T: cmp::PartialOrd<T>> cmp::PartialOrd<T> for Signal<T> {
    fn partial_cmp(&self, other: &T) -> Option<cmp::Ordering> {
        self.inner.lock_ref().partial_cmp(other)
    }
}

impl<T: cmp::PartialEq<T>> cmp::PartialEq<Signal<T>> for Signal<T> {
    fn eq(&self, other: &Signal<T>) -> bool {
        *self.inner.lock_ref() == *other.inner.lock_ref()
    }
}

impl<T: cmp::PartialOrd<T>> cmp::PartialOrd<Signal<T>> for Signal<T> {
    fn partial_cmp(&self, other: &Signal<T>) -> Option<cmp::Ordering> {
        self.inner.lock_ref().partial_cmp(&*other.inner.lock_ref())
    }
}

impl<T: cmp::Eq> cmp::Eq for Signal<T> {}
