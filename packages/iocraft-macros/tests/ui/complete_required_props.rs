use iocraft::prelude::*;

mod props {
    use super::*;

    pub struct RequiredValue;

    #[derive(Props)]
    pub struct PublicProps {
        #[iocraft(required)]
        pub value: RequiredValue,
        pub optional: String,
    }
}

#[component]
fn Example(props: &props::PublicProps) -> impl Into<AnyElement<'static>> {
    let _ = &props.value;
    element!(Text(content: props.optional.clone()))
}

fn main() {
    // A required type need not implement Default. Public setters must remain
    // usable across module boundaries even though the builder fields are private.
    let _ = props::PublicProps::__iocraft_builder()
        .value(props::RequiredValue)
        .__iocraft_build();
    let _ = element!(Example(value: props::RequiredValue));
}
