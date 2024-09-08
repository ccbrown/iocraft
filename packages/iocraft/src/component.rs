use crate::{
    element::{ElementKey, ElementType},
    render::{ComponentRenderer, ComponentUpdater, LayoutEngine},
};
use futures::future::poll_fn;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
pub use taffy::NodeId;

#[derive(Clone, Copy, Default)]
pub struct NoProps;

pub(crate) struct ComponentHelper<C: Component> {
    _marker: PhantomData<C>,
}

impl<C: Component> ComponentHelper<C> {
    pub fn boxed() -> Box<dyn ComponentHelperExt> {
        Box::new(Self {
            _marker: PhantomData,
        })
    }
}

#[doc(hidden)]
pub trait ComponentHelperExt: Any + Send {
    fn new_component(&self, props: &dyn Any) -> Box<dyn AnyComponent>;
    fn update_component(
        &self,
        component: &mut Box<dyn AnyComponent>,
        props: &dyn Any,
        updater: &mut ComponentUpdater<'_>,
    );
    fn component_type_id(&self) -> TypeId;
    fn copy(&self) -> Box<dyn ComponentHelperExt>;
}

impl<C: Component> ComponentHelperExt for ComponentHelper<C> {
    fn new_component(&self, props: &dyn Any) -> Box<dyn AnyComponent> {
        Box::new(C::new(
            props.downcast_ref().expect("we should be able to downcast"),
        ))
    }

    fn update_component(
        &self,
        component: &mut Box<dyn AnyComponent>,
        props: &dyn Any,
        updater: &mut ComponentUpdater<'_>,
    ) {
        component.update(props, updater);
    }

    fn component_type_id(&self) -> TypeId {
        TypeId::of::<C>()
    }

    fn copy(&self) -> Box<dyn ComponentHelperExt> {
        Self::boxed()
    }
}

pub trait Component: Any + Unpin + Send {
    type Props;

    fn new(props: &Self::Props) -> Self;

    fn update(&mut self, _props: &Self::Props, _updater: &mut ComponentUpdater<'_>) {}
    fn render(&self, _renderer: &mut ComponentRenderer<'_>) {}

    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Pending
    }
}

impl<C: Component> ElementType for C {
    type Props = C::Props;
}

#[doc(hidden)]
pub trait AnyComponent: Any + Unpin + Send {
    fn update(&mut self, props: &dyn Any, updater: &mut ComponentUpdater<'_>);
    fn render(&self, renderer: &mut ComponentRenderer<'_>);
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()>;
}

impl<C: Any + Component> AnyComponent for C {
    fn update(&mut self, props: &dyn Any, updater: &mut ComponentUpdater<'_>) {
        Component::update(
            self,
            props.downcast_ref().expect("we should be able to downcast"),
            updater,
        );
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        Component::render(self, renderer);
    }

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        Component::poll_change(self, cx)
    }
}

#[derive(Clone, Default)]
pub(crate) enum ComponentContextProvider<'a> {
    #[default]
    Root,
    Child {
        parent: &'a ComponentContextProvider<'a>,
        context: Box<&'a dyn Any>,
    },
}

impl<'a> ComponentContextProvider<'a> {
    pub fn with_context(&'a self, context: Box<&'a dyn Any>) -> Self {
        Self::Child {
            parent: self,
            context,
        }
    }

    pub fn get_context<T: Any>(&self) -> Option<&T> {
        match self {
            Self::Root => None,
            Self::Child { parent, context } => {
                if let Some(context) = context.downcast_ref::<T>() {
                    Some(context)
                } else {
                    parent.get_context()
                }
            }
        }
    }
}

pub(crate) struct InstantiatedComponent {
    node_id: NodeId,
    component: Box<dyn AnyComponent>,
    children: Components,
    helper: Box<dyn ComponentHelperExt>,
}

impl InstantiatedComponent {
    pub fn new(
        node_id: NodeId,
        props: &(dyn Any + Send),
        helper: Box<dyn ComponentHelperExt>,
    ) -> Self {
        Self {
            node_id,
            component: helper.new_component(props),
            children: Components::default(),
            helper,
        }
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn component(&self) -> &dyn AnyComponent {
        &*self.component
    }

    pub fn update(
        &mut self,
        layout_engine: &mut LayoutEngine,
        context_provider: &ComponentContextProvider<'_>,
        props: &(dyn Any + Send),
    ) {
        let mut updater = ComponentUpdater::new(
            self.node_id,
            &mut self.children,
            layout_engine,
            context_provider,
        );
        self.helper
            .update_component(&mut self.component, props, &mut updater);
    }

    pub fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        self.component.render(renderer);
        self.children.render(renderer);
    }

    pub async fn wait(&mut self) {
        let mut self_mut = Pin::new(self);
        poll_fn(|cx| self_mut.as_mut().poll_change(cx)).await;
    }

    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let component_status = Pin::new(&mut *self.component).poll_change(cx);
        let children_status = Pin::new(&mut self.children).poll_change(cx);
        if component_status.is_ready() || children_status.is_ready() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub(crate) struct Components {
    pub components: HashMap<ElementKey, InstantiatedComponent>,
}

impl Components {
    pub fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        for (_, component) in self.components.iter() {
            renderer.for_child_node(component.node_id, |renderer| {
                component.render(renderer);
            });
        }
    }

    pub fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut is_ready = false;
        for component in self.components.values_mut() {
            if Pin::new(&mut *component).poll_change(cx).is_ready() {
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

impl Default for Components {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}
