use iocraft::prelude::*;

#[component]
fn Broken<'a>(_hooks: Hooks<'a, '_>) -> impl Into<AnyElement<'a>> {
    element!(Text(content: "hooks"))
}

fn main() {}
