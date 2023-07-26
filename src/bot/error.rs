use std::fmt::Display;

use crate::api;

#[derive(Debug)]
pub enum CongratulatorError {
  AsyncSheetsHubError(api::error::AsyncSheetsHubError),
  TeloxideRequestError(teloxide::RequestError),
  EmptyCallbackData,
  EmptyDashboard,
  EmpyParticipants,
  BotIsNotInitialized,
  PersonNotFound
}

impl Display for CongratulatorError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CongratulatorError::BotIsNotInitialized => writeln!(f, "Bot is not initialized"),
      CongratulatorError::PersonNotFound => writeln!(f, "Person was not found"),
      CongratulatorError::EmptyCallbackData => writeln!(f, "Empty (None) callback data received"),
      CongratulatorError::EmptyDashboard => writeln!(f, "Dashboard is empty"),
      CongratulatorError::EmpyParticipants => writeln!(f, "No participants have been identified"),
      CongratulatorError::AsyncSheetsHubError(err) => err.fmt(f),
      CongratulatorError::TeloxideRequestError(err) => err.fmt(f)
    }
  }
}

impl std::convert::From<api::error::AsyncSheetsHubError> for CongratulatorError {
  fn from(value: api::error::AsyncSheetsHubError) -> Self {
    CongratulatorError::AsyncSheetsHubError(value)
  }
}

impl std::convert::From<teloxide::RequestError> for CongratulatorError {
  fn from(value: teloxide::RequestError) -> Self {
    CongratulatorError::TeloxideRequestError(value)
  }
}

impl std::error::Error for CongratulatorError {}
