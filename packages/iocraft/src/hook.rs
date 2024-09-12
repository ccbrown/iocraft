use crate::ComponentUpdater;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub trait Hook: Default {
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()>;

    fn pre_component_update(&mut self, _updater: &mut ComponentUpdater) {}
    fn post_component_update(&mut self, _updater: &mut ComponentUpdater) {}
}
