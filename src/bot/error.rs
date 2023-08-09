use thiserror::Error;

use crate::api;

#[derive(Error, Debug)]
pub enum CongratulatorError {
  #[error(transparent)]
  AsyncSheetsHubError(#[from] api::error::AsyncSheetsHubError),
  #[error(transparent)]
  TeloxideRequestError(#[from] teloxide::RequestError),
  #[error(transparent)]
  ConfigError(#[from] config::ConfigError),
  #[error("Empty (None) callback data received")]
  EmptyCallbackData,
  #[error("Dashboard is empty")]
  EmptyDashboard,
  #[error("No participants have been identified")]
  EmpyParticipants,
  #[error("Bot is not initialized")]
  BotIsNotInitialized,
  #[error("Person was not found")]
  PersonNotFound,
}
