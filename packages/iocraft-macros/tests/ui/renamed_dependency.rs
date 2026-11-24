use uiterm::prelude::*;

#[uiterm::with_layout_style_props]
#[derive(Props)]
struct MessageProps {
    #[iocraft(required)]
    value: String,
}

#[component]
fn Message(props: &MessageProps) -> impl Into<AnyElement<'static>> {
    element!(Text(content: props.value.clone()))
}

#[component]
fn Screen() -> impl Into<AnyElement<'static>> {
    element! {
        View(width: 100pct) {
            Message(value: "renamed dependency")
        }
    }
}

fn main() {
    element!(Screen).print();
}
