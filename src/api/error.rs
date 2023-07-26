use std::fmt::Display;

use crate::dashboard::score_table::error::ScoreTableRecordError;

#[derive(Debug)]
pub enum InvalidFetchedData {
  EmptySheets,
  EmptyGridData,
  EmptyRowData,
  EmptyCellData,
  EmptyFormattedValue,
  EmptyPersonNameCell,
  InvalidVectorSize,
  NotFoundSheetId(String),
}

#[derive(Debug)]
pub enum AsyncSheetsHubError {
  ScoreTableRecordError(ScoreTableRecordError),
  GoogleSheetsApiError(google_sheets4::Error),
  IO(std::io::Error),
  InvalidFetchedData(InvalidFetchedData),
  Unknown(&'static str),
}

impl Display for AsyncSheetsHubError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AsyncSheetsHubError::GoogleSheetsApiError(err) => err.fmt(f),
      AsyncSheetsHubError::ScoreTableRecordError(err) => err.fmt(f),
      AsyncSheetsHubError::IO(err) => err.fmt(f),
      AsyncSheetsHubError::InvalidFetchedData(kind) => match kind {
        InvalidFetchedData::EmptyFormattedValue => {
          writeln!(f, "Formatted value is empty")
        }
        InvalidFetchedData::EmptyCellData => writeln!(f, "Cell data is empty"),
        InvalidFetchedData::EmptyGridData => writeln!(f, "Grid data is empty"),
        InvalidFetchedData::EmptyRowData => writeln!(f, "Row data is empty"),
        InvalidFetchedData::EmptySheets => {
          writeln!(f, "Google Sheets data is empty")
        }
        InvalidFetchedData::InvalidVectorSize => {
          writeln!(f, "Invalid vector size observed")
        }
        InvalidFetchedData::NotFoundSheetId(title_name) => {
          writeln!(f, "Sheet id for the derived title {} was not found", title_name)
        }
        InvalidFetchedData::EmptyPersonNameCell => {
          writeln!(f, "Person name cell is empty")
        },
      },
      AsyncSheetsHubError::Unknown(msg) => writeln!(f, "Unknown error has occured: {}", msg),
    }
  }
}

impl std::convert::From<ScoreTableRecordError> for AsyncSheetsHubError {
  fn from(value: ScoreTableRecordError) -> Self {
    AsyncSheetsHubError::ScoreTableRecordError(value)
  }
}

impl std::convert::From<google_sheets4::Error> for AsyncSheetsHubError {
  fn from(value: google_sheets4::Error) -> Self {
    AsyncSheetsHubError::GoogleSheetsApiError(value)
  }
}

impl std::convert::From<std::io::Error> for AsyncSheetsHubError {
  fn from(value: std::io::Error) -> Self {
    AsyncSheetsHubError::IO(value)
  }
}

impl std::error::Error for AsyncSheetsHubError {}
