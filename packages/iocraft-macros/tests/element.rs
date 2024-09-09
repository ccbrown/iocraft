use iocraft::{element, AnyElement, Component, Covariant, Element};

#[derive(Default)]
struct MyComponent;

#[derive(Covariant, Default)]
struct MyComponentProps {
    foo: String,
    children: Vec<Element<'static, MyComponent>>,
}

impl Component for MyComponent {
    type Props<'a> = MyComponentProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }
}

struct MyContainer;

#[derive(Covariant, Default)]
struct MyContainerProps {
    children: Vec<AnyElement<'static>>,
}

impl Component for MyContainer {
    type Props<'a> = MyContainerProps;

    fn new(_props: &Self::Props<'_>) -> Self {
        Self
    }
}

#[test]
fn minimal() {
    let _: Element<MyComponent> = element!(MyComponent);
}

#[test]
fn full_qualified_type() {
    pub mod foo {
        pub mod bar {
            pub type MyComponent = crate::MyComponent;
        }
    }
    let _: Element<MyComponent> = element!(foo::bar::MyComponent);
    let _: Element<::iocraft::Box> = element!(::iocraft::Box);
}

#[test]
fn props() {
    let e = element! {
        MyComponent(foo: "bar")
    };
    assert_eq!(e.props.foo, "bar");
}

#[test]
fn children() {
    let e = element! {
        MyComponent {
            MyComponent(foo: "bar")
        }
    };
    assert_eq!(e.props.children.len(), 1);
    assert_eq!(e.props.children[0].props.foo, "bar");
}

#[test]
fn any_children() {
    let e = element! {
        MyContainer {
            MyContainer
            MyComponent(foo: "bar")
        }
    };
    assert_eq!(e.props.children.len(), 2);
}
