use crate::{
    context::ContextStack,
    element::{ElementKey, ElementType},
    props::{AnyProps, Covariant},
    render::{ComponentRenderer, ComponentUpdater, UpdateContext},
};
use futures::future::poll_fn;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use taffy::NodeId;

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
pub trait ComponentHelperExt: Any {
    fn new_component(&self, props: AnyProps) -> Box<dyn AnyComponent>;
    fn update_component(
        &self,
        component: &mut Box<dyn AnyComponent>,
        props: AnyProps,
        updater: &mut ComponentUpdater,
    );
    fn component_type_id(&self) -> TypeId;
    fn copy(&self) -> Box<dyn ComponentHelperExt>;
}

impl<C: Component> ComponentHelperExt for ComponentHelper<C> {
    fn new_component(&self, props: AnyProps) -> Box<dyn AnyComponent> {
        Box::new(C::new(unsafe { props.downcast_ref_unchecked() }))
    }

    fn update_component(
        &self,
        component: &mut Box<dyn AnyComponent>,
        props: AnyProps,
        updater: &mut ComponentUpdater,
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

/// `Component` defines a component type and the methods required for instantiating and rendering
/// the component.
///
/// Most users will not need to implement this trait directly. This is only required for new, low
/// level component type definitions.
pub trait Component: Any + Unpin {
    /// The type of properties that the component accepts.
    type Props<'a>: Covariant
    where
        Self: 'a;

    /// Creates a new instance of the component from a set of properties.
    fn new(props: &Self::Props<'_>) -> Self;

    /// Invoked whenever the properties of the component or layout may have changed.
    fn update(&mut self, _props: &mut Self::Props<'_>, _updater: &mut ComponentUpdater) {}

    /// Invoked to render the component.
    fn render(&self, _renderer: &mut ComponentRenderer<'_>) {}

    /// Invoked to determine whether a change has occurred that would require the component to be
    /// updated and re-rendered.
    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Pending
    }
}

impl<C: Component> ElementType for C {
    type Props<'a> = C::Props<'a>;
}

#[doc(hidden)]
pub trait AnyComponent: Any + Unpin {
    fn update(&mut self, props: AnyProps, updater: &mut ComponentUpdater);
    fn render(&self, renderer: &mut ComponentRenderer<'_>);
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()>;
}

impl<C: Any + Component> AnyComponent for C {
    fn update(&mut self, mut props: AnyProps, updater: &mut ComponentUpdater) {
        Component::update(self, unsafe { props.downcast_mut_unchecked() }, updater);
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        Component::render(self, renderer);
    }

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        Component::poll_change(self, cx)
    }
}

pub(crate) struct InstantiatedComponent {
    node_id: NodeId,
    component: Box<dyn AnyComponent>,
    children: Components,
    helper: Box<dyn ComponentHelperExt>,
}

impl InstantiatedComponent {
    pub fn new(node_id: NodeId, props: AnyProps, helper: Box<dyn ComponentHelperExt>) -> Self {
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
        context: &mut UpdateContext<'_>,
        component_context_stack: &mut ContextStack<'_>,
        props: AnyProps,
    ) {
        let mut updater = ComponentUpdater::new(
            self.node_id,
            &mut self.children,
            context,
            component_context_stack,
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

#[derive(Default)]
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
