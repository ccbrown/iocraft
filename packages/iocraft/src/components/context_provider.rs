use crate::{AnyElement, Component, ComponentUpdater, Context, Covariant};

#[derive(Covariant, Default)]
pub struct ContextProviderProps<'a> {
    pub children: Vec<AnyElement<'a>>,
    pub value: Option<Context<'a>>,
}

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
