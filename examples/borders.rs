use flashy_io::prelude::*;

fn main() {
    flashy! {
        Box {
            Text(value: "hi!")
        }
    }
    .print();
}
