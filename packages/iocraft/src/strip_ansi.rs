use regex::Regex;
use std::{borrow::Cow, sync::LazyLock};

pub const ANSI_REGEX_PATTERN: &str = concat!(
    // OSC (Operating System Command): ESC ] ... terminated by BEL, ST (ESC \), or 0x9C.
    "(?:\\x1B\\][^\\x07\\x1B\\x9C]*?(?:\\x07|\\x1B\\\\|\\x9C))",
    "|",
    // String sequences DCS (P), APC (_), PM (^), SOS (X): ESC <intro> ... terminated by
    // ST (ESC \ or 0x9C).  Mirrors OSC but without BEL as a terminator.
    "(?:\\x1B[P_^X][^\\x1B\\x9C]*?(?:\\x1B\\\\|\\x9C))",
    "|",
    // CSI (Control Sequence Introducer), 7-bit form.  Per ECMA-48 the grammar is
    //   ESC '[' (parameter bytes 0x30-0x3F)* (intermediate bytes 0x20-0x2F)* (final byte 0x40-0x7E)
    "(?:\\x1B\\[[\\x30-\\x3F]*[\\x20-\\x2F]*[\\x40-\\x7E])",
    "|",
    // CSI, 8-bit form introduced by 0x9B.
    "(?:\\x9B[\\x30-\\x3F]*[\\x20-\\x2F]*[\\x40-\\x7E])",
    "|",
    // VT52 / short escapes (single final byte).
    "(?:\\x1B[ABCDHIKJSTZ=><su78EMcNO])",
    "|",
    // Charset selection: ESC ( X or ESC ) X where X in A B 0 1 2.
    "(?:\\x1B[()][AB012])",
    "|",
    // Hash sequences: ESC # 3 4 5 6 8.
    "(?:\\x1B#[34568])"
);

static ANSI_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(ANSI_REGEX_PATTERN).expect("valid ANSI regex"));

pub(crate) fn strip_ansi(string: &str) -> Cow<'_, str> {
    ANSI_REGEX.replace_all(string, "")
}

#[cfg(test)]
mod tests {
    use super::strip_ansi;

    fn stripped(input: &str) -> String {
        strip_ansi(input).into_owned()
    }

    // ── Regression guards: common sequences that must keep working ──

    #[test]
    fn strips_basic_sgr() {
        assert_eq!(stripped("\x1b[31mhi\x1b[0m"), "hi");
    }

    #[test]
    fn strips_truecolor_sgr() {
        assert_eq!(stripped("\x1b[38;2;255;0;0mhi\x1b[0m"), "hi");
    }

    #[test]
    fn strips_bare_sgr_reset() {
        assert_eq!(stripped("\x1b[mhi"), "hi");
    }

    #[test]
    fn strips_osc8_hyperlink() {
        assert_eq!(stripped("\x1b]8;;http://x.com\x07link\x1b]8;;\x07"), "link");
    }

    #[test]
    fn strips_osc_with_st_terminator() {
        assert_eq!(stripped("\x1b]0;title\x1b\\hi"), "hi");
    }

    #[test]
    fn strips_cursor_position() {
        assert_eq!(stripped("\x1b[10;20Hhi"), "hi");
    }

    #[test]
    fn strips_bracketed_paste() {
        assert_eq!(stripped("\x1b[?2004hhi"), "hi");
    }

    #[test]
    fn strips_csi_private_mode() {
        assert_eq!(stripped("\x1b[?1049h"), "");
    }

    #[test]
    fn leaves_lone_esc_untouched() {
        assert_eq!(stripped("a\x1bb"), "a\x1bb");
    }

    // ── Bug 1: CSI final byte must not accept parameter digits ──
    // `\x1b[99999m` should strip whole; the broken class terminated on the
    // 5th digit and leaked the real final byte `m` as literal text.

    #[test]
    fn strips_long_parameter_sgr() {
        assert_eq!(stripped("\x1b[99999mhi"), "hi");
    }

    // ── Bug 2: DCS / APC / PM string sequences must be stripped ──

    #[test]
    fn strips_dcs_sequence() {
        assert_eq!(stripped("\x1bPq#0;2\x1b\\hi"), "hi");
    }

    #[test]
    fn strips_apc_sequence() {
        assert_eq!(stripped("\x1b_Gf=100\x1b\\hi"), "hi");
    }
}
