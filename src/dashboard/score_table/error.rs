use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
  #[error("Date parse error occured. Error kind: {0:?}")]
  DateParseError(chrono::format::ParseErrorKind),
  #[error("Score parse error occured (cell index={0}). Error: {1:?}")]
  ScoreParseError(usize, std::num::ParseFloatError),
  #[error("Percent parse error occured. Error kind: {0:?}")]
  PercentParseError(std::num::IntErrorKind),
}

#[derive(Error, Debug)]
pub enum InvalidCell {
  #[error("The date cell is invalid: {0:}")]
  InvalidDateCell(&'static str),
  #[error("The percent cell is invalid: {0:}")]
  InvalidPercentCell(&'static str),
}

#[derive(Error, Debug)]
pub enum Empty {
  #[error("Effective format can't be empty (cell index={0})")]
  EmptyEffectiveFormat(usize),
  #[error("Formatted value can't be empty (cell index={0})")]
  EmptyFormattedValue(usize),
}

#[derive(Error, Debug)]
pub enum ScoreTableRecordError {
  #[error("Unexpected field index = {0}")]
  UnexpectedFieldIndex(usize),
  #[error(transparent)]
  ParseError(ParseError),
  #[error(transparent)]
  InvalidCell(InvalidCell),
  #[error(transparent)]
  Empty(Empty),
  #[error("Unknown error has occured: {0}")]
  Unknown(&'static str),
}