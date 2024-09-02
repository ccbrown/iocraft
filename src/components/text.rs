use crate::Component;

pub struct TextProps {
    pub value: String,
}

pub struct Text {
    props: TextProps,
}

impl Component for Text {
    type Props = TextProps;
    type State = ();

    fn new(props: Self::Props) -> Self {
        Self { props }
    }

    fn update(&mut self, props: Self::Props) {
        self.props = props;
    }

    fn render(&mut self) {
        println!("{}", self.props.value);
    }

    async fn wait(&mut self) {
        std::future::pending::<()>().await;
    }
}
