use crate::{AnyElement, Component, ComponentProps, ComponentRenderer, ComponentUpdater};
use crossterm::style::{Color, ContentStyle, PrintStyledContent, StyledContent};
use flashy_macros::with_layout_style_props;
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

impl ComponentProps for BoxProps {
    type Component = Box;
}

pub struct Box {
    props: BoxProps,
}

impl Component for Box {
    type Props = BoxProps;
    type State = ();

    fn new(props: Self::Props) -> Self {
        Self { props }
    }

    fn set_props(&mut self, props: Self::Props) {
        self.props = props;
    }

    fn update(&self, updater: &mut ComponentUpdater<'_>) {
        let mut style: taffy::style::Style = self.props.layout_style().into();
        style.border = Rect::length(if self.props.border_style.is_none() {
            0.0
        } else {
            1.0
        });
        updater.set_layout_style(style);
        updater.update_children(self.props.children.iter().cloned());
    }

    fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        let layout = renderer.layout();

        if let Some(border) = self.props.border_style.border_characters() {
            let style = ContentStyle {
                foreground_color: self.props.border_color,
                ..ContentStyle::new()
            };

            renderer.queue(PrintStyledContent(StyledContent::new(
                style,
                &border.top_left,
            )));

            let top = border
                .top
                .to_string()
                .repeat(layout.size.width as usize - 2);
            renderer.queue(PrintStyledContent(StyledContent::new(style, &top)));

            renderer.queue(PrintStyledContent(StyledContent::new(
                style,
                &border.top_right,
            )));

            renderer.move_cursor(0, 1);
            let left = PrintStyledContent(StyledContent::new(style, border.left));
            let right = PrintStyledContent(StyledContent::new(style, border.right));
            for y in 1..(layout.size.height as u16 - 1) {
                renderer.move_cursor(0, y);
                renderer.queue(left);
                renderer.move_cursor(layout.size.width as u16 - 1, y);
                renderer.queue(right);
            }

            renderer.move_cursor(0, layout.size.height as u16 - 1);

            renderer.queue(PrintStyledContent(StyledContent::new(
                style,
                &border.bottom_left,
            )));

            let bottom = border
                .bottom
                .to_string()
                .repeat(layout.size.width as usize - 2);
            renderer.queue(PrintStyledContent(StyledContent::new(style, &bottom)));

            renderer.queue(PrintStyledContent(StyledContent::new(
                style,
                &border.bottom_right,
            )));
        }
    }
}
