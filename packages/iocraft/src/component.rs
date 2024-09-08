use crate::{
    element::{ElementKey, ElementType},
    render::{ComponentRenderer, ComponentUpdater, LayoutEngine},
};
use futures::future::poll_fn;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
};
pub use taffy::NodeId;

#[derive(Clone, Default)]
pub struct NoProps;

pub(crate) struct ComponentProps<C: Component>(pub(crate) C::Props);

pub(crate) trait AnyComponentProps: Any + Send {
    fn new_component(&self) -> Box<dyn AnyComponent>;
    fn update_component(
        &self,
        component: &mut Box<dyn AnyComponent>,
        updater: &mut ComponentUpdater<'_>,
    );
    fn clone_impl(&self) -> Box<dyn AnyComponentProps>;
    fn component_type_id(&self) -> TypeId;
}

impl<C: Component> AnyComponentProps for ComponentProps<C>
where
    C::Props: Clone + Send,
{
    fn new_component(&self) -> Box<dyn AnyComponent> {
        Box::new(C::new(&self.0))
    }

    fn update_component(
        &self,
        component: &mut Box<dyn AnyComponent>,
        updater: &mut ComponentUpdater<'_>,
    ) {
        component.update(&self.0, updater);
    }

    fn clone_impl(&self) -> Box<dyn AnyComponentProps> {
        Box::new(Self(self.0.clone()))
    }

    fn component_type_id(&self) -> TypeId {
        TypeId::of::<C>()
    }
}

impl Clone for Box<dyn AnyComponentProps> {
    fn clone(&self) -> Self {
        self.clone_impl()
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

pub(crate) trait AnyComponent: Any + Unpin + Send {
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
    props: Box<dyn AnyComponentProps>,
    children: Components,
}

impl InstantiatedComponent {
    pub fn new(node_id: NodeId, props: Box<dyn AnyComponentProps>) -> Self {
        Self {
            node_id,
            component: props.new_component(),
            props,
            children: Components::default(),
        }
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn component(&self) -> &dyn AnyComponent {
        &*self.component
    }

    pub fn set_props(&mut self, props: Box<dyn AnyComponentProps>) {
        self.props = props;
    }

    pub fn update(
        &mut self,
        layout_engine: &mut LayoutEngine,
        context_provider: &ComponentContextProvider<'_>,
    ) {
        let mut updater = ComponentUpdater::new(
            self.node_id,
            &mut self.children,
            layout_engine,
            context_provider,
        );
        self.props
            .update_component(&mut self.component, &mut updater);
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
