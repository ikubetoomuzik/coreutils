use std::str::FromStr;

use crate::units::Unit;
use uucore::ranges::Range;

pub const DELIMITER: &str = "delimiter";
pub const FIELD: &str = "field";
pub const FIELD_DEFAULT: &str = "1";
pub const FORMAT: &str = "format";
pub const FROM: &str = "from";
pub const FROM_DEFAULT: &str = "none";
pub const FROM_UNIT: &str = "from-unit";
pub const FROM_UNIT_DEFAULT: &str = "1";
pub const HEADER: &str = "header";
pub const HEADER_DEFAULT: &str = "1";
pub const NUMBER: &str = "NUMBER";
pub const PADDING: &str = "padding";
pub const ROUND: &str = "round";
pub const SUFFIX: &str = "suffix";
pub const TO: &str = "to";
pub const TO_DEFAULT: &str = "none";
pub const TO_UNIT: &str = "to-unit";
pub const TO_UNIT_DEFAULT: &str = "1";

pub struct TransformOptions {
    pub from: Unit,
    pub from_unit: usize,
    pub to: Unit,
    pub to_unit: usize,
}

pub struct NumfmtOptions {
    pub transform: TransformOptions,
    pub padding: isize,
    pub header: usize,
    pub fields: Vec<Range>,
    pub delimiter: Option<String>,
    pub round: RoundMethod,
    pub suffix: Option<String>,
    pub format: FormatOptions,
}

#[derive(Clone, Copy)]
pub enum RoundMethod {
    Up,
    Down,
    FromZero,
    TowardsZero,
    Nearest,
}

impl RoundMethod {
    pub fn round(&self, f: f64) -> f64 {
        match self {
            Self::Up => f.ceil(),
            Self::Down => f.floor(),
            Self::FromZero => {
                if f < 0.0 {
                    f.floor()
                } else {
                    f.ceil()
                }
            }
            Self::TowardsZero => {
                if f < 0.0 {
                    f.ceil()
                } else {
                    f.floor()
                }
            }
            Self::Nearest => f.round(),
        }
    }
}

// Represents the options extracted from the --format argument provided by the user.
#[derive(Debug, PartialEq)]
pub struct FormatOptions {
    pub grouping: bool,
    pub padding: Option<isize>,
    pub precision: usize,
    pub prefix: String,
    pub suffix: String,
    pub zero_padding: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            grouping: false,
            padding: None,
            precision: 0,
            prefix: String::from(""),
            suffix: String::from(""),
            zero_padding: false,
        }
    }
}

impl FromStr for FormatOptions {
    type Err = String;

    // The recognized format is: [PREFIX]%[0]['][-][N][.][N]f[SUFFIX]
    //
    // The format defines the printing of a floating point argument '%f'.
    // An optional quote (%'f) enables --grouping.
    // An optional width value (%10f) will pad the number.
    // An optional zero (%010f) will zero pad the number.
    // An optional negative value (%-10f) will left align.
    // An optional precision (%.1f) determines the precision of the number.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.chars().peekable();
        let mut options = Self::default();

        let mut padding = String::from("");
        let mut precision = String::from("");
        let mut double_percentage_counter = 0;

        // '%' chars in the prefix, if any, must appear in blocks of even length, for example: "%%%%" and
        // "%% %%" are ok, "%%% %" is not ok. A single '%' is treated as the beginning of the
        // floating point argument.
        while let Some(c) = iter.next() {
            match c {
                '%' if iter.peek() == Some(&'%') => {
                    iter.next();
                    double_percentage_counter += 1;

                    for _ in 0..2 {
                        options.prefix.push('%');
                    }
                }
                '%' => break,
                _ => options.prefix.push(c),
            }
        }

        // GNU numfmt drops a char from the prefix for every '%%' in the prefix, so we do the same
        for _ in 0..double_percentage_counter {
            options.prefix.pop();
        }

        if iter.peek().is_none() {
            return if options.prefix == s {
                Err(format!("format '{}' has no % directive", s))
            } else {
                Err(format!("format '{}' ends in %", s))
            };
        }

        // GNU numfmt allows to mix the characters " ", "'", and "0" in any way, so we do the same
        while matches!(iter.peek(), Some(' ') | Some('\'') | Some('0')) {
            match iter.next().unwrap() {
                ' ' => (),
                '\'' => options.grouping = true,
                '0' => options.zero_padding = true,
                _ => unreachable!(),
            }
        }

        if let Some('-') = iter.peek() {
            iter.next();

            match iter.peek() {
                Some(c) if c.is_ascii_digit() => padding.push('-'),
                _ => {
                    return Err(format!(
                        "invalid format '{}', directive must be %[0]['][-][N][.][N]f",
                        s
                    ))
                }
            }
        }

        while let Some(c) = iter.peek() {
            if c.is_ascii_digit() {
                padding.push(*c);
                iter.next();
            } else {
                break;
            }
        }

        if !padding.is_empty() {
            if let Ok(p) = padding.parse() {
                options.padding = Some(p);
            } else {
                return Err(format!("invalid format '{}' (width overflow)", s));
            }
        }

        if let Some('.') = iter.peek() {
            iter.next();

            if matches!(iter.peek(), Some(' ') | Some('+') | Some('-')) {
                return Err(format!("invalid precision in format '{}'", s));
            }

            while let Some(c) = iter.peek() {
                if c.is_ascii_digit() {
                    precision.push(*c);
                    iter.next();
                } else {
                    break;
                }
            }

            if !precision.is_empty() {
                if let Ok(p) = precision.parse() {
                    options.precision = p;
                } else {
                    return Err(format!("invalid precision in format '{}'", s));
                }
            }
        }

        if let Some('f') = iter.peek() {
            iter.next();
        } else {
            return Err(format!(
                "invalid format '{}', directive must be %[0]['][-][N][.][N]f",
                s
            ));
        }

        // '%' chars in the suffix, if any, must appear in blocks of even length, otherwise
        // it is an error. For example: "%%%%" and "%% %%" are ok, "%%% %" is not ok.
        while let Some(c) = iter.next() {
            if c != '%' {
                options.suffix.push(c);
            } else if iter.peek() == Some(&'%') {
                for _ in 0..2 {
                    options.suffix.push('%');
                }
                iter.next();
            } else {
                return Err(format!("format '{}' has too many % directives", s));
            }
        }

        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_format() {
        assert_eq!(FormatOptions::default(), "%f".parse().unwrap());
        assert_eq!(FormatOptions::default(), "%  f".parse().unwrap());
    }

    #[test]
    fn test_parse_format_with_invalid_formats() {
        assert!("".parse::<FormatOptions>().is_err());
        assert!("hello".parse::<FormatOptions>().is_err());
        assert!("hello%".parse::<FormatOptions>().is_err());
        assert!("%-f".parse::<FormatOptions>().is_err());
        assert!("%d".parse::<FormatOptions>().is_err());
        assert!("%4 f".parse::<FormatOptions>().is_err());
        assert!("%f%".parse::<FormatOptions>().is_err());
        assert!("%f%%%".parse::<FormatOptions>().is_err());
        assert!("%%f".parse::<FormatOptions>().is_err());
        assert!("%%%%f".parse::<FormatOptions>().is_err());
        assert!("%.-1f".parse::<FormatOptions>().is_err());
        assert!("%. 1f".parse::<FormatOptions>().is_err());
        assert!("%18446744073709551616f".parse::<FormatOptions>().is_err());
        assert!("%.18446744073709551616f".parse::<FormatOptions>().is_err());
    }

    #[test]
    fn test_parse_format_with_prefix_and_suffix() {
        let formats = vec![
            ("--%f", "--", ""),
            ("%f::", "", "::"),
            ("--%f::", "--", "::"),
            ("%f%%", "", "%%"),
            ("%%%f", "%", ""),
            ("%% %f", "%%", ""),
        ];

        for (format, expected_prefix, expected_suffix) in formats {
            let options: FormatOptions = format.parse().unwrap();
            assert_eq!(expected_prefix, options.prefix);
            assert_eq!(expected_suffix, options.suffix);
        }
    }

    #[test]
    fn test_parse_format_with_padding() {
        let mut expected_options = FormatOptions::default();
        let formats = vec![("%12f", Some(12)), ("%-12f", Some(-12))];

        for (format, expected_padding) in formats {
            expected_options.padding = expected_padding;
            assert_eq!(expected_options, format.parse().unwrap());
        }
    }

    #[test]
    fn test_parse_format_with_precision() {
        let mut expected_options = FormatOptions::default();
        let formats = vec![
            ("%6.2f", Some(6), 2),
            ("%6.f", Some(6), 0),
            ("%.2f", None, 2),
            ("%.f", None, 0),
        ];

        for (format, expected_padding, expected_precision) in formats {
            expected_options.padding = expected_padding;
            expected_options.precision = expected_precision;
            assert_eq!(expected_options, format.parse().unwrap());
        }
    }

    #[test]
    fn test_parse_format_with_grouping() {
        let expected_options = FormatOptions {
            grouping: true,
            ..Default::default()
        };
        assert_eq!(expected_options, "%'f".parse().unwrap());
        assert_eq!(expected_options, "% ' f".parse().unwrap());
        assert_eq!(expected_options, "%'''''''f".parse().unwrap());
    }

    #[test]
    fn test_parse_format_with_zero_padding() {
        let expected_options = FormatOptions {
            padding: Some(10),
            zero_padding: true,
            ..Default::default()
        };
        assert_eq!(expected_options, "%010f".parse().unwrap());
        assert_eq!(expected_options, "% 0 10f".parse().unwrap());
        assert_eq!(expected_options, "%0000000010f".parse().unwrap());
    }

    #[test]
    fn test_parse_format_with_grouping_and_zero_padding() {
        let expected_options = FormatOptions {
            grouping: true,
            zero_padding: true,
            ..Default::default()
        };
        assert_eq!(expected_options, "%0'f".parse().unwrap());
        assert_eq!(expected_options, "%'0f".parse().unwrap());
        assert_eq!(expected_options, "%0'0'0'f".parse().unwrap());
        assert_eq!(expected_options, "%'0'0'0f".parse().unwrap());
    }
}
