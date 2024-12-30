use iocraft::prelude::*;

struct NumberOfTheDay(i32);

#[component]
fn MyContextConsumer(hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let number = hooks.use_context::<NumberOfTheDay>();

    element! {
        View(border_style: BorderStyle::Round, border_color: Color::Cyan) {
            Text(content: "The number of the day is... ")
            Text(color: Color::Green, weight: Weight::Bold, content: number.0.to_string())
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
