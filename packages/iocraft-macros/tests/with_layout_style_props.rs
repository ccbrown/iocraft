use iocraft::Display;
use iocraft_macros::{props, with_layout_style_props};

#[with_layout_style_props]
#[props]
struct MyProps {
    foo: String,
}

#[test]
fn layout_style_props() {
    let props: MyProps = Default::default();
    assert_eq!(props.foo, "");
    assert_eq!(props.display, Display::DEFAULT);
}
