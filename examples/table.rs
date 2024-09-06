use iocraft::prelude::*;

fn main() {
    element! {
        Box(flex_direction: FlexDirection::Column, width: 80) {
            Box {
                Box(width: 10pct) {
                    Text(content: "Id")
                }

                Box(width: 50pct) {
                    Text(content: "Name")
                }

                Box(width: 40pct) {
                    Text(content: "Email")
                }
            }
        }
    }
    .print();
}
