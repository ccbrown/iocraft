use iocraft::Props;

mod props {
    use super::Props;

    #[derive(Props)]
    pub struct PublicProps {
        #[iocraft(required)]
        pub value: String,
    }
}

fn main() {
    let builder = props::PublicProps::__iocraft_builder();
    let _ = builder.value;
}
