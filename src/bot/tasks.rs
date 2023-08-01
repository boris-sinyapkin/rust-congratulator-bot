use std::sync::Arc;

use chrono::Utc;
use log::{debug, error, info, trace};
use tokio::sync::RwLock;
use tokio_schedule::{every, Job};

use crate::{dashboard::Dashboard, helpers};

use super::AsyncSheetsHub;

pub struct PeriodicDataFetcher {
  task_handle: tokio::task::JoinHandle<()>,
}

impl PeriodicDataFetcher {
  pub fn schedule(interval_min: u32, hub: Arc<AsyncSheetsHub>, dashboard: Arc<RwLock<Dashboard>>) -> Self {
    info!("[PeriodicDataFetcher] Scheduling the task");
    let task = move || {
      let cloned_hub = hub.clone();
      let cloned_dashboard = dashboard.clone();
      async move {
        PeriodicDataFetcher::do_update(cloned_hub, cloned_dashboard).await;
      }
    };
    let task_future = every(interval_min).minutes().in_timezone(&Utc).perform(task);
    Self {
      task_handle: tokio::spawn(task_future),
    }
  }

  pub fn cancel(&self) {
    self.task_handle.abort();
    debug!("[PeriodicDataFetcher] The task was aborted");
  }

  async fn do_update(hub: Arc<AsyncSheetsHub>, dashboard: Arc<RwLock<Dashboard>>) {
    info!("[PeriodicDataFetcher] Task has started {}", helpers::current_time());
    debug!("[PeriodicDataFetcher] Fetching the latest data...");
    let latest_dashboard = match hub.fetch_dashboard().await {
      Ok(data) => {
        debug!("[PeriodicDataFetcher] New dashboard has been successfully fetched");
        data
      }
      Err(hub_err) => {
        error!(
          "[PeriodicDataFetcher] Error occured while fetching the data: {:#?}. Exiting the task...",
          hub_err
        );
        return;
      }
    };

    trace!("[PeriodicDataFetcher] Acquiring WRITE lock on dashboard...");
    {
      let mut locked_dashboard = dashboard.write().await;
      trace!("[PeriodicDataFetcher] WRITE lock on dashboard has been acquired");
      *locked_dashboard = latest_dashboard;
      trace!("[PeriodicDataFetcher] New dashboard has been successfully fetched and replaced with the old one");
    }
    info!("[PeriodicDataFetcher] Task has finished {}", helpers::current_time());
  }
}
