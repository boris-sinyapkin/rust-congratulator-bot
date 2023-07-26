use std::fmt::Display;

#[derive(Debug)]
pub enum ParseError {
  DateParseError(chrono::format::ParseErrorKind),
  ScoreParseError(usize, std::num::ParseFloatError),
  PercentParseError(std::num::IntErrorKind),
}

#[derive(Debug)]
pub enum InvalidCell {
  InvalidDateCell(&'static str),
  InvalidPercentCell(&'static str),
}

#[derive(Debug)]
pub enum Empty {
  EmptyEffectiveFormat(usize),
  EmptyFormattedValue(usize),
}

#[derive(Debug)]
pub enum ScoreTableRecordError {
  UnexpectedFieldIndex(usize),
  ParseError(ParseError),
  InvalidCell(InvalidCell),
  Empty(Empty),
  Unknown(&'static str),
}

impl Display for ScoreTableRecordError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ScoreTableRecordError::Empty(kind) => match kind {
        Empty::EmptyEffectiveFormat(i) => {
          writeln!(f, "Effective format can't be empty (cell index={})", i)
        }
        Empty::EmptyFormattedValue(i) => {
          writeln!(f, "Formatted value can't be empty (cell index={})", i)
        }
      },
      ScoreTableRecordError::InvalidCell(kind) => match kind {
        InvalidCell::InvalidDateCell(msg) => {
          writeln!(f, "The date cell is invalid: {msg:}")
        }
        InvalidCell::InvalidPercentCell(msg) => {
          writeln!(f, "The percent cell is invalid: {msg:}")
        }
      },
      ScoreTableRecordError::ParseError(kind) => match kind {
        ParseError::DateParseError(parse_error_kind) => {
          writeln!(f, "Date parse error occured. Error kind: {:#?}", parse_error_kind)
        }
        ParseError::PercentParseError(parse_error_kind) => {
          writeln!(f, "Percent parse error occured. Error kind: {:#?}", parse_error_kind)
        }
        ParseError::ScoreParseError(i, parse_error_kind) => {
          writeln!(f, "Score parse error occured (cell index={}). Error: {:#?}", i, parse_error_kind)
        }
      },
      ScoreTableRecordError::UnexpectedFieldIndex(i) => {
        writeln!(f, "Unexpected field index = {}", i)
      }
      ScoreTableRecordError::Unknown(msg) => writeln!(f, "Unknown error has occured: {}", msg),
    }
  }
}

impl std::error::Error for ScoreTableRecordError {}