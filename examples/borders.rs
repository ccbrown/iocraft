use flashy_io::prelude::*;

fn main() {
    flashy! {
        Box(flex_direction: FlexDirection::Column, padding: 2) {
            Box {
                Box(border_style: BorderStyle::Single, margin_right: 2) {
                    Text(content: "single")
                }
                Box(border_style: BorderStyle::Double, margin_right: 2) {
                    Text(content: "double")
                }
                Box(border_style: BorderStyle::Round, margin_right: 2) {
                    Text(content: "round")
                }
                Box(border_style: BorderStyle::Bold) {
                    Text(content: "bold")
                }
            }

            Box(margin_top: 1) {
                Box(border_style: BorderStyle::SingleDouble, margin_right: 2) {
                    Text(content: "single-double")
                }
                Box(border_style: BorderStyle::DoubleSingle, margin_right: 2) {
                    Text(content: "double-single")
                }
                Box(border_style: BorderStyle::Classic) {
                    Text(content: "classic")
                }
            }
        }
    }
    .print();
}
