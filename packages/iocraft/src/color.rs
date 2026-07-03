use std::fmt;
use std::sync::OnceLock;

/// Builds a CSI (Control Sequence Introducer) escape string at compile time,
/// e.g. `csi!("0m")` produces `"\x1b[0m"`. Mirrors crossterm's `csi!` so call
/// sites read the same, without depending on crossterm.
macro_rules! csi {
    ($( $l:literal ),*) => { concat!("\x1b[", $( $l ),*) };
}
pub(crate) use csi;

/// A color that can be applied to text or backgrounds.
///
/// The named variants map to the standard 16 terminal colors; [`Color::Rgb`]
/// and [`Color::AnsiValue`] allow true-color and 256-color selection where the
/// terminal supports it.
///
/// This mirrors the color model used by common terminal libraries, but is
/// owned by iocraft so that rendering backends do not have to depend on any
/// particular one. When the `crossterm` feature is enabled, `From`/`Into`
/// conversions to and from [`crossterm::style::Color`] are provided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Color {
    /// Resets the color to the terminal's default.
    Reset,
    /// Black.
    Black,
    /// Dark grey.
    DarkGrey,
    /// Light red.
    Red,
    /// Dark red.
    DarkRed,
    /// Light green.
    Green,
    /// Dark green.
    DarkGreen,
    /// Light yellow.
    Yellow,
    /// Dark yellow.
    DarkYellow,
    /// Light blue.
    Blue,
    /// Dark blue.
    DarkBlue,
    /// Light magenta.
    Magenta,
    /// Dark magenta.
    DarkMagenta,
    /// Light cyan.
    Cyan,
    /// Dark cyan.
    DarkCyan,
    /// White.
    White,
    /// Grey.
    Grey,
    /// A 24-bit RGB color. Supported by most modern terminals.
    Rgb {
        /// The red channel.
        r: u8,
        /// The green channel.
        g: u8,
        /// The blue channel.
        b: u8,
    },
    /// An 8-bit indexed color from the 256-color palette.
    AnsiValue(u8),
}

impl TryFrom<&str> for Color {
    type Error = ();

    /// Tries to create a `Color` from a name like `"red"` or `"dark_grey"`
    /// (case-insensitive), matching the names crossterm accepts. Returns an
    /// error if the string does not match a named color.
    fn try_from(src: &str) -> Result<Self, Self::Error> {
        match src.to_lowercase().as_str() {
            "reset" => Ok(Color::Reset),
            "black" => Ok(Color::Black),
            "dark_grey" => Ok(Color::DarkGrey),
            "red" => Ok(Color::Red),
            "dark_red" => Ok(Color::DarkRed),
            "green" => Ok(Color::Green),
            "dark_green" => Ok(Color::DarkGreen),
            "yellow" => Ok(Color::Yellow),
            "dark_yellow" => Ok(Color::DarkYellow),
            "blue" => Ok(Color::Blue),
            "dark_blue" => Ok(Color::DarkBlue),
            "magenta" => Ok(Color::Magenta),
            "dark_magenta" => Ok(Color::DarkMagenta),
            "cyan" => Ok(Color::Cyan),
            "dark_cyan" => Ok(Color::DarkCyan),
            "white" => Ok(Color::White),
            "grey" => Ok(Color::Grey),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for Color {
    type Err = ();

    /// Creates a `Color` from a name like `"red"`. Unknown names fall back to
    /// [`Color::White`], matching crossterm's behavior.
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(Color::try_from(src).unwrap_or(Color::White))
    }
}

impl From<(u8, u8, u8)> for Color {
    /// Creates a [`Color::Rgb`] from an `(r, g, b)` tuple.
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Color::Rgb { r, g, b }
    }
}

/// SGR parameters for the attributes iocraft emits, matching the codes standard
/// terminals (and crossterm) use, so canvas output stays byte-for-byte stable.
pub(crate) mod sgr {
    /// Bold / increased intensity.
    pub const BOLD: u8 = 1;
    /// Dim / decreased intensity.
    pub const DIM: u8 = 2;
    /// Italic.
    pub const ITALIC: u8 = 3;
    /// Underline.
    pub const UNDERLINED: u8 = 4;
    /// Reverse video (swap foreground and background).
    pub const REVERSE: u8 = 7;
}

/// Whether color output is disabled via the `NO_COLOR` environment variable
/// (see <https://no-color.org/>). The variable is read once and memoized, so
/// this stays cheap on the per-cell render path. Matches crossterm, which
/// applies the same suppression inside `Colored`'s `Display`.
fn color_output_disabled() -> bool {
    static DISABLED: OnceLock<bool> = OnceLock::new();
    *DISABLED.get_or_init(|| std::env::var("NO_COLOR").map(|v| !v.is_empty()).unwrap_or(false))
}

/// Wraps a [`Color`] so it renders as the SGR parameters for a foreground or
/// background color, e.g. `38;5;9` or `49`. Used with `write!` inside a
/// `CSI … m` sequence. The encoding is identical to what crossterm emits,
/// including emitting nothing when `NO_COLOR` is set.
pub(crate) enum SgrColor {
    Foreground(Color),
    Background(Color),
}

impl SgrColor {
    /// Writes the SGR parameters, or nothing when `disabled` is true (the
    /// `NO_COLOR` case). Split out from [`fmt::Display`] so it can be tested
    /// without touching the process-wide memoized flag.
    fn write_params(&self, f: &mut impl fmt::Write, disabled: bool) -> fmt::Result {
        if disabled {
            return Ok(());
        }
        let (color, reset, prefix) = match *self {
            SgrColor::Foreground(c) => (c, "39", "38;"),
            SgrColor::Background(c) => (c, "49", "48;"),
        };
        if color == Color::Reset {
            return f.write_str(reset);
        }
        f.write_str(prefix)?;
        match color {
            Color::Black => f.write_str("5;0"),
            Color::DarkGrey => f.write_str("5;8"),
            Color::Red => f.write_str("5;9"),
            Color::DarkRed => f.write_str("5;1"),
            Color::Green => f.write_str("5;10"),
            Color::DarkGreen => f.write_str("5;2"),
            Color::Yellow => f.write_str("5;11"),
            Color::DarkYellow => f.write_str("5;3"),
            Color::Blue => f.write_str("5;12"),
            Color::DarkBlue => f.write_str("5;4"),
            Color::Magenta => f.write_str("5;13"),
            Color::DarkMagenta => f.write_str("5;5"),
            Color::Cyan => f.write_str("5;14"),
            Color::DarkCyan => f.write_str("5;6"),
            Color::White => f.write_str("5;15"),
            Color::Grey => f.write_str("5;7"),
            Color::Rgb { r, g, b } => write!(f, "2;{r};{g};{b}"),
            Color::AnsiValue(v) => write!(f, "5;{v}"),
            Color::Reset => unreachable!("Reset returned early above"),
        }
    }
}

impl fmt::Display for SgrColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_params(f, color_output_disabled())
    }
}

#[cfg(feature = "crossterm")]
impl From<Color> for crossterm::style::Color {
    fn from(c: Color) -> Self {
        use crossterm::style::Color as Ct;
        match c {
            Color::Reset => Ct::Reset,
            Color::Black => Ct::Black,
            Color::DarkGrey => Ct::DarkGrey,
            Color::Red => Ct::Red,
            Color::DarkRed => Ct::DarkRed,
            Color::Green => Ct::Green,
            Color::DarkGreen => Ct::DarkGreen,
            Color::Yellow => Ct::Yellow,
            Color::DarkYellow => Ct::DarkYellow,
            Color::Blue => Ct::Blue,
            Color::DarkBlue => Ct::DarkBlue,
            Color::Magenta => Ct::Magenta,
            Color::DarkMagenta => Ct::DarkMagenta,
            Color::Cyan => Ct::Cyan,
            Color::DarkCyan => Ct::DarkCyan,
            Color::White => Ct::White,
            Color::Grey => Ct::Grey,
            Color::Rgb { r, g, b } => Ct::Rgb { r, g, b },
            Color::AnsiValue(v) => Ct::AnsiValue(v),
        }
    }
}

#[cfg(feature = "crossterm")]
impl From<crossterm::style::Color> for Color {
    fn from(c: crossterm::style::Color) -> Self {
        use crossterm::style::Color as Ct;
        match c {
            Ct::Reset => Color::Reset,
            Ct::Black => Color::Black,
            Ct::DarkGrey => Color::DarkGrey,
            Ct::Red => Color::Red,
            Ct::DarkRed => Color::DarkRed,
            Ct::Green => Color::Green,
            Ct::DarkGreen => Color::DarkGreen,
            Ct::Yellow => Color::Yellow,
            Ct::DarkYellow => Color::DarkYellow,
            Ct::Blue => Color::Blue,
            Ct::DarkBlue => Color::DarkBlue,
            Ct::Magenta => Color::Magenta,
            Ct::DarkMagenta => Color::DarkMagenta,
            Ct::Cyan => Color::Cyan,
            Ct::DarkCyan => Color::DarkCyan,
            Ct::White => Color::White,
            Ct::Grey => Color::Grey,
            Ct::Rgb { r, g, b } => Color::Rgb { r, g, b },
            Ct::AnsiValue(v) => Color::AnsiValue(v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_color_names() {
        assert_eq!(Color::try_from("red"), Ok(Color::Red));
        assert_eq!(Color::try_from("DARK_GREY"), Ok(Color::DarkGrey));
        assert!(Color::try_from("not_a_color").is_err());
        assert_eq!("dark_magenta".parse::<Color>(), Ok(Color::DarkMagenta));
        // Unknown names fall back to white, matching crossterm.
        assert_eq!("nope".parse::<Color>(), Ok(Color::White));
        assert_eq!(Color::from((1, 2, 3)), Color::Rgb { r: 1, g: 2, b: 3 });
    }

    #[test]
    fn foreground_sgr_matches_expected() {
        assert_eq!(SgrColor::Foreground(Color::Reset).to_string(), "39");
        assert_eq!(SgrColor::Foreground(Color::Red).to_string(), "38;5;9");
        assert_eq!(SgrColor::Foreground(Color::Grey).to_string(), "38;5;7");
        assert_eq!(
            SgrColor::Foreground(Color::Rgb { r: 1, g: 2, b: 3 }).to_string(),
            "38;2;1;2;3"
        );
        assert_eq!(
            SgrColor::Foreground(Color::AnsiValue(200)).to_string(),
            "38;5;200"
        );
    }

    #[test]
    fn background_sgr_matches_expected() {
        assert_eq!(SgrColor::Background(Color::Reset).to_string(), "49");
        assert_eq!(SgrColor::Background(Color::Red).to_string(), "48;5;9");
        assert_eq!(
            SgrColor::Background(Color::Rgb { r: 4, g: 5, b: 6 }).to_string(),
            "48;2;4;5;6"
        );
    }

    #[test]
    fn no_color_suppresses_sgr_output() {
        // With NO_COLOR in effect, no SGR parameters are emitted (matching
        // crossterm), so the surrounding `CSI … m` collapses to a plain reset.
        // Without it, the normal parameters are produced.
        for color in [Color::Red, Color::Reset, Color::Rgb { r: 1, g: 2, b: 3 }] {
            for sgr in [SgrColor::Foreground(color), SgrColor::Background(color)] {
                let mut disabled = String::new();
                sgr.write_params(&mut disabled, true).unwrap();
                assert_eq!(disabled, "", "expected no output for {color:?} when disabled");

                let mut enabled = String::new();
                sgr.write_params(&mut enabled, false).unwrap();
                assert!(
                    !enabled.is_empty(),
                    "expected SGR params for {color:?} when enabled"
                );
            }
        }
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn crossterm_roundtrip_and_output_parity() {
        let colors = [
            Color::Reset,
            Color::Black,
            Color::DarkGrey,
            Color::Red,
            Color::DarkRed,
            Color::Green,
            Color::DarkGreen,
            Color::Yellow,
            Color::DarkYellow,
            Color::Blue,
            Color::DarkBlue,
            Color::Magenta,
            Color::DarkMagenta,
            Color::Cyan,
            Color::DarkCyan,
            Color::White,
            Color::Grey,
            Color::Rgb {
                r: 10,
                g: 20,
                b: 30,
            },
            Color::AnsiValue(123),
        ];
        for c in colors {
            let ct: crossterm::style::Color = c.into();
            assert_eq!(Color::from(ct), c, "roundtrip failed for {c:?}");
            // Output must match crossterm's Colored exactly.
            assert_eq!(
                SgrColor::Foreground(c).to_string(),
                crossterm::style::Colored::ForegroundColor(ct).to_string(),
                "foreground SGR mismatch for {c:?}"
            );
            assert_eq!(
                SgrColor::Background(c).to_string(),
                crossterm::style::Colored::BackgroundColor(ct).to_string(),
                "background SGR mismatch for {c:?}"
            );
        }
    }
}
