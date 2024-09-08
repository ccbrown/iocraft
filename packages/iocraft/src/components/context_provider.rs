use crate::{AnyElement, Component, ComponentUpdater};
use std::{any::Any, marker::PhantomData};

#[derive(Default)]
pub struct ContextProviderProps<T> {
    pub children: Vec<AnyElement>,
    pub value: Option<T>,
}

#[derive(Default)]
pub struct ContextProvider<T> {
    _marker: PhantomData<T>,
}

impl<T: Unpin + Send + 'static> Component for ContextProvider<T> {
    type Props = ContextProviderProps<T>;

    fn new(_props: &Self::Props) -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    fn update(&mut self, props: &Self::Props, updater: &mut ComponentUpdater<'_>) {
        updater.update_children(
            props.children.iter(),
            props.value.as_ref().map(|v| Box::new(v as &dyn Any)),
        );
    }
}
