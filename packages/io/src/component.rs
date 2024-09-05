use crate::{
    element::{ElementKey, ElementType},
    render::{ComponentRenderer, ComponentUpdater, LayoutEngine},
};
use futures::future::{pending, select, select_all, BoxFuture, FutureExt};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
};
pub use taffy::NodeId;

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

pub trait Component: Any + Send {
    type Props;

    fn new(props: &Self::Props) -> Self;
    fn update(&mut self, props: &Self::Props, updater: &mut ComponentUpdater<'_>);
    fn render(&self, _renderer: &mut ComponentRenderer<'_>) {}

    fn wait(&mut self) -> impl Future<Output = ()> + Send {
        pending()
    }
}

impl<C: Component> ElementType for C {
    type Props = C::Props;
}

pub(crate) trait AnyComponent: Any + Send {
    fn update(&mut self, props: &dyn Any, updater: &mut ComponentUpdater<'_>);
    fn render(&self, renderer: &mut ComponentRenderer<'_>);
    fn wait(&mut self) -> BoxFuture<()>;
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

    fn wait(&mut self) -> BoxFuture<()> {
        Component::wait(self).boxed()
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

    pub fn update(&mut self, layout_engine: &mut LayoutEngine) {
        let mut updater = ComponentUpdater::new(self.node_id, &mut self.children, layout_engine);
        self.props
            .update_component(&mut self.component, &mut updater);
    }

    pub fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        self.component.render(renderer);
        self.children.render(renderer);
    }

    pub async fn wait(&mut self) {
        select(self.component.wait(), self.children.wait().boxed()).await;
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

    pub async fn wait(&mut self) {
        select_all(
            self.components
                .iter_mut()
                .map(|(_, component)| component.component.wait()),
        )
        .await;
    }
}

impl Default for Components {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}
