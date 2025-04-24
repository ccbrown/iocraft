#![allow(dead_code)]
#![deny(missing_docs, missing_debug_implementations)]

use core::iter::once;

/// The [Unicode version](https://www.unicode.org/versions/) conformed to.
pub const UNICODE_VERSION: (u8, u8, u8) = (15, 0, 0);

include!("shared.rs");
include!("tables.rs");

#[inline(always)]
pub fn break_property(codepoint: u32) -> BreakClass {
    const BMP_INDEX_LENGTH: u32 = BMP_LIMIT >> BMP_SHIFT;
    const OMITTED_BMP_INDEX_1_LENGTH: u32 = BMP_LIMIT >> SHIFT_1;

    let data_pos = if codepoint < BMP_LIMIT {
        let i = codepoint >> BMP_SHIFT;
        BREAK_PROP_TRIE_INDEX[i as usize] + (codepoint & (BMP_DATA_BLOCK_LENGTH - 1)) as u16
    } else if codepoint < BREAK_PROP_TRIE_HIGH_START {
        let i1 = codepoint >> SHIFT_1;
        let i2 = BREAK_PROP_TRIE_INDEX
            [(i1 + BMP_INDEX_LENGTH - OMITTED_BMP_INDEX_1_LENGTH) as usize]
            + ((codepoint >> SHIFT_2) & (INDEX_2_BLOCK_LENGTH - 1)) as u16;
        let i3_block = BREAK_PROP_TRIE_INDEX[i2 as usize];
        let i3_pos = ((codepoint >> SHIFT_3) & (INDEX_3_BLOCK_LENGTH - 1)) as u16;

        debug_assert!(i3_block & 0x8000 == 0, "18-bit indices are unexpected");
        let data_block = BREAK_PROP_TRIE_INDEX[(i3_block + i3_pos) as usize];
        data_block + (codepoint & (SMALL_DATA_BLOCK_LENGTH - 1)) as u16
    } else {
        return XX;
    };
    BREAK_PROP_TRIE_DATA[data_pos as usize]
}

/// Break opportunity type.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BreakOpportunity {
    /// A line must break at this spot.
    Mandatory,
    /// A line is allowed to end at this spot.
    Allowed,
}

pub fn linebreaks(s: &str) -> impl Iterator<Item = (usize, BreakOpportunity)> + Clone + '_ {
    linebreaks_iter(s.char_indices(), s.len())
}

/// Returns an iterator over line breaks opportunities in the specified input iterator
///
/// This is identical to `linebreaks()` but provides the ability to find line breaks
/// across any iterable set of characters.  Indexes are also factored out
pub fn linebreaks_iter<'a, N>(
    iter: impl Iterator<Item = (N, char)> + Clone + 'a,
    final_idx: N,
) -> impl Iterator<Item = (N, BreakOpportunity)> + Clone + 'a
where
    N: Clone + 'a,
{
    use BreakOpportunity::{Allowed, Mandatory};

    iter.map(|(i, c)| (i, break_property(c as u32) as u8))
        .chain(once((final_idx, eot)))
        .scan((sot, false), |state, (i, cls)| {
            // ZWJ is handled outside the table to reduce its size
            let val = PAIR_TABLE[state.0 as usize][cls as usize];
            let is_mandatory = val & MANDATORY_BREAK_BIT != 0;
            let is_break = val & ALLOWED_BREAK_BIT != 0 && (!state.1 || is_mandatory);
            *state = (
                val & !(ALLOWED_BREAK_BIT | MANDATORY_BREAK_BIT),
                cls == BreakClass::ZeroWidthJoiner as u8,
            );

            Some((i, is_break, is_mandatory))
        })
        .filter_map(|(i, is_break, is_mandatory)| {
            if is_break {
                Some((i, if is_mandatory { Mandatory } else { Allowed }))
            } else {
                None
            }
        })
}

pub fn split_at_safe(s: &str) -> (&str, &str) {
    let mut chars = s.char_indices().rev().scan(None, |state, (i, c)| {
        let cls = break_property(c as u32);
        let is_safe_pair = state
            .replace(cls).is_some_and(|prev| is_safe_pair(cls, prev)); // Reversed since iterating backwards
        Some((i, is_safe_pair))
    });
    chars.find(|&(_, is_safe_pair)| is_safe_pair);
    // Include preceding char for `linebreaks` to pick up break before match (disallowed after sot)
    s.split_at(chars.next().map_or(0, |(i, _)| i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(break_property(0xA), BreakClass::LineFeed);
        assert_eq!(break_property(0xDB80), BreakClass::Surrogate);
        assert_eq!(break_property(0xe01ef), BreakClass::CombiningMark);
        assert_eq!(break_property(0x10ffff), BreakClass::Unknown);
    }
}
