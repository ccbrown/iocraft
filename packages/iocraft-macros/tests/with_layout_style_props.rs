use iocraft::Display;
use iocraft_macros::{with_layout_style_props, Props};

#[with_layout_style_props]
#[derive(Default, Props)]
struct MyProps {
    foo: String,
}

#[with_layout_style_props]
#[derive(Default, Props)]
struct MyPropsWithLifetime<'lt> {
    foo: Option<&'lt str>,
}

#[with_layout_style_props]
#[derive(Default, Props)]
struct MyPropsWithTypeGeneric<T: Send + Sync> {
    foo: Option<T>,
}

#[with_layout_style_props]
#[derive(Default, Props)]
struct MyPropsWithConstParam<const N: usize> {
    foo: Option<[u8; N]>,
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

    let props: MyPropsWithConstParam<1> = Default::default();
    assert_eq!(props.foo, None);
    assert_eq!(props.display, Display::DEFAULT);
}
