use config::Config;
use log::info;
use serde::{Deserialize, Serialize};

use super::error::CongratulatorError;

#[derive(Serialize, Deserialize, Debug)]
pub struct CongratulatorConfig {
  bot_token: String,
  spreadsheet_id: String,
  api_creds_json_data: String,
  api_token_path: String,
  api_data_fetch_task_interval_min: u32
}

impl CongratulatorConfig {
  pub fn load_from_env() -> Result<CongratulatorConfig, CongratulatorError> {
    info!("[Config] Application config is getting loaded from env");
    let serialized = Config::builder().add_source(config::Environment::default()).build()?;
    let deserialized = serialized.try_deserialize::<Self>()?;
    info!("[Config] Application config has been loaded");
    Ok(deserialized)
  }

  pub fn fetch_data_interval_min(&self) -> u32 {
    self.api_data_fetch_task_interval_min
  }

  pub fn spreadsheet_id(&self) -> &str {
    &self.spreadsheet_id
  }

  pub fn api_creds_json_data(&self) -> &str {
    &self.api_creds_json_data
  }

  pub fn api_token_path(&self) -> &str {
    &self.api_token_path
  }

  pub fn bot_token_str(&self) -> &str {
    &self.bot_token
  }
}
