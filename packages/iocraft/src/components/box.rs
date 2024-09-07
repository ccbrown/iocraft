use crate::{AnyElement, Color, Component, ComponentRenderer, ComponentUpdater, Edges, TextStyle};
use iocraft_macros::with_layout_style_props;
use taffy::{LengthPercentage, Rect};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BorderStyle {
    #[default]
    None,
    Single,
    Double,
    Round,
    Bold,
    DoubleLeftRight,
    DoubleTopBottom,
    Classic,
    Custom(BorderCharacters),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BorderCharacters {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub left: char,
    pub right: char,
    pub top: char,
    pub bottom: char,
}

impl BorderStyle {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

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

#[with_layout_style_props]
#[derive(Clone, Default)]
pub struct BoxProps {
    pub children: Vec<AnyElement>,
    pub border_style: BorderStyle,
    pub border_color: Option<Color>,
    pub border_edges: Option<Edges>,
    pub background_color: Option<Color>,
}

#[derive(Default)]
pub struct Box {
    border_style: BorderStyle,
    border_text_style: TextStyle,
    border_edges: Edges,
    background_color: Option<Color>,
}

impl Component for Box {
    type Props = BoxProps;

    fn new(_props: &Self::Props) -> Self {
        Default::default()
    }

    fn update(&mut self, props: &Self::Props, updater: &mut ComponentUpdater<'_>) {
        self.border_style = props.border_style;
        self.border_text_style = TextStyle {
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
        updater.update_children(props.children.iter().cloned());
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
    }
}
