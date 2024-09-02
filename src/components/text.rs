use crate::Component;

pub struct Text {}

impl Component for Text {
    type Props = ();
    type State = ();

    fn new(props: Self::Props) -> Self {
        Self {}
    }

    fn render(&self) {
        // TODO
    }

    async fn wait(&mut self) {
        std::future::pending::<()>().await;
    }
}
