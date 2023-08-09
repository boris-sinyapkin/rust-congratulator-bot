use thiserror::Error;

use crate::dashboard::score_table::error::ScoreTableRecordError;

#[derive(Error, Debug)]
pub enum InvalidFetchedData {
  #[error("Google Sheets data is empty")]
  EmptySheets,
  #[error("Grid data is empty")]
  EmptyGridData,
  #[error("Row data is empty")]
  EmptyRowData,
  #[error("Cell data is empty")]
  EmptyCellData,
  #[error("Formatted value is empty")]
  EmptyFormattedValue,
  #[error("Person name cell is empty")]
  EmptyPersonNameCell,
  #[error("Invalid vector size observed")]
  InvalidVectorSize,
  #[error("Sheet id for the derived title `{0}` was not found")]
  NotFoundSheetId(String),
}

#[derive(Error, Debug)]
pub enum AsyncSheetsHubError {
  #[error(transparent)]
  ScoreTableRecordError(#[from] ScoreTableRecordError),
  #[error(transparent)]
  GoogleSheetsApiError(#[from] google_sheets4::Error),
  #[error(transparent)]
  IO(#[from] std::io::Error),
  #[error(transparent)]
  InvalidFetchedData(InvalidFetchedData),
  #[error("Unknown error has occured: `{0}`")]
  Unknown(&'static str),
}