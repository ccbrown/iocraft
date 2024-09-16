use crate::{
    AnyElement, CanvasTextStyle, Color, Component, ComponentRenderer, ComponentUpdater, Covariant,
    Edges,
};
use iocraft_macros::with_layout_style_props;
use taffy::{LengthPercentage, Rect};

/// A border style which can be applied to a [`Box`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BorderStyle {
    /// No border.
    #[default]
    None,
    /// A single-line border with 90-degree corners.
    Single,
    /// A double-line border with 90-degree corners.
    Double,
    /// A single-line border with rounded corners.
    Round,
    /// A single-line border with bold lines and 90-degree corners.
    Bold,
    /// A double-line border on the left and right with a single-line border on the top and bottom.
    DoubleLeftRight,
    /// A double-line border on the top and bottom with a single-line border on the left and right.
    DoubleTopBottom,
    /// A simple border consisting of basic ASCII characters.
    Classic,
    /// A custom border, rendered with characters of your choice.
    Custom(BorderCharacters),
}

/// The characters used to render a custom border for a [`Box`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BorderCharacters {
    /// The character used for the top-left corner.
    pub top_left: char,
    /// The character used for the top-right corner.
    pub top_right: char,
    /// The character used for the bottom-left corner.
    pub bottom_left: char,
    /// The character used for the bottom-right corner.
    pub bottom_right: char,
    /// The character used for the left edge.
    pub left: char,
    /// The character used for the right edge.
    pub right: char,
    /// The character used for the top edge.
    pub top: char,
    /// The character used for the bottom edge.
    pub bottom: char,
}

impl BorderStyle {
    /// Returns `true` if the border style is `None`.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns the characters used to render the border.
    pub fn border_characters(&self) -> Option<BorderCharacters> {
        Some(match self {
            Self::None => return None,
            Self::Single => BorderCharacters {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                left: '│',
                right: '│',
                top: '─',
                bottom: '─',
            },
            Self::Double => BorderCharacters {
                top_left: '╔',
                top_right: '╗',
                bottom_left: '╚',
                bottom_right: '╝',
                left: '║',
                right: '║',
                top: '═',
                bottom: '═',
            },
            Self::Round => BorderCharacters {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
                left: '│',
                right: '│',
                top: '─',
                bottom: '─',
            },
            Self::Bold => BorderCharacters {
                top_left: '┏',
                top_right: '┓',
                bottom_left: '┗',
                bottom_right: '┛',
                left: '┃',
                right: '┃',
                top: '━',
                bottom: '━',
            },
            Self::DoubleLeftRight => BorderCharacters {
                top_left: '╓',
                top_right: '╖',
                bottom_left: '╙',
                bottom_right: '╜',
                left: '║',
                right: '║',
                top: '─',
                bottom: '─',
            },
            Self::DoubleTopBottom => BorderCharacters {
                top_left: '╒',
                top_right: '╕',
                bottom_left: '╘',
                bottom_right: '╛',
                left: '│',
                right: '│',
                top: '═',
                bottom: '═',
            },
            Self::Classic => BorderCharacters {
                top_left: '+',
                top_right: '+',
                bottom_left: '+',
                bottom_right: '+',
                left: '|',
                right: '|',
                top: '-',
                bottom: '-',
            },
            Self::Custom(chars) => *chars,
        })
    }
}

/// The props which can be passed to the [`Box`] component.
#[with_layout_style_props]
#[derive(Covariant, Default)]
pub struct BoxProps<'a> {
    /// The elements to render inside of the box.
    pub children: Vec<AnyElement<'a>>,

    /// The style of the border. By default, the box will have no border.
    pub border_style: BorderStyle,

    /// The color of the border.
    pub border_color: Option<Color>,

    /// The edges to render the border on. By default, the border will be rendered on all edges.
    pub border_edges: Option<Edges>,

    /// The color of the background.
    pub background_color: Option<Color>,
}

/// `Box` is your most fundamental building block for laying out and styling components.
#[derive(Default)]
pub struct Box {
    border_style: BorderStyle,
    border_text_style: CanvasTextStyle,
    border_edges: Edges,
    background_color: Option<Color>,
}

impl Component for Box {
    type Props<'a> = BoxProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Default::default()
    }

    fn update(&mut self, props: &mut Self::Props<'_>, updater: &mut ComponentUpdater) {
        self.border_style = props.border_style;
        self.border_text_style = CanvasTextStyle {
            color: props.border_color,
            ..Default::default()
        };
        self.border_edges = props.border_edges.unwrap_or(Edges::all());
        self.background_color = props.background_color;
        let mut style: taffy::style::Style = props.layout_style().into();
        style.border = if self.border_style.is_none() {
            Rect::zero()
        } else {
            Rect {
                top: LengthPercentage::Length(if self.border_edges.contains(Edges::Top) {
                    1.0
                } else {
                    0.0
                }),
                bottom: LengthPercentage::Length(if self.border_edges.contains(Edges::Bottom) {
                    1.0
                } else {
                    0.0
                }),
                left: LengthPercentage::Length(if self.border_edges.contains(Edges::Left) {
                    1.0
                } else {
                    0.0
                }),
                right: LengthPercentage::Length(if self.border_edges.contains(Edges::Right) {
                    1.0
                } else {
                    0.0
                }),
            }
        };
        updater.set_layout_style(style);
        updater.update_children(props.children.iter_mut(), None);
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        let layout = renderer.layout();

        let mut canvas = renderer.canvas();

        if let Some(color) = self.background_color {
            canvas.set_background_color(
                0,
                0,
                layout.size.width as usize,
                layout.size.height as usize,
                color,
            );
        }

        if let Some(border) = self.border_style.border_characters() {
            let left_border_size = if self.border_edges.contains(Edges::Left) {
                1
            } else {
                0
            };
            let right_border_size = if self.border_edges.contains(Edges::Right) {
                1
            } else {
                0
            };
            let top_border_size = if self.border_edges.contains(Edges::Top) {
                1
            } else {
                0
            };
            let bottom_border_size = if self.border_edges.contains(Edges::Bottom) {
                1
            } else {
                0
            };

            if self.border_edges.contains(Edges::Top) {
                if self.border_edges.contains(Edges::Left) {
                    canvas.set_text(0, 0, &border.top_left.to_string(), self.border_text_style);
                }

                let top = border
                    .top
                    .to_string()
                    .repeat(layout.size.width as usize - left_border_size - right_border_size);
                canvas.set_text(left_border_size as _, 0, &top, self.border_text_style);

                if self.border_edges.contains(Edges::Right) {
                    canvas.set_text(
                        layout.size.width as isize - 1,
                        0,
                        &border.top_right.to_string(),
                        self.border_text_style,
                    );
                }
            }

            for y in top_border_size..(layout.size.height as isize - bottom_border_size) {
                if self.border_edges.contains(Edges::Left) {
                    canvas.set_text(0, y, &border.left.to_string(), self.border_text_style);
                }
                if self.border_edges.contains(Edges::Right) {
                    canvas.set_text(
                        layout.size.width as isize - 1,
                        y,
                        &border.right.to_string(),
                        self.border_text_style,
                    );
                }
            }

            if self.border_edges.contains(Edges::Bottom) {
                if self.border_edges.contains(Edges::Left) {
                    canvas.set_text(
                        0,
                        layout.size.height as isize - 1,
                        &border.bottom_left.to_string(),
                        self.border_text_style,
                    );
                }

                let bottom = border
                    .bottom
                    .to_string()
                    .repeat(layout.size.width as usize - left_border_size - right_border_size);
                canvas.set_text(
                    left_border_size as _,
                    layout.size.height as isize - 1,
                    &bottom,
                    self.border_text_style,
                );

                if self.border_edges.contains(Edges::Right) {
                    canvas.set_text(
                        layout.size.width as isize - 1,
                        layout.size.height as isize - 1,
                        &border.bottom_right.to_string(),
                        self.border_text_style,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use indoc::indoc;

    #[test]
    fn test_box() {
        assert_eq!(element!(Box).to_string(), "");

        assert_eq!(
            element! {
                Box {
                    Text(content: "foo")
                    Text(content: "bar")
                }
            }
            .to_string(),
            "foobar\n"
        );

        assert_eq!(
            element! {
                Box(padding: 1) {
                    Text(content: "foo")
                }
            }
            .to_string(),
            "\n foo\n\n"
        );

        assert_eq!(
            element! {
                Box(margin: 2) {
                    Text(content: "foo")
                }
            }
            .to_string(),
            "\n\n  foo\n\n\n"
        );

        assert_eq!(
            element! {
                Box(width: 20) {
                    Box(width: 60pct) {
                        Text(content: "foo")
                    }
                    Box(width: 40pct) {
                        Text(content: "bar")
                    }
                }
            }
            .to_string(),
            "foo         bar\n"
        );

        assert_eq!(
            element! {
                Box(width: 20, border_style: BorderStyle::Single) {
                    Box(width: 60pct) {
                        Text(content: "foo")
                    }
                    Box(width: 40pct) {
                        Text(content: "bar")
                    }
                }
            }
            .to_string(),
            indoc! {"
                ┌──────────────────┐
                │foo        bar    │
                └──────────────────┘
            "},
        );

        assert_eq!(
            element! {
                Box(width: 8, border_style: BorderStyle::Single, justify_content: JustifyContent::Center) {
                    Text(content: "✅")
                }
            }
            .to_string(),
            indoc! {"
                ┌──────┐
                │  ✅  │
                └──────┘
            "},
        );
    }
}
