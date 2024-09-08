use iocraft::{element, AnyElement, Component, ComponentUpdater, Element};

#[derive(Clone, Default)]
struct MyComponent;

#[derive(Clone, Default)]
struct MyComponentProps {
    foo: String,
    children: Vec<Element<MyComponent>>,
}

impl Component for MyComponent {
    type Props = MyComponentProps;

    fn new(_props: &Self::Props) -> Self {
        Self
    }

    fn update(&mut self, _props: &Self::Props, _updater: &mut ComponentUpdater<'_>) {}
}

struct MyContainer;

#[derive(Clone, Default)]
struct MyContainerProps {
    children: Vec<AnyElement>,
}

impl Component for MyContainer {
    type Props = MyContainerProps;

    fn new(_props: &Self::Props) -> Self {
        Self
    }

    fn update(&mut self, _props: &Self::Props, _updater: &mut ComponentUpdater<'_>) {}
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
