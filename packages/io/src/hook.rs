use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub trait Hook: Default {
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()>;
}
