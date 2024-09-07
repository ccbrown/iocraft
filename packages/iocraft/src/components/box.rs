use crate::{AnyElement, Color, Component, ComponentRenderer, ComponentUpdater, TextStyle};
use iocraft_macros::with_layout_style_props;
use taffy::Rect;

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
}

#[derive(Default)]
pub struct Box {
    border_style: BorderStyle,
    border_text_style: TextStyle,
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
        let mut style: taffy::style::Style = props.layout_style().into();
        style.border = Rect::length(if props.border_style.is_none() {
            0.0
        } else {
            1.0
        });
        updater.set_layout_style(style);
        updater.update_children(props.children.iter().cloned());
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        let layout = renderer.layout();

        if let Some(border) = self.border_style.border_characters() {
            let mut canvas = renderer.canvas();

            canvas.set_text(0, 0, &border.top_left.to_string(), self.border_text_style);

            let top = border
                .top
                .to_string()
                .repeat(layout.size.width as usize - 2);
            canvas.set_text(1, 0, &top, self.border_text_style);

            canvas.set_text(
                layout.size.width as isize - 1,
                0,
                &border.top_right.to_string(),
                self.border_text_style,
            );

            for y in 1..(layout.size.height as isize - 1) {
                canvas.set_text(0, y, &border.left.to_string(), self.border_text_style);
                canvas.set_text(
                    layout.size.width as isize - 1,
                    y,
                    &border.right.to_string(),
                    self.border_text_style,
                );
            }

            canvas.set_text(
                0,
                layout.size.height as isize - 1,
                &border.bottom_left.to_string(),
                self.border_text_style,
            );

            let bottom = border
                .bottom
                .to_string()
                .repeat(layout.size.width as usize - 2);
            canvas.set_text(
                1,
                layout.size.height as isize - 1,
                &bottom,
                self.border_text_style,
            );

            canvas.set_text(
                layout.size.width as isize - 1,
                layout.size.height as isize - 1,
                &border.bottom_right.to_string(),
                self.border_text_style,
            );
        }
    }
}
