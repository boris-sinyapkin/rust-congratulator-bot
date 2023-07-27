pub mod api;
pub mod bot;
pub mod dashboard;

use bot::Congratulator;
use bot::{config::CongratulatorConfig, error::CongratulatorError};

static APPLICATION_CONFIG_PATH: &str = "etc/app-config.toml";

#[tokio::main]
async fn main() -> Result<(), CongratulatorError> {
  pretty_env_logger::init();
  // Load application config
  let app_config = CongratulatorConfig::load(APPLICATION_CONFIG_PATH)?;
  // Start listening events
  Congratulator::new(app_config).await?.listen().await;
  Ok(())
}
