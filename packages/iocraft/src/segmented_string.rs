use crate::unicode_linebreak::{linebreaks_iter, BreakOpportunity};
use core::{
    fmt::{self, Display},
    mem,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A `SegmentedString` is a string consisting of multiple segments, which don't have to be
/// contiguous.
///
/// This is primarily used for wrapping text as the result will include information sufficient to
/// map output regions to input data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SegmentedString<'a> {
    segments: Vec<&'a str>,
}

/// A `SegmentedStringLine` is a line of text after wrapping.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SegmentedStringLine<'a> {
    pub segments: Vec<SegmentedStringLineSegment<'a>>,
    pub width: usize,
}

impl<'a> SegmentedStringLine<'a> {
    fn push_segment(&mut self, segment: SegmentedStringLineSegment<'a>) {
        self.width += segment.width;
        self.segments.push(segment);
    }

    /// Removes trailing whitespace from the line.
    pub fn trim_end(&mut self) {
        for i in (0..self.segments.len()).rev() {
            let segment = &mut self.segments[i];
            let width_before = segment.width;
            segment.trim_end();
            self.width -= width_before - segment.width;
            if segment.width > 0 {
                break;
            } else {
                self.segments.pop();
            }
        }
    }
}

impl Display for SegmentedStringLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.segments {
            write!(f, "{}", segment)?;
        }
        Ok(())
    }
}

/// A `SegmentedStringLineSegment` is a segment making up part of a `SegmentedStringLine`, along
/// with information about where it came from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SegmentedStringLineSegment<'a> {
    pub text: &'a str,
    pub index: usize,
    pub offset: usize,
    pub width: usize,
}

impl SegmentedStringLineSegment<'_> {
    fn substring(&self, start: usize, end: usize) -> Self {
        let text = &self.text[start..end];
        let width = text.width();
        SegmentedStringLineSegment {
            text,
            index: self.index,
            offset: self.offset + start,
            width,
        }
    }

    fn trim_end(&mut self) {
        self.text = self.text.trim_end();
        self.width = self.text.width();
    }
}

impl Display for SegmentedStringLineSegment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl<'a> SegmentedString<'a> {
    fn new_segmented_string_line_segment(
        &self,
        index: usize,
        offset: usize,
        end: usize,
    ) -> SegmentedStringLineSegment {
        let text = self.segments[index][offset..end].trim_end_matches('\n');
        let width = text.width();
        SegmentedStringLineSegment {
            text,
            index,
            offset,
            width,
        }
    }

    fn merge_adjacent_line_segments(&self, line: &mut SegmentedStringLine<'a>) {
        if line.segments.is_empty() {
            return;
        }

        let mut segments = mem::take(&mut line.segments).into_iter();

        line.segments.push(segments.next().unwrap());

        for segment in segments {
            let prev = line.segments.last_mut().unwrap();
            if segment.index == prev.index && segment.offset == prev.offset + prev.text.len() {
                let merged_text =
                    &self.segments[segment.index][prev.offset..segment.offset + segment.text.len()];
                let merged_width = prev.width + segment.width;

                prev.text = merged_text;
                prev.width = merged_width;
            } else {
                line.segments.push(segment);
            }
        }
    }

    /// Wraps the string into lines of a given width.
    pub fn wrap(&self, width: usize) -> Vec<SegmentedStringLine> {
        if self.segments.is_empty() {
            return vec![];
        }

        let chars = self
            .segments
            .iter()
            .enumerate()
            .flat_map(|(i, s)| s.char_indices().map(move |(j, c)| ((i, j), c)));

        let break_opportunities = linebreaks_iter(
            chars,
            (self.segments.len() - 1, self.segments.last().unwrap().len()),
        );

        let mut lines = Vec::new();
        let mut current_line = SegmentedStringLine::default();

        let (mut start_segment_idx, mut start_char_idx) = (0, 0);
        for ((segment_idx, char_idx), opportunity_type) in break_opportunities {
            let mut new_line_segments = vec![];

            while start_segment_idx < segment_idx {
                new_line_segments.push(self.new_segmented_string_line_segment(
                    start_segment_idx,
                    start_char_idx,
                    self.segments[start_segment_idx].len(),
                ));
                start_segment_idx += 1;
                start_char_idx = 0;
            }
            if start_char_idx < char_idx {
                new_line_segments.push(self.new_segmented_string_line_segment(
                    start_segment_idx,
                    start_char_idx,
                    char_idx,
                ));
                start_char_idx = char_idx;
            }

            let new_line_segments_width: usize = new_line_segments.iter().map(|s| s.width).sum();

            let trailing_whitespace_width: usize = new_line_segments
                .iter()
                .rev()
                .flat_map(|s| s.text.chars().rev())
                .take_while(|c| c.is_whitespace())
                .map(|c| c.width().unwrap_or(0))
                .sum();

            if current_line.width + (new_line_segments_width - trailing_whitespace_width) <= width {
                // Everything fits into the current line
                current_line.segments.append(&mut new_line_segments);
                current_line.width += new_line_segments_width;
            } else {
                // Break if necessary, then add more lines
                if current_line.width > 0 {
                    lines.push(current_line);
                    current_line = SegmentedStringLine::default();
                }
                for segment in new_line_segments {
                    let trailing_whitespace_idx = segment
                        .text
                        .char_indices()
                        .rev()
                        .take_while(|(_, c)| c.is_whitespace())
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(segment.text.len());
                    let trailing_whitespace_width = segment.text[trailing_whitespace_idx..].width();

                    if current_line.width + (segment.width - trailing_whitespace_width) > width {
                        // This segment is too wide, we need to forcefully break it
                        let mut w = 0;
                        let mut start_idx = 0;
                        for (idx, c) in segment.text.char_indices() {
                            if idx >= trailing_whitespace_idx {
                                break;
                            }
                            let char_width = c.width().unwrap_or(0);
                            if w > 0 && w + char_width > width {
                                // We have a full line
                                current_line.push_segment(segment.substring(start_idx, idx));
                                lines.push(current_line);
                                current_line = SegmentedStringLine::default();
                                w = 0;
                                start_idx = idx;
                            }
                            w += char_width;
                        }
                        // Add the remaining part of the segment, if any
                        if start_idx < segment.text.len() {
                            current_line
                                .push_segment(segment.substring(start_idx, segment.text.len()));
                        }
                    } else {
                        current_line.push_segment(segment);
                    }
                }
            }

            if opportunity_type == BreakOpportunity::Mandatory {
                // We have to break here
                lines.push(current_line);
                current_line = SegmentedStringLine::default();
            }
        }

        // Add another line if the last segment ends with a newline
        {
            let last_segment = &self.segments[self.segments.len() - 1];
            let has_trailing_newline = last_segment.ends_with('\n');
            if has_trailing_newline {
                current_line.push_segment(self.new_segmented_string_line_segment(
                    self.segments.len() - 1,
                    last_segment.len(),
                    last_segment.len(),
                ));
                lines.push(current_line);
            }
        }

        for line in &mut lines {
            self.merge_adjacent_line_segments(line);
        }

        lines
    }
}

impl<'a> From<&'a str> for SegmentedString<'a> {
    fn from(text: &'a str) -> Self {
        [text].into_iter().collect()
    }
}

impl<'a> FromIterator<&'a str> for SegmentedString<'a> {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        SegmentedString {
            segments: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segmented_string_wrap() {
        {
            let segmented_string = SegmentedString::from("Hello, world! This is a test string.");
            let lines = segmented_string.wrap(12);
            assert_eq!(
                lines,
                vec![
                    SegmentedStringLine {
                        segments: vec![SegmentedStringLineSegment {
                            text: "Hello, ",
                            index: 0,
                            offset: 0,
                            width: 7
                        },],
                        width: 7,
                    },
                    SegmentedStringLine {
                        segments: vec![SegmentedStringLineSegment {
                            text: "world! This ",
                            index: 0,
                            offset: 7,
                            width: 12
                        },],
                        width: 12,
                    },
                    SegmentedStringLine {
                        segments: vec![SegmentedStringLineSegment {
                            text: "is a test ",
                            index: 0,
                            offset: 19,
                            width: 10
                        },],
                        width: 10,
                    },
                    SegmentedStringLine {
                        segments: vec![SegmentedStringLineSegment {
                            text: "string.",
                            index: 0,
                            offset: 29,
                            width: 7
                        },],
                        width: 7,
                    }
                ],
            );
        }

        {
            let segmented_string = SegmentedString::from("foo bar");
            let lines = segmented_string
                .wrap(0)
                .into_iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            assert_eq!(lines, vec!["f", "o", "o ", "b", "a", "r"]);
        }

        {
            let segmented_string = SegmentedString::from("Hello, world! This is a test string.");
            let lines = segmented_string
                .wrap(12)
                .into_iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            assert_eq!(
                lines,
                vec!["Hello, ", "world! This ", "is a test ", "string.",]
            );
        }

        {
            let segmented_string = SegmentedString::from("Hello, world! This is a test string.");
            let lines = segmented_string
                .wrap(11)
                .into_iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            assert_eq!(
                lines,
                vec!["Hello, ", "world! This ", "is a test ", "string.",]
            );
        }

        {
            let segmented_string =
                SegmentedString::from("Hello, thisisalongunbreakablemultiline str.");
            let lines = segmented_string
                .wrap(12)
                .into_iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            assert_eq!(
                lines,
                vec!["Hello, ", "thisisalongu", "nbreakablemu", "ltiline str.",]
            );
        }

        {
            let segmented_string =
                SegmentedString::from("Hello, this\nstring\nhas\nnewlines in it.\n\n");
            let lines = segmented_string
                .wrap(11)
                .into_iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            assert_eq!(
                lines,
                vec![
                    "Hello, this",
                    "string",
                    "has",
                    "newlines in ",
                    "it.",
                    "",
                    ""
                ]
            );
        }

        {
            let segmented_string: SegmentedString =
                ["this is ", "a wrapping test"].into_iter().collect();
            let lines = segmented_string
                .wrap(14)
                .into_iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            assert_eq!(lines, vec!["this is a ", "wrapping test"]);
        }
    }
}
