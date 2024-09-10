use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExitMode {
    ClearOutput,
    PreserveOutput,
}

#[derive(Default)]
struct SystemContextInner {
    exit_mode: Option<ExitMode>,
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

    pub fn exit(&self, mode: ExitMode) {
        let mut inner = self.inner.borrow_mut();
        inner.exit_mode = Some(mode);
    }

    pub(crate) fn exit_mode(&self) -> Option<ExitMode> {
        self.inner.borrow().exit_mode
    }
}
