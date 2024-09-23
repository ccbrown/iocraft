use iocraft::prelude::*;

fn main() {
    element! {
        Box(
            border_style: BorderStyle::Round,
            border_color: Color::Blue,
        ) {
            Text(content: "Hello, world!")
        }
    }
    .print();
}