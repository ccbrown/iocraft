use iocraft::{
    prelude::*,
    taffy::{
        geometry::Line,
        prelude::{fr, span},
    },
};

fn main() {
    element! {
        View(
            display: Display::Grid,
            width: 40,
            padding: 1,
            gap: 1,
            border_style: BorderStyle::Round,
            grid_template_columns: vec![fr(1.0_f32), fr(2.0_f32)],
        ) {
            View(grid_column: span::<Line<_>>(2)) {
                Text(content: "Grid layout", weight: Weight::Bold)
            }
            View(grid_row: span::<Line<_>>(2), border_style: BorderStyle::Single) {
                Text(content: "Sidebar")
            }
            View(border_style: BorderStyle::Single) {
                Text(content: "First row")
            }
            View(border_style: BorderStyle::Single) {
                Text(content: "Second row")
            }
        }
    }
    .print();
}
