use std::future::Future;

pub trait Component {
    type Props;
    type State;

    fn new(props: Self::Props) -> Self;
    fn update(&mut self, props: Self::Props);
    fn render(&mut self);
    fn wait(&mut self) -> impl Future<Output = ()>;
}

pub struct Components<C> {
    components: Vec<C>,
}

impl<C> Default for Components<C> {
    fn default() -> Self {
        Self {
            components: Vec::new(),
        }
    }
}

impl<C: Component> Components<C> {
    pub fn renderer(&mut self) -> ComponentsRenderer<'_, C> {
        ComponentsRenderer {
            components: self,
            next_index: 0,
        }
    }
}

pub struct ComponentsRenderer<'a, C> {
    components: &'a mut Components<C>,
    next_index: usize,
}

impl<'a, C: Component> ComponentsRenderer<'a, C> {
    pub fn render(&mut self, props: C::Props) {
        if self.components.components.len() > self.next_index {
            self.components.components[self.next_index].update(props);
        } else {
            let component = C::new(props);
            self.components.components.push(component);
        }
        self.next_index += 1;
        self.components.components[self.next_index - 1].render();
    }
}

impl<'a, C> Drop for ComponentsRenderer<'a, C> {
    fn drop(&mut self) {
        self.components.components.truncate(self.next_index);
    }
}

pub async fn render<C: Component>(props: C::Props) {
    let mut component = C::new(props);
    loop {
        component.render();
        component.wait().await;
    }
}
