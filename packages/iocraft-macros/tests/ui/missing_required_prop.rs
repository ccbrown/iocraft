use iocraft::Props;

#[derive(Props)]
struct ExampleProps {
    #[iocraft(required)]
    important_value: String,
}

fn main() {
    let _ = ExampleProps::__iocraft_builder().__iocraft_build();
}
