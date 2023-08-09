use std::sync::Arc;

use chrono::{Local, Utc};
use log::{debug, error, info, trace, warn};
use teloxide::{
  payloads::SendMessageSetters,
  requests::Requester,
  types::{ChatId, ParseMode},
  Bot,
};
use tokio::sync::RwLock;
use tokio_schedule::{EveryDay, EveryMinute, Job};

use crate::{
  dashboard::{Dashboard, DashboardError},
  helpers,
};

use super::{AsyncSheetsHub, LockedDashboard};

pub type EveryDayTime = EveryDay<Utc, Local>;
pub type EveryMinuteTime = EveryMinute<Utc, Local>;
pub type TaskHandler = tokio::task::JoinHandle<()>;

pub trait PeriodicTask<T> {
  fn schedule(&self, when: T) -> TaskHandler;
}

/// This task periodically downloads latest data from Sheets through the AsyncHub instance,
/// and updates the Dashboard through RwLock
pub struct PeriodicDataFetcher {
  hub: Arc<AsyncSheetsHub>,
  dashboard: Arc<LockedDashboard>,
}

impl PeriodicDataFetcher {
  pub fn new(hub: Arc<AsyncSheetsHub>, dashboard: Arc<LockedDashboard>) -> Self {
    Self { hub, dashboard }
  }

  async fn do_update(hub: Arc<AsyncSheetsHub>, dashboard: Arc<RwLock<Dashboard>>) {
    info!("[PeriodicDataFetcher] Task has started at {}", helpers::current_time_utc());
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
    info!("[PeriodicDataFetcher] Task has finished at {}", helpers::current_time_utc());
  }
}

impl PeriodicTask<EveryMinuteTime> for PeriodicDataFetcher {
  fn schedule(&self, when: EveryMinuteTime) -> TaskHandler {
    info!("[PeriodicDataFetcher] Scheduling the task");

    let hub = self.hub.clone();
    let dashboard = self.dashboard.clone();

    let task = move || {
      let cloned_hub = hub.clone();
      let cloned_dashboard = dashboard.clone();
      async move {
        PeriodicDataFetcher::do_update(cloned_hub, cloned_dashboard).await;
      }
    };
    tokio::spawn(when.perform(task))
  }
}

/// This task periodically (once a day) sends text to the specified 'chat_id'
pub struct PeriodicNotifier {
  bot: Bot,
  text: String,
  chat_id: ChatId,
}

impl PeriodicNotifier {
  pub fn new(bot: Bot, text: String, chat_id: ChatId) -> Self {
    Self { bot, text, chat_id }
  }

  async fn do_notify(bot: Bot, text: String, chat_id: ChatId) {
    info!("[PeriodicNotifier] Task has started at {}", helpers::current_time_utc());
    match bot.send_message(chat_id, &text[..]).await {
      Ok(_) => info!("[PeriodicNotifier] Sent text='{}' to chat_id={}", text, chat_id),
      Err(err) => error!(
        "[PeriodicNotifier] Unable to send text='{}' to chat_id={} due to {:?}",
        text, chat_id, err
      ),
    }
    info!("[PeriodicNotifier] Task has finished at {}", helpers::current_time_utc());
  }
}

impl PeriodicTask<EveryDayTime> for PeriodicNotifier {
  fn schedule(&self, when: EveryDayTime) -> TaskHandler {
    info!("[PeriodicNotifier] Scheduling every day task for {:?}", when);

    let bot = self.bot.clone();
    let text = self.text.clone();
    let chat_id = self.chat_id;

    let task = move || {
      let cloned_bot = bot.clone();
      let cloned_text = text.clone();
      async move {
        PeriodicNotifier::do_notify(cloned_bot, cloned_text, chat_id).await;
      }
    };

    tokio::spawn(when.perform(task))
  }
}

pub struct PeriodicSummarySender {
  bot: Bot,
  chat_id: ChatId,
  dashboard: Arc<LockedDashboard>,
}

impl PeriodicSummarySender {
  pub fn new(bot: Bot, dashboard: Arc<LockedDashboard>, chat_id: ChatId) -> Self {
    Self { bot, chat_id, dashboard }
  }

  pub async fn send_summary(bot: Bot, dashboard: Arc<LockedDashboard>, chat_id: ChatId) {
    info!("[PeriodicSummarySender] Task has started at {}", helpers::current_time_utc());
    let locked_dashboard = dashboard.read().await;
    let by_date = helpers::current_time_utc().date_naive(); // always send "today" summary
    match locked_dashboard.summary(&by_date) {
      Ok(summary) => {
        let msg = helpers::format_summary_msg(&summary, &by_date);
        let _ = bot.send_message(chat_id, msg).parse_mode(ParseMode::MarkdownV2).await;
        info!("[PeriodicSummarySender] Summary has been successfully sent");
      }
      Err(DashboardError::EmptyParticipants) => {
        warn!("[PeriodicSummarySender] The participants were not found");
      }
    }
    info!("[PeriodicSummarySender] Task has finished at {}", helpers::current_time_utc());
  }
}

impl PeriodicTask<EveryDayTime> for PeriodicSummarySender {
  fn schedule(&self, when: EveryDayTime) -> TaskHandler {
    info!("[PeriodicSummarySender] Scheduling every day task for {:?}", when);

    let bot = self.bot.clone();
    let chat_id = self.chat_id;
    let dashboard = self.dashboard.clone();

    let task = move || {
      let cloned_bot = bot.clone();
      let cloned_dashboard = dashboard.clone();
      async move {
        PeriodicSummarySender::send_summary(cloned_bot, cloned_dashboard, chat_id).await;
      }
    };

    tokio::spawn(when.perform(task))
  }
}
