pub mod config;
pub mod error;
pub mod tasks;

use chrono::NaiveDate;
use itertools::free::join;
use log::{debug, error, info, trace, warn};
use std::sync::Arc;
use teloxide::{
  dispatching::{
    dialogue::{self, InMemStorage},
    DefaultKey, UpdateFilterExt, UpdateHandler,
  },
  prelude::*,
  types::ParseMode,
  types::{InlineKeyboardButton, InlineKeyboardMarkup},
  utils::command::BotCommands,
};
use tokio::sync::RwLock;

use crate::{
  api::AsyncSheetsHub,
  bot::{
    error::CongratulatorError as Error,
    tasks::{EveryDayTime, PeriodicNotifier},
  },
  dashboard::{
    score_table::{entities::Person, ScoreTableRecord},
    Dashboard,
  },
  helpers::current_time,
};

use self::{config::CongratulatorConfig, tasks::PeriodicDataFetcher};

#[derive(Clone, Default)]
pub enum State {
  #[default]
  Default,
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
  #[command(description = "display this text")]
  Help,
  #[command(description = "just roll the dice")]
  Dice,
  #[command(description = "show list of participants")]
  Participants,
  #[command(description = "show scores of a participant")]
  Scores,
  #[command(description = "show score summary for today")]
  TodaySummary,
  #[command(description = "show score summary for yesterday")]
  YesterdaySummary,
}

type CongratulatorDialogue = Dialogue<State, InMemStorage<State>>;
type CongratulatorHandlerError = Box<dyn std::error::Error + Send + Sync>;
type CongratulatorHandlerResult = Result<(), CongratulatorHandlerError>;
type LockedDashboard = RwLock<Dashboard>;

#[allow(dead_code)]
pub struct Congratulator {
  bot: Bot,
  dispatcher: Dispatcher<Bot, CongratulatorHandlerError, DefaultKey>,
  dashboard: Arc<LockedDashboard>,
  fetcher: PeriodicDataFetcher,
  notifiers: Vec<PeriodicNotifier>,
}

impl Congratulator {
  pub async fn new(cfg: CongratulatorConfig) -> Result<Self, Error> {
    info!("[Congratulator] Bot is getting created");

    // Create Hub to fetch the data
    let hub = Arc::new(AsyncSheetsHub::new(cfg.api_service_key(), cfg.spreadsheet_id()).await?);

    // Create shared data - the Dashboard
    let dashboard = Arc::new(RwLock::new(hub.fetch_dashboard().await?));

    // Create periodic task that will fetch the data periodically
    // Schedule every amount of minutes specified in API_DATA_FETCH_TASK_INTERVAL_MIN env variable
    let fetcher = PeriodicDataFetcher::schedule(cfg.fetch_data_interval_min(), hub.clone(), dashboard.clone());

    // Create Bot instance
    let bot = Bot::new(cfg.bot_token_str());

    // Create periodic tasks that send a particular message at some time
    let notifiers = vec![PeriodicNotifier::schedule(
      bot.clone(),
      "Fill in the table 📋".to_string(),
      cfg.notify_chat_id(),
      EveryDayTime::new(18, 0, 0), // MSK = UTC+3
    )];

    bot.set_my_commands(Command::bot_commands()).await?;
    let dispatcher = Dispatcher::builder(bot.clone(), Congratulator::schema())
      .dependencies(dptree::deps![InMemStorage::<State>::new(), dashboard.clone()])
      .default_handler(|upd| async move {
        warn!("[Congratulator] Unhandled update: {:?}", upd);
      })
      .error_handler(LoggingErrorHandler::with_custom_text(
        "[Congratulator] Error has occurred in the dispatcher",
      ))
      .enable_ctrlc_handler()
      .build();

    let congratulator = Congratulator {
      bot,
      dispatcher,
      dashboard,
      fetcher,
      notifiers,
    };

    info!("[Congratulator] Bot successfully created");
    Ok(congratulator)
  }

  pub async fn listen(&mut self) {
    info!("[Congratulator] Bot is starting dispatching events...");
    self.dispatcher.dispatch().await
  }

  pub async fn initialized(&self) -> bool {
    self.dashboard.read().await.tables().is_some()
  }

  async fn help(bot: Bot, msg: Message) -> CongratulatorHandlerResult {
    info!("[Congratulator] Sending help to chat_id={}", msg.chat.id);
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
  }

  async fn dice(bot: Bot, msg: Message) -> CongratulatorHandlerResult {
    info!("[Congratulator] Sending dice to chat_id={}", msg.chat.id);
    bot.send_dice(msg.chat.id).await?;
    Ok(())
  }

  async fn participants(bot: Bot, msg: Message, locked_dashboard: Arc<LockedDashboard>) -> CongratulatorHandlerResult {
    let chat_id = msg.chat.id;
    let dashboard = locked_dashboard.read().await;
    info!("[Congratulator] Sending participants to chat_id={}", chat_id);
    let msg = match dashboard.participants_names() {
      Some(names) => join(&names, "\n"),
      None => "There are no participants found".to_string(),
    };
    bot.send_message(chat_id, msg).await?;
    Ok(())
  }

  async fn scores(bot: Bot, msg: Message, locked_dashboard: Arc<LockedDashboard>) -> CongratulatorHandlerResult {
    let chat_id = msg.chat.id;
    let dashboard = locked_dashboard.read().await;
    info!("[Congratulator][Scores] Start handling Scores (chat_id={})", chat_id);
    match dashboard.participants() {
      Some(persons) => {
        debug!("[Congratulator][Scores] Found {} participants", persons.len());
        let choices = persons
          .iter()
          .map(|person| InlineKeyboardButton::callback(person.name(), person.name()));
        bot
          .send_message(
            msg.chat.id,
            "Могу показать последнюю статистику для какого\\-нибудь *конкретного* \
            пользователя из списка ниже\\. Чьи цифры будем смотреть?",
          )
          .parse_mode(ParseMode::MarkdownV2)
          .reply_markup(InlineKeyboardMarkup::new([choices]))
          .await?;
      }
      None => {
        warn!("[Congratulator][Scores] The participants were not found");
        bot.send_message(chat_id, "Список пользователей пуст 😩😭").await?;
      }
    }

    info!("[Congratulator][Scores] Finished handling (chat_id={})", chat_id);
    Ok(())
  }

  async fn today_summary(bot: Bot, msg: Message, locked_dashboard: Arc<LockedDashboard>) -> CongratulatorHandlerResult {
    Congratulator::summary(bot, msg, locked_dashboard, &current_time().date_naive()).await
  }

  async fn yesterday_summary(bot: Bot, msg: Message, locked_dashboard: Arc<LockedDashboard>) -> CongratulatorHandlerResult {
    if let Some(yesterday) = current_time().date_naive().pred_opt() {
      Congratulator::summary(bot, msg, locked_dashboard, &yesterday).await?
    } else {
      error!("Unable to handle YesterdaySummary: can't derive the date for yesterday");
    }
    Ok(())
  }

  async fn summary(bot: Bot, msg: Message, locked_dashboard: Arc<LockedDashboard>, by_date: &NaiveDate) -> CongratulatorHandlerResult {
    let chat_id = msg.chat.id;
    let dashboard = locked_dashboard.read().await;
    info!(
      "[Congratulator][Summary] Start handling Summary (chat_id={}) for date='{}'",
      chat_id, by_date
    );
    match dashboard.participants() {
      Some(persons) => {
        debug!("[Congratulator][Summary] Found {} participants", persons.len());
        let summary: Vec<String> = persons
          .iter()
          .filter_map(|p| {
            dashboard
              .find_filled_score_table_record(p, by_date)
              .map(|rec| format!("{} молодец на {} {}", p.name(), rec.percent(), rec.percent().emoji()))
          })
          .collect();

        let msg = if summary.is_empty() {
          format!(
            "*{}* пока еще *ни один* из участников таблицу не заполнял 😩😭",
            by_date.format("%d.%m.%Y")
          )
          .replace('.', "\\.")
        } else {
          join(summary, "\n")
        };

        bot.send_message(chat_id, msg).parse_mode(ParseMode::MarkdownV2).await?;
      }
      None => {
        warn!("[Congratulator][Summary] The participants were not found");
        bot.send_message(chat_id, "Список пользователей пуст 😩😭").await?;
      }
    }
    info!("[Congratulator][Summary] Finished handling (chat_id={})", chat_id);
    Ok(())
  }

  async fn unhandled_message(_bot: Bot, msg: Message) -> CongratulatorHandlerResult {
    warn!("Called unhandled_message() callback with msg={:?}", msg);
    Ok(())
  }

  async fn my_chat_member_update_handler(bot: Bot, update: ChatMemberUpdated) -> CongratulatorHandlerResult {
    let ChatMemberUpdated {
      chat,
      from,
      new_chat_member,
      ..
    } = &update;
    let chat_id = chat.id;

    info!("[Congratulator][ChatMemberUpdated] Start handling update (chat_id={})", chat_id);
    let msg = if new_chat_member.is_member() {
      format!("Благодаря *{}* я теперь в этом чатике\\. Большое спасибо\\!", from.full_name())
    } else {
      debug!("[Congratulator][ChatMemberUpdated] The update was unhandled {:?}", update);
      return Ok(());
    };

    bot.send_message(chat_id, msg).parse_mode(ParseMode::MarkdownV2).await?;
    Ok(())
  }

  async fn receive_user_selected(
    bot: Bot,
    dialog: CongratulatorDialogue,
    callback_query: CallbackQuery,
    locked_dashboard: Arc<LockedDashboard>,
  ) -> CongratulatorHandlerResult {
    let chat_id = dialog.chat_id();
    let dashboard = locked_dashboard.read().await;
    info!(
      "[Congratulator][ReceiveSelectedUser] Handling state from User={:?} (chat_id={})",
      callback_query.from, chat_id
    );

    let callback_data: String = callback_query.data.ok_or_else(|| {
      error!("[Congratulator][ReceiveSelectedUser] Received None in callback data");
      Error::EmptyCallbackData
    })?;

    let person = dashboard.get_person_by_name(&callback_data[..]).ok_or_else(|| {
      error!("[Congratulator][ReceiveSelectedUser] Person was not found");
      Error::PersonNotFound
    })?;

    debug!("[Congratulator][ReceiveSelectedUser] Selected person = {:?}", person);
    match dashboard.last_filled_score_table_record(person) {
      Some(last_record) => {
        trace!("[Congratulator][ReceiveSelectedUser] Found {:?}", last_record);
        bot
          .send_message(chat_id, Congratulator::format_user_score_msg(last_record, person))
          .parse_mode(ParseMode::MarkdownV2)
          .await?;
      }
      None => {
        warn!(
          "[Congratulator][ReceiveSelectedUser] Last score record was not found for {:?}",
          person
        );
        bot
          .send_message(
            chat_id,
            format!("*{}* не заполнил\\(а\\) *ни одного* дня за последний месяц 😢", person.name()),
          )
          .parse_mode(ParseMode::MarkdownV2)
          .await?;
      }
    }

    bot.answer_callback_query(callback_query.id).send().await?;
    info!("[Congratulator][ReceiveSelectedUser] Finished handling (chat_id={})", chat_id);
    Ok(())
  }

  fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
      .branch(case![Command::Help].endpoint(Congratulator::help))
      .branch(case![Command::Dice].endpoint(Congratulator::dice))
      .branch(case![Command::Participants].endpoint(Congratulator::participants))
      .branch(case![Command::Scores].endpoint(Congratulator::scores))
      .branch(case![Command::TodaySummary].endpoint(Congratulator::today_summary))
      .branch(case![Command::YesterdaySummary])
      .endpoint(Congratulator::yesterday_summary);

    let updates_handler = Update::filter_my_chat_member().branch(dptree::endpoint(Congratulator::my_chat_member_update_handler));

    let message_handler = Update::filter_message()
      .branch(command_handler)
      .branch(dptree::endpoint(Congratulator::unhandled_message));

    let callback_query_handler =
      Update::filter_callback_query().branch(case![State::Default].endpoint(Congratulator::receive_user_selected));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
      .branch(updates_handler)
      .branch(message_handler)
      .branch(callback_query_handler)
  }

  fn format_user_score_msg(score_table: &ScoreTableRecord, person: &Person) -> String {
    format!("🫥 __Пользователь__: {}\n{}", person.name(), score_table)
      .replace('-', "\\-")
      .replace('.', "\\.")
  }
}

impl Drop for Congratulator {
  fn drop(&mut self) {
    debug!("[Congratulator] Dropping ...");
    self.fetcher.cancel();
    for notifier in self.notifiers.iter() {
      notifier.cancel()
    }
  }
}
