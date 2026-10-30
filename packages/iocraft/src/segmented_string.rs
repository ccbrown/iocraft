use crate::unicode_linebreak::{linebreaks_iter, BreakOpportunity};
use core::fmt::{self, Display};
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Returns the number of columns `text` occupies when each grapheme is drawn separately.
// XXX: This can exceed `str::width`, which narrows some sequences that span graphemes, such as
// Lam-Alef. The canvas draws graphemes separately, so layout must measure them that way too.
pub(crate) fn grapheme_width(text: &str) -> usize {
    text.graphemes(true).map(UnicodeWidthStr::width).sum()
}

pub(crate) fn is_whitespace_grapheme(grapheme: &str) -> bool {
    grapheme.chars().all(char::is_whitespace)
}

// The characters that force a line break (classes BK, CR, LF, and NL in UAX #14).
pub(crate) const HARD_LINE_BREAKS: [char; 7] = [
    '\n', '\r', '\u{b}', '\u{c}', '\u{85}', '\u{2028}', '\u{2029}',
];

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

impl SegmentedStringLine<'_> {
    fn join<'b>(&self, buffer: &'b mut String) -> &'b str {
        buffer.clear();
        for segment in &self.segments {
            buffer.push_str(segment.text);
        }
        buffer
    }

    // Iterates over the graphemes of `text`, which must be the joined segment text, along with the
    // position of the segment each grapheme starts in.
    fn graphemes<'b>(&'b self, text: &'b str) -> impl Iterator<Item = (usize, usize, &'b str)> {
        let mut segment = 0;
        let mut segment_end = 0;
        text.grapheme_indices(true).map(move |(start, grapheme)| {
            while start >= segment_end {
                segment_end += self.segments[segment].text.len();
                segment += 1;
            }
            (segment - 1, start, grapheme)
        })
    }

    /// Calls `f` with consecutive spans of the line's text, along with the index of the segment each
    /// span came from. A grapheme spanning multiple segments is included whole in the span of the
    /// segment it starts in.
    pub(crate) fn for_each_grapheme_span(
        &self,
        buffer: &mut String,
        mut f: impl FnMut(&str, usize),
    ) {
        if let [segment] = self.segments.as_slice() {
            f(segment.text, segment.index);
            return;
        }
        let text = self.join(buffer);
        let mut span: Option<(usize, usize)> = None;
        for (segment, start, _) in self.graphemes(text) {
            match span {
                Some((span_segment, _)) if span_segment == segment => {}
                _ => {
                    if let Some((span_segment, span_start)) = span {
                        f(&text[span_start..start], self.segments[span_segment].index);
                    }
                    span = Some((segment, start));
                }
            }
        }
        if let Some((span_segment, span_start)) = span {
            f(&text[span_start..], self.segments[span_segment].index);
        }
    }

    /// Removes trailing whitespace from the line.
    pub fn trim_end(&mut self, buffer: &mut String) {
        let len = self.len();
        while let Some(segment) = self.segments.last_mut() {
            segment.text = segment.text.trim_end();
            if !segment.text.is_empty() {
                break;
            }
            self.segments.pop();
        }
        if self.len() != len {
            self.width = match self.segments.as_slice() {
                [segment] => grapheme_width(segment.text),
                _ => grapheme_width(self.join(buffer)),
            };
        }
    }

    fn len(&self) -> usize {
        self.segments.iter().map(|segment| segment.text.len()).sum()
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
}

impl Display for SegmentedStringLineSegment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

fn is_hard_line_break(grapheme: &str) -> bool {
    !grapheme.is_empty() && grapheme.chars().all(|c| HARD_LINE_BREAKS.contains(&c))
}

struct Grapheme {
    start: usize,
    end: usize,
    width: usize,
    is_whitespace: bool,
}

// A line under construction, as a byte range of the joined text.
#[derive(Clone, Copy)]
struct LineRange {
    start: usize,
    end: usize,
    width: usize,
}

impl LineRange {
    fn at(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
            width: 0,
        }
    }
}

impl<'a> SegmentedString<'a> {
    /// Wraps the string into lines of a given width without splitting extended grapheme clusters.
    pub fn wrap(&self, width: usize) -> Vec<SegmentedStringLine<'_>> {
        if self.segments.is_empty() {
            return vec![];
        }

        // Graphemes and line break rules can span segments, so wrap the joined text, then map each
        // line back onto the segments.
        let text: Cow<'_, str> = match self.segments.as_slice() {
            [segment] => Cow::Borrowed(segment),
            segments => Cow::Owned(segments.concat()),
        };
        let graphemes: Vec<Grapheme> = text
            .grapheme_indices(true)
            .map(|(start, grapheme)| Grapheme {
                start,
                end: start + grapheme.len(),
                width: grapheme.width(),
                is_whitespace: is_whitespace_grapheme(grapheme),
            })
            .collect();

        let mut ranges = Vec::new();
        let mut line = LineRange::at(0);
        let mut chunk_start = 0;
        let mut chunk_graphemes_start = 0;
        for (offset, opportunity) in linebreaks_iter(text.char_indices(), text.len()) {
            let chunk_graphemes_end = chunk_graphemes_start
                + graphemes[chunk_graphemes_start..].partition_point(|g| g.start < offset);
            let on_grapheme_boundary = graphemes
                .get(chunk_graphemes_end)
                .is_none_or(|g| g.start == offset);
            if opportunity == BreakOpportunity::Allowed && !on_grapheme_boundary {
                continue;
            }

            let mut chunk = &graphemes[chunk_graphemes_start..chunk_graphemes_end];
            if opportunity == BreakOpportunity::Mandatory {
                if let Some((last, rest)) = chunk.split_last() {
                    if is_hard_line_break(&text[last.start..last.end]) {
                        chunk = rest;
                    }
                }
            }
            let chunk_end = chunk.last().map_or(chunk_start, |g| g.end);
            let chunk_width: usize = chunk.iter().map(|g| g.width).sum();
            let trailing_whitespace = chunk.iter().rev().take_while(|g| g.is_whitespace).count();
            let trailing_whitespace_width: usize = chunk[chunk.len() - trailing_whitespace..]
                .iter()
                .map(|g| g.width)
                .sum();

            if line.width + chunk_width - trailing_whitespace_width <= width {
                // Everything fits into the current line
                line.end = chunk_end;
                line.width += chunk_width;
            } else {
                // Break if necessary, then add more lines
                if line.width > 0 {
                    ranges.push(line);
                    line = LineRange::at(chunk_start);
                }
                for (i, g) in chunk.iter().enumerate() {
                    if i < chunk.len() - trailing_whitespace
                        && line.width > 0
                        && line.width + g.width > width
                    {
                        ranges.push(line);
                        line = LineRange::at(g.start);
                    }
                    line.end = g.end;
                    line.width += g.width;
                }
            }

            if opportunity == BreakOpportunity::Mandatory {
                // We have to break here
                ranges.push(line);
                line = LineRange::at(offset);
            }
            chunk_start = offset;
            chunk_graphemes_start = chunk_graphemes_end;
        }

        // Add another line if the text ends with a line break. Empty text is also a single line.
        if ranges.is_empty() || text.ends_with(HARD_LINE_BREAKS) {
            ranges.push(LineRange::at(text.len()));
        }

        self.lines_for_ranges(&ranges)
    }

    fn lines_for_ranges(&self, ranges: &[LineRange]) -> Vec<SegmentedStringLine<'a>> {
        let segment_starts: Vec<usize> = self
            .segments
            .iter()
            .scan(0, |start, segment| {
                let segment_start = *start;
                *start += segment.len();
                Some(segment_start)
            })
            .collect();
        let segment_end = |index: usize| segment_starts[index] + self.segments[index].len();

        let mut segment = 0;
        ranges
            .iter()
            .map(|range| {
                while segment + 1 < self.segments.len() && segment_end(segment) <= range.start {
                    segment += 1;
                }
                let mut line = SegmentedStringLine {
                    segments: vec![],
                    width: range.width,
                };
                for (index, (&segment_start, text)) in segment_starts
                    .iter()
                    .zip(&self.segments)
                    .enumerate()
                    .skip(segment)
                {
                    if segment_start >= range.end {
                        break;
                    }
                    let start = range.start.max(segment_start) - segment_start;
                    let end = range.end.min(segment_start + text.len()) - segment_start;
                    if start < end {
                        line.segments.push(SegmentedStringLineSegment {
                            text: &text[start..end],
                            index,
                            offset: start,
                        });
                    }
                }
                if line.segments.is_empty() {
                    let offset = range.start - segment_starts[segment];
                    line.segments.push(SegmentedStringLineSegment {
                        text: &self.segments[segment][offset..offset],
                        index: segment,
                        offset,
                    });
                }
                line
            })
            .collect()
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
    fn test_trimmed_split_grapheme_widths() {
        let value: SegmentedString = ["\u{600}", " "].into_iter().collect();
        let mut line = value.wrap(4).pop().unwrap();
        line.trim_end(&mut String::new());
        assert_eq!(line.to_string(), "\u{600}");
        assert_eq!(line.width, "\u{600}".width());
    }

    #[test]
    fn test_empty_text_is_one_line() {
        for value in [SegmentedString::from(""), ["", ""].into_iter().collect()] {
            let lines = value.wrap(10);
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0].segments.len(), 1);
            assert_eq!(lines[0].width, 0);
        }
        assert!(SegmentedString::default().wrap(10).is_empty());
    }

    #[test]
    fn test_ligature_wraps_the_same_when_split_across_segments() {
        for text in ["لا", "\u{5d0}\u{200d}\u{5dc}"] {
            let parts: Vec<String> = text.chars().map(String::from).collect();
            let whole: SegmentedString = [text].into_iter().collect();
            let split: SegmentedString = parts.iter().map(String::as_str).collect();
            for width in [1, 2] {
                let lines = |s: &SegmentedString| {
                    s.wrap(width)
                        .iter()
                        .map(|line| (line.to_string(), line.width))
                        .collect::<Vec<_>>()
                };
                assert_eq!(lines(&split), lines(&whole), "{text:?} at width {width}");
            }
        }
    }

    #[test]
    fn test_hard_line_breaks_are_not_part_of_lines() {
        for separator in [
            "\n", "\r", "\r\n", "\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}",
        ] {
            let text = format!("ab{separator}cd{separator}");
            let value = SegmentedString::from(text.as_str());
            let lines = value.wrap(10);
            let rows: Vec<_> = lines
                .iter()
                .map(|line| (line.to_string(), line.segments[0].offset))
                .collect();
            let next = 2 + separator.len();
            assert_eq!(
                rows,
                [
                    ("ab".to_string(), 0),
                    ("cd".to_string(), next),
                    (String::new(), text.len())
                ],
                "{separator:?}"
            );
        }
    }

    #[test]
    fn test_grapheme_spans_follow_segments() {
        for (segments, expected) in [
            (&["ab", "cd"][..], &[("ab", 0), ("cd", 1)][..]),
            (&["ab", "", "cd"], &[("ab", 0), ("cd", 2)]),
            (&["👩", "\u{200d}💻b"], &[("👩\u{200d}💻", 0), ("b", 1)]),
            (&["a", "\u{301}"], &[("a\u{301}", 0)]),
        ] {
            let value: SegmentedString = segments.iter().copied().collect();
            let line = value.wrap(usize::MAX).pop().unwrap();
            let mut spans = Vec::new();
            line.for_each_grapheme_span(&mut String::new(), |text, index| {
                spans.push((text.to_string(), index));
            });
            let expected: Vec<_> = expected
                .iter()
                .map(|&(text, index)| (text.to_string(), index))
                .collect();
            assert_eq!(spans, expected, "{segments:?}");
        }
    }

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
                        },],
                        width: 7,
                    },
                    SegmentedStringLine {
                        segments: vec![SegmentedStringLineSegment {
                            text: "world! This ",
                            index: 0,
                            offset: 7,
                        },],
                        width: 12,
                    },
                    SegmentedStringLine {
                        segments: vec![SegmentedStringLineSegment {
                            text: "is a test ",
                            index: 0,
                            offset: 19,
                        },],
                        width: 10,
                    },
                    SegmentedStringLine {
                        segments: vec![SegmentedStringLineSegment {
                            text: "string.",
                            index: 0,
                            offset: 29,
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
