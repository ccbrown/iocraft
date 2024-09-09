use crate::{AnyElement, Component, ComponentUpdater, Covariant};
use std::{any::Any, marker::PhantomData};

#[derive(Covariant, Default)]
pub struct ContextProviderProps<T: 'static> {
    pub children: Vec<AnyElement<'static>>,
    pub value: Option<T>,
}

#[derive(Default)]
pub struct ContextProvider<T> {
    _marker: PhantomData<T>,
}

impl<T: Unpin + 'static> Component for ContextProvider<T> {
    type Props<'a> = ContextProviderProps<T>;

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
