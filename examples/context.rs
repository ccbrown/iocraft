use iocraft::prelude::*;

struct NumberOfTheDay(i32);

#[context]
struct MyContextConsumerContext<'a> {
    number: &'a NumberOfTheDay,
}

#[component]
fn MyContextConsumer(context: MyContextConsumerContext) -> impl Into<AnyElement<'static>> {
    element! {
        Box(border_style: BorderStyle::Round, border_color: Color::Cyan) {
            Text(content: "The number of the day is... ")
            Text(color: Color::Green, weight: Weight::Bold, content: context.number.0.to_string())
            Text(content: "!")
        }
    }
}

fn main() {
    element! {
        ContextProvider(value: Context::owned(NumberOfTheDay(42))) {
            MyContextConsumer
        }
    }
    .print();
}
