use iocraft::Display;
use iocraft_macros::{props, with_layout_style_props};

#[with_layout_style_props]
#[props]
struct MyProps {
    foo: String,
}

#[with_layout_style_props]
#[props]
struct MyPropsWithLifetime<'lt> {
    foo: Option<&'lt str>,
}

#[with_layout_style_props]
#[props]
struct MyPropsWithTypeGeneric<T> {
    foo: Option<T>,
}

#[test]
fn layout_style_props() {
    let props: MyProps = Default::default();
    assert_eq!(props.foo, "");
    assert_eq!(props.display, Display::DEFAULT);

    let props: MyPropsWithLifetime<'static> = Default::default();
    assert_eq!(props.foo, None);
    assert_eq!(props.display, Display::DEFAULT);

    let props: MyPropsWithTypeGeneric<String> = Default::default();
    assert_eq!(props.foo, None);
    assert_eq!(props.display, Display::DEFAULT);
}
