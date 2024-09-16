use bitflags::bitflags;
use iocraft_macros::with_layout_style_props;
use taffy::{
    geometry,
    style::{Dimension, LengthPercentage, LengthPercentageAuto},
    Rect, Style,
};

// Re-export basic enum types.
pub use crossterm::style::Color;
pub use taffy::style::{
    AlignContent, AlignItems, Display, FlexDirection, FlexWrap, JustifyContent, Overflow,
};

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Percent(pub f32);

/// Defines the area to reserve around the element's content, but inside the border.
///
/// See [the MDN documentation for padding](https://developer.mozilla.org/en-US/docs/Web/CSS/padding).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Padding {
    /// No padding.
    #[default]
    Unset,
    /// Sets an absolute value.
    Length(u32),
    /// Sets a percentage of the width or height of the parent.
    Percent(f32),
}

impl Padding {
    fn or(self, other: Self) -> Self {
        match self {
            Padding::Unset => other,
            _ => self,
        }
    }
}

impl From<Padding> for LengthPercentage {
    fn from(p: Padding) -> Self {
        match p {
            Padding::Unset => LengthPercentage::Length(0.0),
            Padding::Length(l) => LengthPercentage::Length(l as _),
            Padding::Percent(p) => LengthPercentage::Percent(p / 100.0),
        }
    }
}

macro_rules! impl_from_length {
    ($name:ident) => {
        impl From<u32> for $name {
            fn from(l: u32) -> Self {
                $name::Length(l)
            }
        }
    };
}

macro_rules! impl_from_percent {
    ($name:ident) => {
        impl From<Percent> for $name {
            fn from(p: Percent) -> Self {
                $name::Percent(p.0)
            }
        }
    };
}

impl_from_length!(Padding);
impl_from_percent!(Padding);

macro_rules! new_size_type {
    ($(#[$m:meta])* $name:ident, $def:expr) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub enum $name {
            /// The default behavior.
            #[default]
            Unset,
            /// Automatically selects a suitable size.
            Auto,
            /// Sets an absolute value.
            Length(u32),
            /// Sets a percentage of the width or height of the parent.
            Percent(f32),
        }

        impl $name {
            #[allow(dead_code)]
            fn or<T: Into<Self>>(self, other: T) -> Self {
                match self {
                    $name::Unset => other.into(),
                    _ => self,
                }
            }
        }

        impl From<$name> for LengthPercentageAuto {
            fn from(p: $name) -> Self {
                match p {
                    $name::Unset => $def.into(),
                    $name::Auto => LengthPercentageAuto::Auto,
                    $name::Length(l) => LengthPercentageAuto::Length(l as _),
                    $name::Percent(p) => LengthPercentageAuto::Percent(p / 100.0),
                }
            }
        }

        impl From<$name> for Dimension {
            fn from(p: $name) -> Self {
                match p {
                    $name::Unset => $def.into(),
                    $name::Auto => Dimension::Auto,
                    $name::Length(l) => Dimension::Length(l as _),
                    $name::Percent(p) => Dimension::Percent(p / 100.0),
                }
            }
        }

        impl_from_length!($name);
        impl_from_percent!($name);
    };
}

new_size_type!(
    /// Defines the area to reserve around the element's content, but outside the border.
    ///
    /// See [the MDN documentation for margin](https://developer.mozilla.org/en-US/docs/Web/CSS/margin).
    Margin,
    Margin::Length(0)
);

new_size_type!(
    /// Defines a width or height of an element.
    Size,
    Size::Auto
);

/// Sets the initial main size of a flex item.
///
/// See [the MDN documentation for flex-basis](https://developer.mozilla.org/en-US/docs/Web/CSS/flex-basis).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexBasis {
    /// Uses the value of the `width` or `height` property, or the content size if not set.
    #[default]
    Auto,
    /// Sets an absolute value.
    Length(u32),
    /// Sets a percentage of the width or height of the parent.
    Percent(f32),
}

impl From<FlexBasis> for Dimension {
    fn from(b: FlexBasis) -> Self {
        match b {
            FlexBasis::Auto => Dimension::Auto,
            FlexBasis::Length(l) => Dimension::Length(l as _),
            FlexBasis::Percent(p) => Dimension::Percent(p / 100.0),
        }
    }
}

/// A weight which can be applied to text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Weight {
    /// The normal weight.
    #[default]
    Normal,
    /// The bold weight.
    Bold,
    /// The light weight.
    Light,
}

bitflags! {
    /// Defines the edges of an element, e.g. for border styling.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Edges: u8 {
        /// The top edge.
        const Top = 0b00000001;
        /// The right edge.
        const Right = 0b00000010;
        /// The bottom edge.
        const Bottom = 0b00000100;
        /// The left edge.
        const Left = 0b00001000;
    }
}

#[doc(hidden)]
#[with_layout_style_props]
pub struct LayoutStyle {
    // fields added by proc macro, defined in ../macros/src/lib.rs
}

impl From<LayoutStyle> for Style {
    fn from(s: LayoutStyle) -> Self {
        Self {
            display: s.display,
            size: geometry::Size {
                width: s.width.into(),
                height: s.height.into(),
            },
            min_size: geometry::Size {
                width: s.min_width.into(),
                height: s.min_height.into(),
            },
            max_size: geometry::Size {
                width: s.max_width.into(),
                height: s.max_height.into(),
            },
            padding: Rect {
                left: s.padding_left.or(s.padding).into(),
                right: s.padding_right.or(s.padding).into(),
                top: s.padding_top.or(s.padding).into(),
                bottom: s.padding_bottom.or(s.padding).into(),
            },
            margin: Rect {
                left: s.margin_left.or(s.margin).into(),
                right: s.margin_right.or(s.margin).into(),
                top: s.margin_top.or(s.margin).into(),
                bottom: s.margin_bottom.or(s.margin).into(),
            },
            flex_direction: s.flex_direction,
            flex_wrap: s.flex_wrap,
            flex_basis: s.flex_basis.into(),
            flex_grow: s.flex_grow,
            flex_shrink: s.flex_shrink.unwrap_or(1.0),
            align_items: s.align_items,
            align_content: s.align_content,
            justify_content: s.justify_content,
            ..Default::default()
        }
    }
}
