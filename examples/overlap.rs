use iocraft::prelude::*;

const LOREM_IPSUM: &str = "Lorem ipsum odor amet, consectetuer adipiscing elit. \
Lobortis hendrerit nec ipsum dapibus quam. Donec malesuada tincidunt elementum \
mollis vehicula quisque purus. Est volutpat integer, donec sagittis placerat \
fermentum phasellus ipsum sollicitudin. Tempus laoreet ad tempus aptent proin \
per donec lectus. Quisque auctor urna; phasellus urna tortor ligula. Class \
pharetra bibendum tristique, quisque consectetur placerat potenti. Imperdiet ut \
torquent vestibulum eleifend bibendum et. Dictumst vulputate interdum iaculis \
at conubia venenatis.";

fn main() {
    element! {
        View(
            border_style: BorderStyle::DoubleLeftRight,
            border_color: Color::Green,
            margin: 1,
            width: 78,
            flex_direction: FlexDirection::Column,
        ) {
            View(margin_top: -1) {
                Text(content: " Overlap Example ", wrap: TextWrap::NoWrap)
            }
            View(padding: 1) {
                Text(content: format!("{} {}", LOREM_IPSUM, LOREM_IPSUM), color: Color::DarkGrey, weight: Weight::Light)
            }
            View(
                border_color: Color::Red,
                border_style: BorderStyle::DoubleTopBottom,
                padding: 1,
                position: Position::Absolute,
                top: 2,
                left: 4,
            ) {
                Text(content: "This element is overlapping the text!")
            }
            View(
                background_color: Color::Reset,
                border_color: Color::Red,
                border_style: BorderStyle::DoubleTopBottom,
                padding: 1,
                position: Position::Absolute,
                top: 8,
                left: 4,
            ) {
                Text(content: "We can cover it up by setting a background color.")
            }
        }
    }
    .print();
}
