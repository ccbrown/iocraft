use iocraft::prelude::*;

fn main() {
    element! {
        View(flex_direction: FlexDirection::Column, padding: 2) {
            View {
                View(border_style: BorderStyle::Single, margin_right: 2) {
                    Text(content: "Single")
                }
                View(border_style: BorderStyle::Double, margin_right: 2) {
                    Text(content: "Double")
                }
                View(border_style: BorderStyle::Round, margin_right: 2) {
                    Text(content: "Round")
                }
                View(border_style: BorderStyle::Bold) {
                    Text(content: "Bold")
                }
            }

            View(margin_top: 1) {
                View(border_style: BorderStyle::DoubleLeftRight, margin_right: 2) {
                    Text(content: "DoubleLeftRight")
                }
                View(border_style: BorderStyle::DoubleTopBottom, margin_right: 2) {
                    Text(content: "DoubleTopBottom")
                }
                View(border_style: BorderStyle::Classic) {
                    Text(content: "Classic")
                }
            }
        }
    }
    .print();
}
