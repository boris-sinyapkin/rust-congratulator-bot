use config::Config;
use log::info;
use serde::{Deserialize, Serialize};

use super::error::CongratulatorError;

#[derive(Serialize, Deserialize, Debug)]
pub struct CongratulatorConfig {
  bot_token: String,
  spreadsheet_id: String,
  api_creds_path: String,
  api_token_path: String,
}

impl CongratulatorConfig {
  pub fn load(path: &str) -> Result<CongratulatorConfig, CongratulatorError> {
    info!("[Config] Application config is getting loaded from '{}'", path);
    let serialized = Config::builder().add_source(config::File::with_name(path)).build()?;
    let deserialized = serialized.try_deserialize::<Self>()?;
    info!("[Config] Application config has been loaded");
    Ok(deserialized)
  }

  pub fn spreadsheet_id(&self) -> &str {
    &self.spreadsheet_id
  }

  pub fn api_creds_path(&self) -> &str {
    &self.api_creds_path
  }

  pub fn api_token_path(&self) -> &str {
    &self.api_token_path
  }

  pub fn bot_token_str(&self) -> &str {
    &self.bot_token
  }
}

impl ::std::default::Default for CongratulatorConfig {
  fn default() -> Self {
    Self {
      bot_token: "Unknown Bot Token".to_string(),
      spreadsheet_id: "Unknown Spreadsheet Id".to_string(),
      api_creds_path: "etc/credentials.json".to_string(),
      api_token_path: "etc/token.json".to_string(),
    }
  }
}
