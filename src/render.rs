use std::future::Future;

pub trait Component {
    type Props;
    type State;

    fn new(props: Self::Props) -> Self;
    fn render(&self);
    fn wait(&mut self) -> impl Future<Output = ()>;
}

pub async fn render<C: Component>(props: C::Props) {
    let mut component = C::new(props);
    loop {
        component.render();
        component.wait().await;
    }
}
