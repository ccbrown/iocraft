use crate::{
    context::ContextStack,
    element::{ElementKey, ElementType},
    hook::{AnyHook, Hook, Hooks},
    props::{AnyProps, Props},
    render::{ComponentDrawer, ComponentUpdater, UpdateContext},
};
use futures::future::poll_fn;
use indexmap::IndexMap;
use std::{
    any::{Any, TypeId},
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
        hooks: Hooks,
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
        hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        component.update(props, hooks, updater);
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
/// level component type definitions. Instead, the [`component`](macro@crate::component) macro should be used.
pub trait Component: Any + Unpin {
    /// The type of properties that the component accepts.
    type Props<'a>: Props
    where
        Self: 'a;

    /// Creates a new instance of the component from a set of properties.
    fn new(props: &Self::Props<'_>) -> Self;

    /// Invoked whenever the properties of the component or layout may have changed.
    fn update(
        &mut self,
        _props: &mut Self::Props<'_>,
        _hooks: Hooks,
        _updater: &mut ComponentUpdater,
    ) {
    }

    /// Invoked to draw the component.
    fn draw(&mut self, _drawer: &mut ComponentDrawer<'_>) {}

    /// Invoked to determine whether a change has occurred that would require the component to be
    /// updated and redrawn.
    fn poll_change(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        Poll::Pending
    }
}

impl<C: Component> ElementType for C {
    type Props<'a> = C::Props<'a>;
}

#[doc(hidden)]
pub trait AnyComponent: Any + Unpin {
    fn update(&mut self, props: AnyProps, hooks: Hooks, updater: &mut ComponentUpdater);
    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>);
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()>;
}

impl<C: Any + Component> AnyComponent for C {
    fn update(&mut self, mut props: AnyProps, hooks: Hooks, updater: &mut ComponentUpdater) {
        Component::update(
            self,
            unsafe { props.downcast_mut_unchecked() },
            hooks,
            updater,
        );
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        Component::draw(self, drawer);
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
    hooks: Vec<Box<dyn AnyHook>>,
    first_update: bool,
    has_transparent_layout: bool,
}

impl InstantiatedComponent {
    pub fn new(node_id: NodeId, props: AnyProps, helper: Box<dyn ComponentHelperExt>) -> Self {
        Self {
            node_id,
            component: helper.new_component(props),
            children: Components::default(),
            helper,
            hooks: Default::default(),
            first_update: true,
            has_transparent_layout: false,
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
        unattached_child_node_ids: Option<&mut Vec<NodeId>>,
        component_context_stack: &mut ContextStack<'_>,
        props: AnyProps,
    ) {
        let mut updater = ComponentUpdater::new(
            self.node_id,
            &mut self.children,
            unattached_child_node_ids,
            context,
            component_context_stack,
        );
        self.hooks.pre_component_update(&mut updater);
        self.helper.update_component(
            &mut self.component,
            props,
            Hooks::new(&mut self.hooks, self.first_update),
            &mut updater,
        );
        self.hooks.post_component_update(&mut updater);
        self.first_update = false;
        self.has_transparent_layout = updater.has_transparent_layout();
    }

    pub fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        if self.has_transparent_layout {
            // If the component has a transparent layout, provide the first child's layout to the
            // hooks and component.
            if let Some((_, child)) = self.children.components.iter().next().as_ref() {
                drawer.for_child_node_layout(child.node_id, |drawer| {
                    self.hooks.pre_component_draw(drawer);
                    self.component.draw(drawer);
                });
            } else {
                self.hooks.pre_component_draw(drawer);
                self.component.draw(drawer);
            }
        } else {
            self.hooks.pre_component_draw(drawer);
            self.component.draw(drawer);
        }

        self.children.draw(drawer);

        if self.has_transparent_layout {
            if let Some((_, child)) = self.children.components.iter().next().as_ref() {
                drawer.for_child_node_layout(child.node_id, |drawer| {
                    self.hooks.post_component_draw(drawer);
                });
            } else {
                self.hooks.post_component_draw(drawer);
            }
        } else {
            self.hooks.post_component_draw(drawer);
        }
    }

    pub async fn wait(&mut self) {
        let mut self_mut = Pin::new(self);
        poll_fn(|cx| self_mut.as_mut().poll_change(cx)).await;
    }

    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let component_status = Pin::new(&mut *self.component).poll_change(cx);
        let children_status = Pin::new(&mut self.children).poll_change(cx);
        let hooks_status = Pin::new(&mut self.hooks).poll_change(cx);
        if component_status.is_ready() || children_status.is_ready() || hooks_status.is_ready() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

#[derive(Default)]
pub(crate) struct Components {
    pub components: IndexMap<ElementKey, InstantiatedComponent>,
}

impl Components {
    pub fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        for (_, component) in self.components.iter_mut() {
            if component.has_transparent_layout {
                component.draw(drawer);
            } else {
                drawer.for_child_node_layout(component.node_id, |drawer| {
                    component.draw(drawer);
                });
            }
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
