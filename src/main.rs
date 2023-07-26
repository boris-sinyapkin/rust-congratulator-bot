pub mod api;
pub mod bot;
pub mod dashboard;

use bot::error::CongratulatorError;
use bot::Congratulator;

#[tokio::main]
async fn main() -> Result<(), CongratulatorError> {
  pretty_env_logger::init();
  Congratulator::new().await?.listen().await;
  Ok(())
}
