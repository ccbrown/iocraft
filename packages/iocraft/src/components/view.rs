use crate::{
    AnyElement, CanvasTextStyle, Color, Component, ComponentDrawer, ComponentUpdater, Edges, Hooks,
    Props,
};
use iocraft_macros::with_layout_style_props;
use taffy::{LengthPercentage, Rect};

/// A border style which can be applied to a [`View`].
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

/// The characters used to render a custom border for a [`View`].
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BorderTitlePos {
    #[default]
    Top,
    Bottom
}

/// The characters used to render a custom border for a [`View`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BorderTitle {
    pub title: String,
    pub pos: BorderTitlePos,
}

fn center_and_clip(border_char: &str, length: usize, title: &str) -> String {
    let title_len = title.chars().count();

    // If inner is longer, clip it
    let (clipped_title, clipped_len) = if title_len > length {
        (title.chars().take(length).collect(), length)
    } else {
        (title.to_string(), title_len)
    };

    // Calculate padding
    let total_padding = length.saturating_sub(clipped_len);
    let left_padding = total_padding / 2;
    let right_padding = total_padding - left_padding;

    format!(
        "{}{}{}",
        border_char.repeat(left_padding),
        clipped_title,
        border_char.repeat(right_padding)
    )
}

/// The props which can be passed to the [`View`] component.
#[non_exhaustive]
#[with_layout_style_props]
#[derive(Default, Props)]
pub struct ViewProps<'a> {
    /// The elements to render inside of the view.
    pub children: Vec<AnyElement<'a>>,

    /// The style of the border. By default, the view will have no border.
    pub border_style: BorderStyle,

    /// The color of the border.
    pub border_color: Option<Color>,

    /// The edges to render the border on. By default, the border will be rendered on all edges.
    pub border_edges: Option<Edges>,

    /// The color of the border.
    pub border_title: Option<BorderTitle>,

    /// The color of the background.
    pub background_color: Option<Color>,
}

/// `View` is your most fundamental building block for laying out and styling components.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # fn my_element() -> impl Into<AnyElement<'static>> {
/// element! {
///     View(padding: 2, border_style: BorderStyle::Round) {
///         Text(content: "Hello!")
///     }
/// }
/// # }
/// ```
#[derive(Default)]
pub struct View {
    border_style: BorderStyle,
    border_text_style: CanvasTextStyle,
    border_edges: Edges,
    background_color: Option<Color>,
    border_title: Option<BorderTitle>,
}

impl Component for View {
    type Props<'a> = ViewProps<'a>;

    fn new(_props: &Self::Props<'_>) -> Self {
        Default::default()
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        _hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        self.border_style = props.border_style;
        self.border_text_style = CanvasTextStyle {
            color: props.border_color,
            ..Default::default()
        };
        self.border_edges = props.border_edges.unwrap_or(Edges::all());
        self.background_color = props.background_color;
        self.border_title = props.border_title.clone();

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

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_>) {
        let layout = drawer.layout();

        let mut canvas = drawer.canvas();

        if let Some(color) = self.background_color {
            canvas.clear_text(
                0,
                0,
                layout.size.width as usize,
                layout.size.height as usize,
            );
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

                let (top_size, top_char) = (
                    layout.size.width as usize - left_border_size - right_border_size,
                    border.top.to_string(),
                );
                let top = match self.border_title {
                    Some(ref title) if title.pos == BorderTitlePos::Top =>
                        center_and_clip(&top_char, top_size, &title.title),
                    _ => top_char.repeat(top_size),
                };

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

                let (bottom_size, bottom_char) = (
                    layout.size.width as usize - left_border_size - right_border_size,
                    border.bottom.to_string(),
                );
                let bottom = match self.border_title {
                    Some(ref title) if title.pos == BorderTitlePos::Bottom =>
                        center_and_clip(&bottom_char, bottom_size, &title.title),
                    _ => bottom_char.repeat(bottom_size),
                };

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

    #[derive(Default, Props)]
    pub struct MyTextProps {
        pub content: String,
    }

    #[component]
    pub fn MyText<'a>(props: &MyTextProps) -> impl Into<AnyElement<'a>> {
        element! {
            Text(content: &props.content)
        }
    }

    #[test]
    fn test_view() {
        assert_eq!(element!(View).to_string(), "");

        assert_eq!(
            element! {
                View {
                    Text(content: "foo")
                    Text(content: "bar")
                }
            }
            .to_string(),
            "foobar\n"
        );

        assert_eq!(
            element! {
                View(padding: 1) {
                    Text(content: "foo")
                }
            }
            .to_string(),
            "\n foo\n\n"
        );

        assert_eq!(
            element! {
                View(margin: 2) {
                    Text(content: "foo")
                }
            }
            .to_string(),
            "\n\n  foo\n\n\n"
        );

        assert_eq!(
            element! {
                View(width: 20) {
                    View(width: 60pct) {
                        Text(content: "foo")
                    }
                    View(width: 40pct) {
                        Text(content: "bar")
                    }
                }
            }
            .to_string(),
            "foo         bar\n"
        );

        assert_eq!(
            element! {
                View(width: 20, border_style: BorderStyle::Single) {
                    View(width: 60pct) {
                        Text(content: "foo")
                    }
                    View(width: 40pct) {
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
                View(flex_direction: FlexDirection::Column) {
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
            .to_string(),
            indoc! {"
                ┌──────┐  ╔══════╗  ╭─────╮  ┏━━━━┓
                │Single│  ║Double║  │Round│  ┃Bold┃
                └──────┘  ╚══════╝  ╰─────╯  ┗━━━━┛

                ╓───────────────╖  ╒═══════════════╕  +-------+
                ║DoubleLeftRight║  │DoubleTopBottom│  |Classic|
                ╙───────────────╜  ╘═══════════════╛  +-------+
            "},
        );

        assert_eq!(
            element! {
                View(width: 8, border_style: BorderStyle::Single, justify_content: JustifyContent::Center) {
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

        let extra_space = if handles_vs16_incorrectly() { " " } else { "" };

        assert_eq!(
            element! {
                View(width: 8, border_style: BorderStyle::Single, justify_content: JustifyContent::Center) {
                    Text(content: "☀️")
                }
            }
            .to_string(),
            format!(indoc! {"
                ┌──────┐
                │  ☀️{}  │
                └──────┘
            "}, extra_space),
        );

        assert_eq!(
            element! {
                View(width: 8, border_style: BorderStyle::Single, justify_content: JustifyContent::Center) {
                    Text(content: "☀️☀️")
                }
            }
            .to_string(),
            format!(indoc! {"
                ┌──────┐
                │ ☀️{}☀️{} │
                └──────┘
            "}, extra_space, extra_space),
        );

        assert_eq!(
            element! {
                View(width: 12, border_style: BorderStyle::Single, justify_content: JustifyContent::Center) {
                    Text(content: "フーバー")
                }
            }
            .to_string(),
            indoc! {"
                ┌──────────┐
                │ フーバー │
                └──────────┘
            "},
        );

        assert_eq!(
            element! {
                View(
                    border_style: BorderStyle::Round,
                    flex_direction: FlexDirection::Column,
                ) {
                    View(
                        margin_top: -1,
                    ) {
                        Text(content: "Title")
                    }
                    Text(content: "Hello, world!")
                }
            }
            .to_string(),
            indoc! {"
                ╭Title────────╮
                │Hello, world!│
                ╰─────────────╯
            "},
        );

        assert_eq!(
            element! {
                View {
                    Text(content: "This is the background text.")
                    View(
                        position: Position::Absolute,
                        top: 0,
                        left: 3,
                    ) {
                        Text(content: "Foo!")
                    }
                }
            }
            .to_string(),
            "ThiFoo! the background text.\n",
        );

        assert_eq!(
            element! {
                View {
                    Text(content: "This is the background text.")
                    View(
                        position: Position::Absolute,
                        top: 0,
                        left: 3,
                        width: 6,
                        height: 1,
                        background_color: Color::Red,
                    )
                }
            }
            .to_string(),
            "Thi      he background text.\n",
        );

        assert_eq!(
            element! {
                View(width: 20, border_style: BorderStyle::Single, column_gap: 2) {
                    View(width: 3) {
                        Text(content: "foo")
                    }
                    View(width: 3) {
                        Text(content: "bar")
                    }
                }
            }
            .to_string(),
            indoc! {"
                ┌──────────────────┐
                │foo  bar          │
                └──────────────────┘
            "},
        );

        // regression test for https://github.com/ccbrown/iocraft/issues/52
        assert_eq!(
            element! {
                View(width: 20, border_style: BorderStyle::Single, row_gap: 1, flex_direction: FlexDirection::Column) {
                    Text(content: "foo")
                    MyText(content: "bar")
                    MyText(content: "baz")
                }
            }
            .to_string(),
            indoc! {"
                ┌──────────────────┐
                │foo               │
                │                  │
                │bar               │
                │                  │
                │baz               │
                └──────────────────┘
            "},
        );

        assert_eq!(
            element! {
                View(width: 20, height: 7, margin_top: 1, border_style: BorderStyle::Single) {
                    View(width: 5, height: 3, position: Position::Absolute, top: -2) {
                        Text(content: "foo")
                    }
                }
            }
            .to_string(),
            indoc! {"
                 foo
                ┌──────────────────┐
                │                  │
                │                  │
                │                  │
                │                  │
                │                  │
                └──────────────────┘
            "},
        );

        assert_eq!(
            element! {
                View(width: 20, height: 7, margin_top: 1, border_style: BorderStyle::Single) {
                    View(width: 5, height: 3, position: Position::Absolute, top: -3) {
                        Text(content: "foo\nbar")
                    }
                }
            }
            .to_string(),
            indoc! {"
                 bar
                ┌──────────────────┐
                │                  │
                │                  │
                │                  │
                │                  │
                │                  │
                └──────────────────┘
            "},
        );

        assert_eq!(
            element! {
                View(width: 20, height: 7, border_style: BorderStyle::Single, overflow: Overflow::Hidden) {
                    View(position: Position::Absolute, top: -1, left: 17) {
                        Text(content: "foo\nbar")
                    }
                }
            }
            .to_string(),
            indoc! {"
                ┌──────────────────┐
                │                 b│
                │                  │
                │                  │
                │                  │
                │                  │
                └──────────────────┘
            "},
        );
    }
}
