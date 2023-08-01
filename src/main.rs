pub mod api;
pub mod bot;
pub mod dashboard;
pub mod helpers;

use bot::Congratulator;
use bot::{config::CongratulatorConfig, error::CongratulatorError};

#[tokio::main]
async fn main() -> Result<(), CongratulatorError> {
  pretty_env_logger::init();
  // Load application config
  let app_config = CongratulatorConfig::load_from_env()?;
  // Start listening events
  Congratulator::new(app_config).await?.listen().await;
  Ok(())
}
