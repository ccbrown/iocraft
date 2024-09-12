use std::cell::RefCell;

#[derive(Default)]
struct SystemContextInner {
    should_exit: bool,
}

pub struct SystemContext {
    inner: RefCell<SystemContextInner>,
}

impl SystemContext {
    pub(crate) fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn exit(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.should_exit = true;
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.inner.borrow().should_exit
    }
}
