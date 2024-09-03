use flashy_cli::prelude::*;

fn main() {
    flashy! {
        Box {
            Text(value: "hi!")
        }
    }
    .print();
}
