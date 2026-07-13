#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

const MAX_LOG_BYTES: usize = 4 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 512;
const MAX_ROWS: usize = 50_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceRow {
    pub line: usize,
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub status: u8,
    pub sp: u8,
    pub cycles: u64,
    opcode_bytes: [u8; 3],
    opcode_len: u8,
}

impl ReferenceRow {
    #[must_use]
    pub fn opcode_bytes(&self) -> &[u8] {
        &self.opcode_bytes[..usize::from(self.opcode_len)]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceLog {
    rows: Vec<ReferenceRow>,
}

impl ReferenceLog {
    #[must_use]
    pub fn rows(&self) -> &[ReferenceRow] {
        &self.rows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    InputTooLarge {
        size: usize,
        maximum: usize,
    },
    NonAscii,
    Empty,
    TooManyRows {
        maximum: usize,
    },
    BlankLine {
        line: usize,
    },
    LineTooLong {
        line: usize,
        size: usize,
        maximum: usize,
    },
    InvalidProgramCounter {
        line: usize,
    },
    MissingOpcode {
        line: usize,
    },
    TooManyOpcodeBytes {
        line: usize,
    },
    MissingField {
        line: usize,
        field: &'static str,
    },
    DuplicateField {
        line: usize,
        field: &'static str,
    },
    InvalidField {
        line: usize,
        field: &'static str,
    },
    FieldsOutOfOrder {
        line: usize,
    },
    TrailingFields {
        line: usize,
    },
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputTooLarge { size, maximum } => {
                write!(
                    formatter,
                    "reference log is {size} bytes; maximum is {maximum}"
                )
            }
            Self::NonAscii => formatter.write_str("reference log is not ASCII"),
            Self::Empty => formatter.write_str("reference log has no rows"),
            Self::TooManyRows { maximum } => {
                write!(formatter, "reference log exceeds {maximum} rows")
            }
            Self::BlankLine { line } => write!(formatter, "blank reference row at line {line}"),
            Self::LineTooLong {
                line,
                size,
                maximum,
            } => write!(
                formatter,
                "reference row {line} is {size} bytes; maximum is {maximum}"
            ),
            Self::InvalidProgramCounter { line } => {
                write!(formatter, "invalid program counter at line {line}")
            }
            Self::MissingOpcode { line } => write!(formatter, "missing opcode at line {line}"),
            Self::TooManyOpcodeBytes { line } => {
                write!(formatter, "more than three opcode bytes at line {line}")
            }
            Self::MissingField { line, field } => {
                write!(formatter, "missing {field} field at line {line}")
            }
            Self::DuplicateField { line, field } => {
                write!(formatter, "duplicate {field} field at line {line}")
            }
            Self::InvalidField { line, field } => {
                write!(formatter, "invalid {field} field at line {line}")
            }
            Self::FieldsOutOfOrder { line } => {
                write!(
                    formatter,
                    "reference fields are out of order at line {line}"
                )
            }
            Self::TrailingFields { line } => {
                write!(formatter, "unexpected fields follow CYC at line {line}")
            }
        }
    }
}

impl Error for ParseError {}

pub fn parse(input: &[u8]) -> Result<ReferenceLog, ParseError> {
    if input.len() > MAX_LOG_BYTES {
        return Err(ParseError::InputTooLarge {
            size: input.len(),
            maximum: MAX_LOG_BYTES,
        });
    }
    if !input.is_ascii() {
        return Err(ParseError::NonAscii);
    }
    let text = std::str::from_utf8(input).map_err(|_| ParseError::NonAscii)?;
    let mut rows = Vec::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        if rows.len() == MAX_ROWS {
            return Err(ParseError::TooManyRows { maximum: MAX_ROWS });
        }
        if raw_line.is_empty() {
            return Err(ParseError::BlankLine { line: line_number });
        }
        if raw_line.len() > MAX_LINE_BYTES {
            return Err(ParseError::LineTooLong {
                line: line_number,
                size: raw_line.len(),
                maximum: MAX_LINE_BYTES,
            });
        }
        rows.push(parse_row(raw_line, line_number)?);
    }
    if rows.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(ReferenceLog { rows })
}

fn parse_row(row: &str, line: usize) -> Result<ReferenceRow, ParseError> {
    let tokens: Vec<&str> = row.split_ascii_whitespace().collect();
    let pc_token = tokens
        .first()
        .copied()
        .ok_or(ParseError::InvalidProgramCounter { line })?;
    if pc_token.len() != 4 {
        return Err(ParseError::InvalidProgramCounter { line });
    }
    let pc = u16::from_str_radix(pc_token, 16)
        .map_err(|_| ParseError::InvalidProgramCounter { line })?;

    let mut opcode_bytes = [0; 3];
    let mut opcode_len = 0_usize;
    for token in tokens.iter().skip(1) {
        if !is_hex_byte(token) {
            break;
        }
        if opcode_len == opcode_bytes.len() {
            return Err(ParseError::TooManyOpcodeBytes { line });
        }
        opcode_bytes[opcode_len] =
            u8::from_str_radix(token, 16).map_err(|_| ParseError::MissingOpcode { line })?;
        opcode_len += 1;
    }
    if opcode_len == 0 {
        return Err(ParseError::MissingOpcode { line });
    }

    let (a_index, a) = find_hex_field(&tokens, "A:", "A", line)?;
    let (x_index, x) = find_hex_field(&tokens, "X:", "X", line)?;
    let (y_index, y) = find_hex_field(&tokens, "Y:", "Y", line)?;
    let (status_index, status) = find_hex_field(&tokens, "P:", "P", line)?;
    let (sp_index, sp) = find_hex_field(&tokens, "SP:", "SP", line)?;
    let (cycles_index, cycles) = find_cycle_field(&tokens, line)?;
    if !(a_index < x_index
        && x_index < y_index
        && y_index < status_index
        && status_index < sp_index
        && sp_index < cycles_index)
    {
        return Err(ParseError::FieldsOutOfOrder { line });
    }
    if cycles_index + 1 != tokens.len() {
        return Err(ParseError::TrailingFields { line });
    }

    Ok(ReferenceRow {
        line,
        pc,
        a,
        x,
        y,
        status,
        sp,
        cycles,
        opcode_bytes,
        opcode_len: opcode_len as u8,
    })
}

fn find_hex_field(
    tokens: &[&str],
    prefix: &'static str,
    field: &'static str,
    line: usize,
) -> Result<(usize, u8), ParseError> {
    let mut result = None;
    for (index, token) in tokens.iter().enumerate() {
        let Some(value) = token.strip_prefix(prefix) else {
            continue;
        };
        if result.is_some() {
            return Err(ParseError::DuplicateField { line, field });
        }
        if !is_hex_byte(value) {
            return Err(ParseError::InvalidField { line, field });
        }
        let parsed =
            u8::from_str_radix(value, 16).map_err(|_| ParseError::InvalidField { line, field })?;
        result = Some((index, parsed));
    }
    result.ok_or(ParseError::MissingField { line, field })
}

fn find_cycle_field(tokens: &[&str], line: usize) -> Result<(usize, u64), ParseError> {
    let mut result = None;
    for (index, token) in tokens.iter().enumerate() {
        let Some(value) = token.strip_prefix("CYC:") else {
            continue;
        };
        if result.is_some() {
            return Err(ParseError::DuplicateField { line, field: "CYC" });
        }
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ParseError::InvalidField { line, field: "CYC" });
        }
        let parsed = value
            .parse()
            .map_err(|_| ParseError::InvalidField { line, field: "CYC" })?;
        result = Some((index, parsed));
    }
    result.ok_or(ParseError::MissingField { line, field: "CYC" })
}

fn is_hex_byte(value: &str) -> bool {
    value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: &str = "C000  A9 01     LDA #$01 A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7";

    #[test]
    fn parses_lf_crlf_and_final_line_without_newline() {
        let log =
            parse(format!("{ROW}\r\nC002  AA TAX A:01 X:00 Y:00 P:24 SP:FD CYC:9").as_bytes())
                .expect("generated reference log parses");
        assert_eq!(log.rows().len(), 2);
        assert_eq!(log.rows()[0].pc, 0xc000);
        assert_eq!(log.rows()[0].opcode_bytes(), &[0xa9, 0x01]);
        assert_eq!(log.rows()[1].cycles, 9);
    }

    #[test]
    fn rejects_every_strict_truncation_of_a_valid_row() {
        for end in 0..ROW.len() {
            assert!(
                parse(&ROW.as_bytes()[..end]).is_err(),
                "prefix length {end}"
            );
        }
    }

    #[test]
    fn rejects_malformed_boundaries_without_echoing_input() {
        let cases: &[&[u8]] = &[
            b"",
            b"\n",
            b"C000  LDA A:00 X:00 Y:00 P:24 SP:FD CYC:7",
            b"C000 A9 01 02 03 LDA A:00 X:00 Y:00 P:24 SP:FD CYC:7",
            b"C000 A9 LDA A:GG X:00 Y:00 P:24 SP:FD CYC:7",
            b"C000 A9 LDA X:00 A:00 Y:00 P:24 SP:FD CYC:7",
            b"C000 A9 LDA A:00 A:00 X:00 Y:00 P:24 SP:FD CYC:7",
            b"C000 A9 LDA A:00 X:00 Y:00 P:24 SP:FD CYC:18446744073709551616",
            b"C000 A9 LDA A:00 X:00 Y:00 P:24 SP:FD CYC:7 EXTRA",
            b"\xff",
        ];
        for input in cases {
            assert!(parse(input).is_err());
        }
    }

    #[test]
    fn rejects_oversized_input_and_line() {
        assert_eq!(
            parse(&vec![b'A'; MAX_LOG_BYTES + 1]),
            Err(ParseError::InputTooLarge {
                size: MAX_LOG_BYTES + 1,
                maximum: MAX_LOG_BYTES,
            })
        );
        assert_eq!(
            parse(&vec![b'A'; MAX_LINE_BYTES + 1]),
            Err(ParseError::LineTooLong {
                line: 1,
                size: MAX_LINE_BYTES + 1,
                maximum: MAX_LINE_BYTES,
            })
        );
    }

    #[test]
    fn rejects_excessive_row_count_before_allocating_unbounded_state() {
        let mut input = String::with_capacity((ROW.len() + 1) * (MAX_ROWS + 1));
        for _ in 0..=MAX_ROWS {
            input.push_str(ROW);
            input.push('\n');
        }
        assert_eq!(
            parse(input.as_bytes()),
            Err(ParseError::TooManyRows { maximum: MAX_ROWS })
        );
    }

    #[test]
    fn arbitrary_small_inputs_do_not_panic() {
        for length in 0..=256 {
            let bytes: Vec<u8> = (0..length).map(|index| index as u8).collect();
            let _ = parse(&bytes);
        }
    }
}
