use crate::{AnyElement, Component, ComponentUpdater, Covariant};
use std::{any::Any, marker::PhantomData};

#[derive(Covariant, Default)]
pub struct ContextProviderProps<'a, T: Any + Unpin> {
    pub children: Vec<AnyElement<'a>>,
    pub value: Option<T>,
}

#[derive(Default)]
pub struct ContextProvider<T> {
    _marker: PhantomData<T>,
}

impl<T: Any + Unpin> Component for ContextProvider<T> {
    type Props<'a> = ContextProviderProps<'a, T>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    fn update(&mut self, props: &Self::Props<'_>, updater: &mut ComponentUpdater<'_>) {
        updater.update_children(
            props.children.iter(),
            props.value.as_ref().map(|v| Box::new(v as &dyn Any)),
        );
    }
}
