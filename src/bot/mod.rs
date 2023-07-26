pub mod error;

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

use crate::{
  api::AsyncSheetsHub,
  bot::error::CongratulatorError as Error,
  dashboard::{
    score_table::{entities::Person, ScoreTableRecord},
    Dashboard,
  },
};

#[derive(Clone, Default)]
pub enum State {
  #[default]
  Default,
  ReceiveSelectedUser,
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
}

type CongratulatorDialogue = Dialogue<State, InMemStorage<State>>;
type CongratulatorHandlerError = Box<dyn std::error::Error + Send + Sync>;
type CongratulatorHandlerResult = Result<(), CongratulatorHandlerError>;

pub struct Congratulator {
  bot: Bot,
  dispatcher: Dispatcher<Bot, CongratulatorHandlerError, DefaultKey>,
  dashboard: Arc<Dashboard>,
}

impl Congratulator {
  pub async fn new() -> Result<Self, Error> {
    info!("[Congratulator] Bot is getting created");
    let hub = AsyncSheetsHub::new().await?;
    let dashboard = Arc::new(hub.fetch_dashboard().await?);

    let bot = Bot::from_env();
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

    let bot = Congratulator {
      bot,
      dispatcher,
      dashboard,
    };

    info!("[Congratulator] Bot successfully created");
    Ok(bot)
  }

  pub async fn listen(&mut self) {
    info!("[Congratulator] Bot is starting dispatching events...");
    self.dispatcher.dispatch().await
  }

  pub fn initialized(&self) -> bool {
    self.dashboard.tables().is_some()
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

  async fn participants(bot: Bot, msg: Message, dashboard: Arc<Dashboard>) -> CongratulatorHandlerResult {
    let chat_id = msg.chat.id;
    info!("[Congratulator] Sending participants to chat_id={}", chat_id);
    let msg = match dashboard.participants_names() {
      Some(names) => join(&names, "\n"),
      None => "There are no participants found".to_string(),
    };
    bot.send_message(chat_id, msg).await?;
    Ok(())
  }

  async fn scores(bot: Bot, msg: Message, dialog: CongratulatorDialogue, dashboard: Arc<Dashboard>) -> CongratulatorHandlerResult {
    let chat_id = msg.chat.id;
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
        dialog.update(State::ReceiveSelectedUser).await?;
      }
      None => {
        warn!("[Congratulator][Scores] The participants were not found");
        bot.send_message(chat_id, "Список пользователей пуст 😩😭").await?;
      }
    }

    info!("[Congratulator][Scores] Finished handling (chat_id={})", chat_id);
    Ok(())
  }

  async fn invalid_state(bot: Bot, msg: Message) -> CongratulatorHandlerResult {
    bot
      .send_message(
        msg.chat.id,
        "Either I can't handle this message so far😥 or you probably need \
        to select something from the list above👆🏻🙏😥 Type /help to see the usage.",
      )
      .await?;
    Ok(())
  }

  async fn receive_user_selected(
    bot: Bot,
    dialog: CongratulatorDialogue,
    callback_query: CallbackQuery,
    dashboard: Arc<Dashboard>,
  ) -> CongratulatorHandlerResult {
    let chat_id = dialog.chat_id();
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
    match dashboard.last_score_table_record(person) {
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
          .send_message(chat_id, "За последний месяц не было заполнено *ни одного* дня 😢")
          .parse_mode(ParseMode::MarkdownV2)
          .await?;
      }
    }

    dialog.update(State::Default).await?;
    info!("[Congratulator][ReceiveSelectedUser] Finished handling (chat_id={})", chat_id);
    Ok(())
  }

  fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
      .branch(case![Command::Help].endpoint(Congratulator::help))
      .branch(case![Command::Dice].endpoint(Congratulator::dice))
      .branch(case![Command::Participants].endpoint(Congratulator::participants))
      .branch(case![Command::Scores].endpoint(Congratulator::scores));

    let message_handler = Update::filter_message()
      .branch(command_handler)
      .branch(dptree::endpoint(Congratulator::invalid_state));

    let callback_query_handler =
      Update::filter_callback_query().branch(case![State::ReceiveSelectedUser].endpoint(Congratulator::receive_user_selected));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
      .branch(message_handler)
      .branch(callback_query_handler)
  }

  fn format_user_score_msg(score_table: &ScoreTableRecord, person: &Person) -> String {
    format!("🫥 __Пользователь__: {}\n{}", person.name(), score_table)
      .replace('-', "\\-")
      .replace('.', "\\.")
  }
}
