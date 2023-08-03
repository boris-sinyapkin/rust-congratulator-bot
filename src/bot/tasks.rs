use std::sync::Arc;

use chrono::Utc;
use log::{debug, error, info, trace};
use teloxide::{requests::Requester, types::ChatId, Bot};
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
    info!("[PeriodicDataFetcher] Task has started at {}", helpers::current_time());
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
    info!("[PeriodicDataFetcher] Task has finished at {}", helpers::current_time());
  }
}

pub struct PeriodicNotifier {
  task_handle: tokio::task::JoinHandle<()>,
}

impl PeriodicNotifier {
  pub fn schedule(bot: Bot, text: String, chat_id: ChatId, when: (u32, u32, u32)) -> Self {
    info!("[PeriodicNotifier] Scheduling the task");
    let task = move || {
      let cloned_bot = bot.clone();
      let cloned_text = text.clone();
      async move {
        PeriodicNotifier::do_notify(cloned_bot, cloned_text, chat_id).await;
      }
    };
    let (h, m, s) = when;
    let task_future = every(1).day().at(h, m, s).in_timezone(&Utc).perform(task);
    Self {
      task_handle: tokio::spawn(task_future),
    }
  }

  async fn do_notify(bot: Bot, text: String, chat_id: ChatId) {
    info!("[PeriodicNotifier] Task has started at {}", helpers::current_time());
    match bot.send_message(chat_id, &text[..]).await {
      Ok(_) => info!("[PeriodicNotifier] Sent text='{}' to chat_id={}", text, chat_id),
      Err(err) => error!(
        "[PeriodicNotifier] Unable to send text='{}' to chat_id={} due to {:?}",
        text, chat_id, err
      ),
    }
    info!("[PeriodicNotifier] Task has finished at {}", helpers::current_time());
  }

  pub fn cancel(&self) {
    self.task_handle.abort();
    debug!("[PeriodicNotifier] The task was aborted");
  }
}
