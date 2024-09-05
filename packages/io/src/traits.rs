use crate::render::{ComponentRenderer, ComponentUpdater};
use futures::future::{pending, BoxFuture, FutureExt};
use std::any::{Any, TypeId};

pub trait ComponentProps: Any + Send + Sized {
    type Component: Component<Props = Self>;
}

pub(crate) trait AnyComponentProps: Any + Send {
    fn into_new_component(self: Box<Self>) -> Box<dyn AnyComponent>;
    fn update_component(self: Box<Self>, component: &mut Box<dyn AnyComponent>);
    fn clone_impl(&self) -> Box<dyn AnyComponentProps>;
    fn component_type_id(&self) -> TypeId;
}

impl<P: ComponentProps + Clone> AnyComponentProps for P {
    fn into_new_component(self: Box<Self>) -> Box<dyn AnyComponent> {
        Box::new(P::Component::new(*self))
    }

    fn update_component(self: Box<Self>, component: &mut Box<dyn AnyComponent>) {
        component.set_props(self);
    }

    fn clone_impl(&self) -> Box<dyn AnyComponentProps> {
        Box::new(self.clone())
    }

    fn component_type_id(&self) -> TypeId {
        TypeId::of::<P::Component>()
    }
}

impl Clone for Box<dyn AnyComponentProps> {
    fn clone(&self) -> Self {
        self.clone_impl()
    }
}

pub trait Component: Any + Send {
    type Props: ComponentProps<Component = Self>;
    type State;

    fn new(props: Self::Props) -> Self;
    fn set_props(&mut self, props: Self::Props);
    fn update(&mut self, updater: ComponentUpdater<'_>);
    fn render(&self, renderer: &mut ComponentRenderer<'_>);

    fn wait(&mut self) -> BoxFuture<()> {
        pending().boxed()
    }
}

pub(crate) trait AnyComponent: Any + Send {
    fn set_props(&mut self, props: Box<dyn Any>);
    fn update(&mut self, updater: ComponentUpdater<'_>);
    fn render(&self, renderer: &mut ComponentRenderer<'_>);
    fn wait(&mut self) -> BoxFuture<()>;
}

impl<C: Any + Component> AnyComponent for C {
    fn set_props(&mut self, props: Box<dyn Any>) {
        Component::set_props(
            self,
            *props.downcast().expect("we should be able to downcast"),
        );
    }

    fn update(&mut self, updater: ComponentUpdater<'_>) {
        Component::update(self, updater);
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        Component::render(self, renderer);
    }

    fn wait(&mut self) -> BoxFuture<()> {
        Component::wait(self)
    }
}
