use std::sync::Arc;

use log::{debug, error, info, trace, warn};
use teloxide::{
  payloads::SendMessageSetters,
  requests::Requester,
  types::{ChatId, ParseMode},
  Bot,
};

use crate::{
  dashboard::DashboardError,
  helpers::{self, PeriodicTimeUtc},
};

use super::{AsyncSheetsHub, LockedDashboard};

pub type TaskHandle = tokio::task::JoinHandle<()>;

#[derive(PartialEq)]
pub enum PeriodcTaskType {
  Notifier,
  Fetcher,
}

pub trait PeriodicTask: Sync + Send {
  fn schedule(&mut self, when: PeriodicTimeUtc) -> bool {
    info!("[{}] Scheduling the task ({})", self.name(), when);

    if !self.is_finished() {
      warn!("[{}] The task is already scheduled at {:?}", self.name(), when);
      return false;
    }

    self.submit_job(when);
    info!("[{}] Job was submitted", self.name());
    true
  }

  fn is_finished(&self) -> bool {
    if let Some(h) = self.handle() {
      h.is_finished()
    } else {
      true
    }
  }

  fn cancel(&self) {
    if !self.is_finished() {
      if let Some(h) = self.handle().as_ref() { h.abort() }
      info!("[{}] Task was cancelled", self.name());
    }
  }

  fn submit_job(&mut self, when: PeriodicTimeUtc);

  fn name(&self) -> &str; 
  fn when(&self) -> Option<&PeriodicTimeUtc>;
  fn handle(&self) -> Option<&TaskHandle>;
  fn task_type(&self) -> &PeriodcTaskType;
  fn description(&self) -> Option<String>;
}

pub struct TaskManager<'a> {
  bot: Bot,
  tasks: Vec<Box<dyn PeriodicTask + 'a>>,
  dashboard: Arc<LockedDashboard>,
}

impl<'a> TaskManager<'a> {
  pub fn new(bot: Bot, dashboard: Arc<LockedDashboard>) -> Self {
    Self {
      bot,
      dashboard,
      tasks: Vec::new(),
    }
  }

  pub fn create_notifier_task(&self, text: String, chat_id: ChatId) -> PeriodicNotifier {
    PeriodicNotifier::new(self.bot.clone(), text, chat_id)
  }

  pub fn create_data_fetcher_task(&self, hub: Arc<AsyncSheetsHub>) -> PeriodicDataFetcher {
    PeriodicDataFetcher::new(hub, self.dashboard.clone())
  }

  pub fn create_summary_sender_task(&self, chat_id: ChatId) -> PeriodicSummarySender {
    PeriodicSummarySender::new(self.bot.clone(), self.dashboard.clone(), chat_id)
  }

  pub fn tasks(&self, task_type: PeriodcTaskType) -> Vec<&(dyn PeriodicTask + 'a)> {
    self
      .tasks
      .iter()
      .filter(|t| *t.task_type() == task_type)
      .map(|t| t.as_ref())
      .collect()
  }

  pub fn schedule_task<Task>(&mut self, mut task: Task, when: PeriodicTimeUtc)
  where
    Task: 'a + PeriodicTask,
  {
    task.schedule(when);
    self.tasks.push(Box::new(task));
  }

  pub fn finalize_tasks(&self) {
    for t in &self.tasks {
      t.cancel();
    }
  }
}

/// This task periodically downloads latest data from Sheets through the AsyncHub instance,
/// and updates the Dashboard through RwLock
pub struct PeriodicDataFetcher {
  hub: Arc<AsyncSheetsHub>,
  name: String,
  when: Option<PeriodicTimeUtc>,
  handle: Option<TaskHandle>,
  task_type: PeriodcTaskType,
  dashboard: Arc<LockedDashboard>,
}

impl PeriodicDataFetcher {
  fn new(hub: Arc<AsyncSheetsHub>, dashboard: Arc<LockedDashboard>) -> Self {
    PeriodicDataFetcher {
      hub,
      dashboard,
      name: "PeriodicDataFetcher".to_string(),
      when: None,
      handle: None,
      task_type: PeriodcTaskType::Fetcher,
    }
  }

  async fn do_update(name: String, hub: Arc<AsyncSheetsHub>, dashboard: Arc<LockedDashboard>) {
    info!("[{}] Task has started at {}", name, helpers::current_time_utc());
    debug!("[{}] Fetching the latest data...", name);
    let latest_dashboard = match hub.fetch_dashboard().await {
      Ok(data) => {
        debug!("[{}] New dashboard has been successfully fetched", name);
        data
      }
      Err(hub_err) => {
        error!(
          "[{}] Error occured while fetching the data: {:#?}. Exiting the task...",
          name, hub_err
        );
        return;
      }
    };

    trace!("[{}] Acquiring WRITE lock on dashboard...", name);
    {
      let mut locked_dashboard = dashboard.write().await;
      trace!("[{}] WRITE lock on dashboard has been acquired", name);
      *locked_dashboard = latest_dashboard;
      trace!(
        "[{}] New dashboard has been successfully fetched and replaced with the old one",
        name
      );
    }
    info!("[{}] Task has finished at {}", name, helpers::current_time_utc());
  }
}

impl PeriodicTask for PeriodicDataFetcher {
  fn submit_job(&mut self, when: PeriodicTimeUtc) {
    assert!(self.is_finished(), "should be finished");

    let hub = self.hub.clone();
    let dashboard = self.dashboard.clone();
    let name = self.name.clone();

    let task = move || {
      let cloned_hub = hub.clone();
      let cloned_dashboard = dashboard.clone();
      let cloned_name = name.clone();
      async move {
        PeriodicDataFetcher::do_update(cloned_name, cloned_hub, cloned_dashboard).await;
      }
    };

    self.when = Some(when.clone());
    self.handle = Some(when.perform_task(task));
  }

  fn description(&self) -> Option<String> {
    self.when().map(|w| format!("Я скачиваю данные из Google Sheets {}", w))
  }

  fn task_type(&self) -> &PeriodcTaskType {
    &self.task_type
  }

  fn handle(&self) -> Option<&TaskHandle> {
    self.handle.as_ref()
  }

  fn when(&self) -> Option<&PeriodicTimeUtc> {
    self.when.as_ref()
  }

  fn name(&self) -> &str {
    &self.name[..]
  }
}

/// This task periodically (once a day) sends text to the specified 'chat_id'
pub struct PeriodicNotifier {
  bot: Bot,
  text: String,
  name: String,
  when: Option<PeriodicTimeUtc>,
  handle: Option<TaskHandle>,
  task_type: PeriodcTaskType,
  chat_id: ChatId,
}

impl PeriodicNotifier {
  fn new(bot: Bot, text: String, chat_id: ChatId) -> Self {
    PeriodicNotifier {
      bot,
      text,
      chat_id,
      name: "PeriodicNotifier".to_string(),
      when: None,
      handle: None,
      task_type: PeriodcTaskType::Notifier,
    }
  }
  async fn do_notify(name: String, bot: Bot, text: String, chat_id: ChatId) {
    info!("[{}] Task has started at {}", name, helpers::current_time_utc());
    match bot.send_message(chat_id, &text[..]).await {
      Ok(_) => info!("[{}] Sent text='{}' to chat_id={}", name, text, chat_id),
      Err(err) => error!("[{}] Unable to send text='{}' to chat_id={} due to {:?}", name, text, chat_id, err),
    }
    info!("[{}] Task has finished at {}", name, helpers::current_time_utc());
  }
}

impl PeriodicTask for PeriodicNotifier {
  fn submit_job(&mut self, when: PeriodicTimeUtc) {
    assert!(self.is_finished(), "should be finished");

    let bot = self.bot.clone();
    let text = self.text.clone();
    let chat_id = self.chat_id;
    let name = self.name.clone();

    let task = move || {
      let cloned_bot = bot.clone();
      let cloned_text = text.clone();
      let cloned_name = name.clone();
      async move {
        PeriodicNotifier::do_notify(cloned_name, cloned_bot, cloned_text, chat_id).await;
      }
    };

    self.when = Some(when.clone());
    self.handle = Some(when.perform_task(task));
  }

  fn description(&self) -> Option<String> {
    self.when().map(|w| format!("Я прошу всех заполнить таблицу {}", w))
  }

  fn task_type(&self) -> &PeriodcTaskType {
    &self.task_type
  }

  fn handle(&self) -> Option<&TaskHandle> {
    self.handle.as_ref()
  }

  fn when(&self) -> Option<&PeriodicTimeUtc> {
    self.when.as_ref()
  }

  fn name(&self) -> &str {
    &self.name[..]
  }
}

/// This task periodically (once a day) sends summary text similar to /todaysummary bot command
pub struct PeriodicSummarySender {
  bot: Bot,
  name: String,
  when: Option<PeriodicTimeUtc>,
  handle: Option<TaskHandle>,
  task_type: PeriodcTaskType,
  chat_id: ChatId,
  dashboard: Arc<LockedDashboard>,
}

impl PeriodicSummarySender {
  fn new(bot: Bot, dashboard: Arc<LockedDashboard>, chat_id: ChatId) -> Self {
    PeriodicSummarySender {
      bot,
      dashboard,
      name: "PeriodicSummarySender".to_string(),
      chat_id,
      when: None,
      handle: None,
      task_type: PeriodcTaskType::Notifier,
    }
  }

  pub async fn send_summary(name: String, bot: Bot, dashboard: Arc<LockedDashboard>, chat_id: ChatId) {
    info!("[{}] Task has started at {}", name, helpers::current_time_utc());
    let locked_dashboard = dashboard.read().await;
    let by_date = helpers::current_time_utc().date_naive(); // always send "today" summary
    match locked_dashboard.summary(&by_date) {
      Ok(summary) => {
        let msg = helpers::format_summary_msg(&summary, &by_date);
        let _ = bot.send_message(chat_id, msg).parse_mode(ParseMode::MarkdownV2).await;
        info!("[{}] Summary has been successfully sent", name);
      }
      Err(DashboardError::EmptyParticipants) => {
        warn!("[{}] The participants were not found", name);
      }
    }
    info!("[{}] Task has finished at {}", name, helpers::current_time_utc());
  }
}

impl PeriodicTask for PeriodicSummarySender {
  fn submit_job(&mut self, when: PeriodicTimeUtc) {
    assert!(self.is_finished(), "should be finished");

    let bot = self.bot.clone();
    let chat_id = self.chat_id;
    let name = self.name.clone();
    let dashboard = self.dashboard.clone();

    let task = move || {
      let cloned_bot = bot.clone();
      let cloned_name = name.clone();
      let cloned_dashboard = dashboard.clone();
      async move {
        PeriodicSummarySender::send_summary(cloned_name, cloned_bot, cloned_dashboard, chat_id).await;
      }
    };

    self.when = Some(when.clone());
    self.handle = Some(when.perform_task(task));
  }

  fn description(&self) -> Option<String> {
    self.when().map(|w| format!("Я отправляю /todaysummary {}", w))
  }

  fn task_type(&self) -> &PeriodcTaskType {
    &self.task_type
  }

  fn handle(&self) -> Option<&TaskHandle> {
    self.handle.as_ref()
  }

  fn when(&self) -> Option<&PeriodicTimeUtc> {
    self.when.as_ref()
  }

  fn name(&self) -> &str {
    &self.name[..]
  }
}
