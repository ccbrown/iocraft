use crate::{
    component::{AnyComponentProps, ComponentProps},
    render::Tree,
    Element, ElementKey,
};
use flashy_element::ElementType;
use std::future::Future;

pub trait ElementExt {
    fn print(self);
    fn render(self) -> impl Future<Output = ()>;
}

#[derive(Clone)]
pub struct AnyElement {
    key: ElementKey,
    props: Box<dyn AnyComponentProps>,
}

impl AnyElement {
    pub(crate) fn into_key_and_props(self) -> (ElementKey, Box<dyn AnyComponentProps>) {
        (self.key, self.props)
    }
}

impl<T> From<Element<T>> for AnyElement
where
    T: ElementType + 'static,
    <T as ElementType>::Props: ComponentProps + Clone,
{
    fn from(e: Element<T>) -> Self {
        Self {
            key: e.key,
            props: Box::new(e.props),
        }
    }
}

impl<T: Into<AnyElement>> ElementExt for T {
    fn print(self) {
        let mut tree = Tree::new(self.into());
        tree.render();
        println!("");
    }

    async fn render(self) {
        let mut tree = Tree::new(self.into());
        tree.render_loop().await;
    }
}
