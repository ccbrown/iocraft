use std::sync::LazyLock;
use regex::Regex;

pub const ANSI_REGEX_PATTERN: &str = concat!(
    // OSC branch
    "(?:\\x1B\\][^\\x07\\x1B\\x9C]*?(?:\\x07|\\x1B\\\\|\\x9C))",
    "|",
    // CSI ESC[ ...
    "(?:\\x1B\\[[\\[\\]()#;?]*(?:[0-9]{1,4}(?:[;:][0-9]{0,4})*)?[0-9A-PR-TZcf-nq-uy=><~])",
    "|",
    // CSI single-byte 0x9B ...
    "(?:\\x9B[\\[\\]()#;?]*(?:[0-9]{1,4}(?:[;:][0-9]{0,4})*)?[0-9A-PR-TZcf-nq-uy=><~])",
    "|",
    // VT52 / short escapes (single final)
    // Added E (NEL), M (RI), c (reset), m (SGR reset), plus existing cursor & mode keys.
    "(?:\\x1B[ABCDHIKJSTZ=><sum78EMcNO])",
    "|",
    // Charset selection ESC (X or )X where X in A B 0 1 2
    "(?:\\x1B[()][AB012])",
    "|",
    // Hash sequences ESC # 3 4 5 6 8
    "(?:\\x1B#[34568])",
    "|",
    // Device status reports / queries: ESC [ 5 n etc (already covered by CSI) but bare 'ESC 5 n' appears in fixtures => add generic ESC [0-9]+[n] pattern fallback
    "(?:\\x1B[0-9]+n)"
);

static ANSI_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(ANSI_REGEX_PATTERN).expect("valid ANSI regex"));

pub(crate) fn strip_ansi(string: &str) -> String {
    ANSI_REGEX.replace_all(string, "").to_string()
}
