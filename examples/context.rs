use iocraft::prelude::*;

#[derive(Default)]
struct NumberOfTheDay(i32);

#[context]
struct MyContextConsumerContext<'a> {
    number: Option<&'a NumberOfTheDay>,
}

#[component]
fn MyContextConsumer(context: MyContextConsumerContext) -> impl Into<AnyElement<'static>> {
    element! {
        Box(border_style: BorderStyle::Round, border_color: Color::Cyan) {
            Text(content: "The number of the day is... ")
            Text(color: Color::Green, weight: Weight::Bold, content: context.number.unwrap().0.to_string())
            Text(content: "!")
        }
    }
}

fn main() {
    element! {
        ContextProvider::<NumberOfTheDay>(value: NumberOfTheDay(42)) {
            MyContextConsumer
        }
    }
    .print();
}
