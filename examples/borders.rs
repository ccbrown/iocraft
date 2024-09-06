use iocraft::prelude::*;

fn main() {
    element! {
        Box(flex_direction: FlexDirection::Column, padding: 2) {
            Box {
                Box(border_style: BorderStyle::Single, margin_right: 2) {
                    Text(content: "Single")
                }
                Box(border_style: BorderStyle::Double, margin_right: 2) {
                    Text(content: "Double")
                }
                Box(border_style: BorderStyle::Round, margin_right: 2) {
                    Text(content: "Round")
                }
                Box(border_style: BorderStyle::Bold) {
                    Text(content: "Bold")
                }
            }

            Box(margin_top: 1) {
                Box(border_style: BorderStyle::DoubleLeftRight, margin_right: 2) {
                    Text(content: "DoubleLeftRight")
                }
                Box(border_style: BorderStyle::DoubleTopBottom, margin_right: 2) {
                    Text(content: "DoubleTopBottom")
                }
                Box(border_style: BorderStyle::Classic) {
                    Text(content: "Classic")
                }
            }
        }
    }
    .print();
}
