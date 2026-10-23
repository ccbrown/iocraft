use crate::unicode_linebreak::{linebreaks_iter, BreakOpportunity};
use core::{
    fmt::{self, Display},
    mem,
};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
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

    pub(crate) fn for_each_grapheme_span(
        &self,
        buffer: &mut String,
        mut f: impl FnMut(&str, usize),
    ) {
        if let [segment] = self.segments.as_slice() {
            f(segment.text, segment.index);
            return;
        }
        let len = self.segments.iter().map(|segment| segment.text.len()).sum();
        let mut pending: Option<(usize, usize, usize)> = None;
        for (start, end) in segmented_graphemes(len, &self.segments) {
            if start.0 == end.0 {
                match pending.take() {
                    Some((segment, begin, _)) if segment == start.0 => {
                        pending = Some((segment, begin, end.1));
                    }
                    previous => {
                        if let Some((segment, begin, end)) = previous {
                            let segment = &self.segments[segment];
                            f(&segment.text[begin..end], segment.index);
                        }
                        pending = Some((start.0, start.1, end.1));
                    }
                }
            } else {
                if let Some((segment, begin, end)) = pending.take() {
                    let segment = &self.segments[segment];
                    f(&segment.text[begin..end], segment.index);
                }
                f(
                    grapheme_text(&self.segments, start, end, buffer),
                    self.segments[start.0].index,
                );
            }
        }
        if let Some((segment, begin, end)) = pending {
            let segment = &self.segments[segment];
            f(&segment.text[begin..end], segment.index);
        }
    }

    /// Removes trailing whitespace from the line.
    pub fn trim_end(&mut self) {
        for i in (0..self.segments.len()).rev() {
            let segment = &mut self.segments[i];
            let width_before = segment.width;
            segment.trim_end();
            self.width -= width_before - segment.width;
            if !segment.text.is_empty() {
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
    // Only the first fragment of a grapheme contributes to the line's width.
    pub width: usize,
}

impl SegmentedStringLineSegment<'_> {
    fn substring(&self, start: usize, end: usize, width: usize) -> Self {
        let text = &self.text[start..end];
        SegmentedStringLineSegment {
            text,
            index: self.index,
            offset: self.offset + start,
            width,
        }
    }

    fn trim_end(&mut self) {
        let text = self.text.trim_end();
        self.width -= self.text[text.len()..].width();
        self.text = text;
    }
}

impl Display for SegmentedStringLineSegment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

type SegmentBoundary = (usize, usize, usize);

fn segmented_grapheme_boundaries<'a>(
    len: usize,
    chunk: impl Fn(usize) -> &'a str,
) -> impl Iterator<Item = SegmentBoundary> {
    let mut cursor = GraphemeCursor::new(0, len, true);
    let (mut segment, mut start) = (0, 0);
    std::iter::from_fn(move || loop {
        match cursor.next_boundary(chunk(segment), start) {
            Ok(Some(offset)) => return Some((segment, offset - start, offset)),
            Ok(None) => return None,
            Err(GraphemeIncomplete::NextChunk) => {
                start += chunk(segment).len();
                segment += 1;
            }
            Err(GraphemeIncomplete::PreContext(offset)) => {
                let (mut context_segment, mut context_start) = (segment, start);
                while context_start >= offset && context_segment > 0 {
                    context_segment -= 1;
                    context_start -= chunk(context_segment).len();
                }
                cursor.provide_context(
                    &chunk(context_segment)[..offset - context_start],
                    context_start,
                );
            }
            Err(incomplete) => {
                unreachable!("forward grapheme cursor has a valid chunk: {incomplete:?}");
            }
        }
    })
}

fn segmented_graphemes<'a>(
    len: usize,
    segments: &'a [SegmentedStringLineSegment<'_>],
) -> impl Iterator<Item = (SegmentBoundary, SegmentBoundary)> + 'a {
    segmented_grapheme_boundaries(len, |i| segments[i].text).scan((0, 0, 0), move |start, end| {
        while start.0 < end.0 && start.1 == segments[start.0].text.len() {
            start.0 += 1;
            start.1 = 0;
        }
        let range = (*start, end);
        *start = end;
        Some(range)
    })
}

fn grapheme_text<'a>(
    segments: &'a [SegmentedStringLineSegment<'_>],
    start: SegmentBoundary,
    end: SegmentBoundary,
    buffer: &'a mut String,
) -> &'a str {
    if start.0 == end.0 {
        return &segments[start.0].text[start.1..end.1];
    }
    buffer.clear();
    for (i, segment) in segments.iter().enumerate().take(end.0 + 1).skip(start.0) {
        let begin = if i == start.0 { start.1 } else { 0 };
        let end = if i == end.0 {
            end.1
        } else {
            segment.text.len()
        };
        buffer.push_str(&segment.text[begin..end]);
    }
    buffer
}

impl<'a> SegmentedString<'a> {
    fn new_segmented_string_line_segment(
        &self,
        index: usize,
        offset: usize,
        end: usize,
    ) -> SegmentedStringLineSegment<'_> {
        let text = self.segments[index][offset..end].trim_end_matches(['\r', '\n']);
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

    /// Wraps the string into lines of a given width without splitting extended grapheme clusters.
    pub fn wrap(&self, width: usize) -> Vec<SegmentedStringLine<'_>> {
        if self.segments.is_empty() {
            return vec![];
        }

        let len = self.segments.iter().map(|segment| segment.len()).sum();
        // Keep whole-text grapheme boundaries without copying the styled segments.
        let mut grapheme_boundaries = segmented_grapheme_boundaries(len, |i| self.segments[i])
            .map(|(_, _, offset)| offset)
            .peekable();

        let chars = self
            .segments
            .iter()
            .enumerate()
            .flat_map(|(i, s)| s.char_indices().map(move |(j, c)| ((i, j), c)))
            .scan(0, |offset, ((segment, character), c)| {
                let index = (segment, character, *offset);
                *offset += c.len_utf8();
                Some((index, c))
            });

        let break_opportunities = linebreaks_iter(
            chars,
            (
                self.segments.len() - 1,
                self.segments.last().unwrap().len(),
                len,
            ),
        );

        let mut lines = Vec::new();
        let mut current_line = SegmentedStringLine::default();
        let mut grapheme_buffer = String::new();
        let mut segment_widths = Vec::new();

        let (mut start_segment_idx, mut start_char_idx) = (0, 0);
        for ((segment_idx, char_idx, offset), opportunity_type) in break_opportunities {
            if opportunity_type == BreakOpportunity::Allowed {
                while grapheme_boundaries.peek().is_some_and(|&end| end < offset) {
                    grapheme_boundaries.next();
                }
                if grapheme_boundaries.peek() != Some(&offset) {
                    continue;
                }
            }
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

            let len = new_line_segments
                .iter()
                .map(|segment| segment.text.len())
                .sum();
            if new_line_segments.len() > 1 {
                segment_widths.clear();
                segment_widths.resize(new_line_segments.len(), 0);
                for (start, end) in segmented_graphemes(len, &new_line_segments) {
                    let width =
                        grapheme_text(&new_line_segments, start, end, &mut grapheme_buffer).width();
                    segment_widths[start.0] += width;
                }
                for (segment, &width) in new_line_segments.iter_mut().zip(&segment_widths) {
                    segment.width = width;
                }
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
                let trailing_start = len
                    - new_line_segments
                        .iter()
                        .rev()
                        .flat_map(|segment| segment.text.chars().rev())
                        .take_while(|c| c.is_whitespace())
                        .map(char::len_utf8)
                        .sum::<usize>();
                let mut fragments = Vec::new();
                for (start, end) in segmented_graphemes(len, &new_line_segments) {
                    let (segment_idx, char_idx, _) = end;
                    let grapheme_width =
                        grapheme_text(&new_line_segments, start, end, &mut grapheme_buffer).width();
                    let single;
                    let pieces = if start.0 == segment_idx {
                        single = [new_line_segments[segment_idx].substring(
                            start.1,
                            char_idx,
                            grapheme_width,
                        )];
                        &single[..]
                    } else {
                        fragments.clear();
                        for (i, segment) in new_line_segments
                            .iter()
                            .enumerate()
                            .take(segment_idx + 1)
                            .skip(start.0)
                        {
                            let begin = if i == start.0 { start.1 } else { 0 };
                            let end = if i == segment_idx {
                                char_idx
                            } else {
                                segment.text.len()
                            };
                            if begin < end {
                                let width = if fragments.is_empty() {
                                    grapheme_width
                                } else {
                                    0
                                };
                                fragments.push(segment.substring(begin, end, width));
                            }
                        }
                        &fragments[..]
                    };
                    if start.2 < trailing_start
                        && current_line.width > 0
                        && current_line.width + grapheme_width > width
                    {
                        lines.push(mem::take(&mut current_line));
                    }
                    for piece in pieces {
                        if let Some(previous) =
                            current_line.segments.last_mut().filter(|previous| {
                                previous.index == piece.index
                                    && previous.offset + previous.text.len() == piece.offset
                            })
                        {
                            previous.text = &self.segments[piece.index]
                                [previous.offset..piece.offset + piece.text.len()];
                            previous.width += piece.width;
                            current_line.width += piece.width;
                        } else {
                            current_line.push_segment(piece.clone());
                        }
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
            let has_trailing_newline = last_segment.ends_with(['\r', '\n']);
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
    fn test_soft_wrap_preserves_graphemes_across_segments() {
        type ExpectedRow<'a> = (&'a str, usize, &'a [(usize, usize)]);
        let cases: &[(&[&str], usize, &[ExpectedRow<'_>])] = &[
            (
                &["a ", "\u{301}b"],
                1,
                &[
                    ("a", 1, &[(0, 0)]),
                    (" \u{301}", 1, &[(0, 1), (1, 0)]),
                    ("b", 1, &[(1, 2)]),
                ],
            ),
            (
                &["a ", "\u{301}b"],
                2,
                &[("a \u{301}", 2, &[(0, 0), (1, 0)]), ("b", 1, &[(1, 2)])],
            ),
            (
                &["", "a ", "", "\u{301}b", ""],
                1,
                &[
                    ("a", 1, &[(1, 0)]),
                    (" \u{301}", 1, &[(1, 1), (3, 0)]),
                    ("b", 1, &[(3, 2)]),
                ],
            ),
        ];
        for &(segments, width, expected) in cases {
            let value: SegmentedString = segments.iter().copied().collect();
            let lines = value.wrap(width);
            let rows: Vec<_> = lines
                .iter()
                .map(|line| {
                    (
                        line.to_string(),
                        line.width,
                        line.segments
                            .iter()
                            .map(|segment| (segment.index, segment.offset))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            let expected: Vec<_> = expected
                .iter()
                .map(|&(text, width, regions)| (text.to_string(), width, regions.to_vec()))
                .collect();
            assert_eq!(rows, expected, "{segments:?} at width {width}");
        }
    }

    #[test]
    fn test_split_zwj_graphemes_use_one_width_budget() {
        type ExpectedRow<'a> = (&'a str, usize);
        let cases: &[(&[&str], usize, &[ExpectedRow<'_>])] = &[
            (&["👩", "\u{200d}💻b"], 3, &[("👩‍💻b", 3)]),
            (&["👩", "\u{200d}💻b"], 2, &[("👩‍💻", 2), ("b", 1)]),
            (&["a ", "👩", "\u{200d}💻b"], 5, &[("a 👩‍💻b", 5)]),
        ];
        for &(segments, width, expected) in cases {
            let value: SegmentedString = segments.iter().copied().collect();
            let lines = value.wrap(width);
            assert_eq!(lines.len(), expected.len(), "{segments:?} at width {width}");
            for (line, &(text, columns)) in lines.iter().zip(expected) {
                assert_eq!(line.to_string(), text);
                assert_eq!(line.width, columns);
            }
        }
    }

    #[test]
    fn test_forced_wrap_keeps_split_regional_indicators_together() {
        let value: SegmentedString = ["🇺", "🇸 ", "\u{301}x"].into_iter().collect();
        let rows: Vec<_> = value.wrap(1).iter().map(ToString::to_string).collect();
        assert_eq!(rows, ["🇺🇸", " \u{301}", "x"]);
    }

    #[test]
    fn test_forced_wrap_keeps_a_joined_emoji_intact() {
        let text = SegmentedString::from("a👩‍💻b");
        let lines = text.wrap(1);
        assert_eq!(
            lines.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["a", "👩‍💻", "b"],
        );
        assert_eq!(lines[1].width, 2);
        assert_eq!(lines[1].segments[0].offset, 1);
        assert_eq!(lines[2].segments[0].offset, 12);
    }

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
