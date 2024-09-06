use iocraft_macros::with_layout_style_props;
use taffy::{style, Point, Rect, Style};

// Re-export basic enum types.
pub use crossterm::style::Color;
pub use taffy::style::{Display, FlexDirection, Overflow};

#[with_layout_style_props]
pub struct LayoutStyle {
    // fields added by proc macro, defined in ../macros/src/lib.rs
}

impl From<LayoutStyle> for Style {
    fn from(s: LayoutStyle) -> Self {
        Self {
            display: s.display,
            padding: Rect {
                left: style::LengthPercentage::Length(
                    s.padding_left.or(s.padding).unwrap_or(0) as _
                ),
                right: style::LengthPercentage::Length(
                    s.padding_right.or(s.padding).unwrap_or(0) as _
                ),
                top: style::LengthPercentage::Length(s.padding_top.or(s.padding).unwrap_or(0) as _),
                bottom: style::LengthPercentage::Length(
                    s.padding_bottom.or(s.padding).unwrap_or(0) as _,
                ),
            },
            margin: Rect {
                left: style::LengthPercentageAuto::Length(
                    s.margin_left.or(s.margin).unwrap_or(0) as _
                ),
                right: style::LengthPercentageAuto::Length(
                    s.margin_right.or(s.margin).unwrap_or(0) as _,
                ),
                top: style::LengthPercentageAuto::Length(
                    s.margin_top.or(s.margin).unwrap_or(0) as _
                ),
                bottom: style::LengthPercentageAuto::Length(
                    s.margin_bottom.or(s.margin).unwrap_or(0) as _,
                ),
            },
            overflow: Point {
                x: s.overflow_x.or(s.overflow).unwrap_or_default(),
                y: s.overflow_y.or(s.overflow).unwrap_or_default(),
            },
            flex_direction: s.flex_direction,
            ..Default::default()
        }
    }
}
