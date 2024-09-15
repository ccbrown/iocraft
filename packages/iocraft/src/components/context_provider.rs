use crate::{AnyElement, Component, ComponentUpdater, Context, Covariant};

/// The props which can be passed to the [`ContextProvider`] component.
#[derive(Covariant, Default)]
pub struct ContextProviderProps<'a> {
    /// The children of the component.
    pub children: Vec<AnyElement<'a>>,

    /// The context to provide to the children.
    pub value: Option<Context<'a>>,
}

/// `ContextProvider` is a component that provides a context to its children.
#[derive(Default)]
pub struct ContextProvider;

impl Component for ContextProvider {
    type Props<'a> = ContextProviderProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }

    fn update(&mut self, props: &mut Self::Props<'_>, updater: &mut ComponentUpdater) {
        updater.update_children(
            props.children.iter_mut(),
            props.value.as_mut().map(|cx| cx.borrow()),
        );
    }
}
