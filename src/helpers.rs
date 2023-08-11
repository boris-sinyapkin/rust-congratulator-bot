use std::{fmt::Display, future::Future};

use crate::{
  bot::tasks::TaskHandle,
  dashboard::score_table::{entities::Person, ScoreTableRecord},
};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};
use google_sheets4::api::Sheet;
use itertools::free::join;
use log::{debug, info, trace};
use tokio_schedule::{every, EveryDay, EveryMinute, Job};

pub type EveryDayTime = EveryDay<Utc, Local>;
pub type EveryMinuteTime = EveryMinute<Utc, Local>;

#[allow(dead_code)]
struct Month {
  ru_name: &'static str,
  eng_name: &'static str,
  number: u8,
}

#[allow(dead_code)]
impl Month {
  fn new(number: u8) -> Month {
    match number {
      1 => Month {
        ru_name: "Январь",
        eng_name: "January",
        number,
      },
      2 => Month {
        ru_name: "Февраль",
        eng_name: "February",
        number,
      },
      3 => Month {
        ru_name: "Март",
        eng_name: "March",
        number,
      },
      4 => Month {
        ru_name: "Апрель",
        eng_name: "April",
        number,
      },
      5 => Month {
        ru_name: "Май",
        eng_name: "May",
        number,
      },
      6 => Month {
        ru_name: "Июнь",
        eng_name: "June",
        number,
      },
      7 => Month {
        ru_name: "Июль",
        eng_name: "July",
        number,
      },
      8 => Month {
        ru_name: "Август",
        eng_name: "August",
        number,
      },
      9 => Month {
        ru_name: "Сентябрь",
        eng_name: "September",
        number,
      },
      10 => Month {
        ru_name: "Октябрь",
        eng_name: "October",
        number,
      },
      11 => Month {
        ru_name: "Ноябрь",
        eng_name: "November",
        number,
      },
      12 => Month {
        ru_name: "Декабрь",
        eng_name: "December",
        number,
      },
      _ => panic!("Undefined month number: {number:?}"),
    }
  }

  fn get_ru(&self) -> &'static str {
    self.ru_name
  }

  fn get_en(&self) -> &'static str {
    self.eng_name
  }

  fn get_num(&self) -> u8 {
    self.number
  }

  fn prev(&self) -> Month {
    let next_number = if self.number == 1 { 12 } else { self.number - 1 };
    Month::new(next_number)
  }

  fn next(&self) -> Month {
    let next_number = if self.number == 12 { 1 } else { self.number + 1 };
    Month::new(next_number)
  }
}

pub fn get_sheet_id_by_title(sheets: &[Sheet], title: &str) -> Option<i32> {
  for sheet in sheets.iter() {
    if let Some(props) = &sheet.properties {
      match &props.title {
        Some(t) => {
          if t == title {
            debug!("[API] Found sheet_id={:?} for title={:?}", props.sheet_id, title);
            return props.sheet_id;
          } else {
            trace!("[API] Current title={:?} != target={:?}", t, title);
          }
        }
        None => continue,
      }
    }
  }
  debug!("[API] Sheet id was not found for title={:?}", title);
  None
}

pub fn current_time_format(format: &str) -> String {
  format!("{}", current_time_utc().format(format))
}

pub fn current_time_utc() -> DateTime<Utc> {
  chrono::Utc::now()
}

pub fn current_time_utc_msk() -> DateTime<Utc> {
  current_time_utc() + Duration::hours(3)
}

pub fn derive_title_name() -> String {
  let current_time = current_time_utc();
  let month_number: u8 = current_time.month().try_into().unwrap();
  let year_number: u16 = current_time.year().try_into().unwrap();

  let month = Month::new(month_number);
  let year_str = year_number.to_string();
  let year_str = &year_str[year_str.len() - 2..];

  debug!(
    "[API] Deriving relevant title name: current_time={:}, corresponding month(ru)={:}({:}), year={:}",
    current_time.format("%d.%m.%Y"),
    month.get_en(),
    month.get_ru(),
    year_str
  );

  let result = format!("{:} {:}", month.get_ru(), year_str);
  info!("[API] Derived relevant title name = {:?}", result);

  result
}

pub fn format_user_score_msg(score_table: &ScoreTableRecord, person: &Person) -> String {
  format!("🫥 __Пользователь__: {}\n{}", person.name(), score_table)
    .replace('-', "\\-")
    .replace('.', "\\.")
}

pub fn format_summary_msg(summary: &Vec<String>, by_date: &NaiveDate) -> String {
  if !summary.is_empty() {
    join(summary, "\n")
  } else {
    format!(
      "*{}* пока еще *ни один* из участников таблицу не заполнял 😩😭",
      by_date.format("%d.%m.%Y")
    )
    .replace('.', "\\.")
  }
}

#[derive(Debug, Clone)]
pub enum PeriodicTimeUtc {
  EveryDay(EveryDayTime, u32, u32, u32),
  EveryMin(EveryMinuteTime, u32),
}

impl PeriodicTimeUtc {
  pub fn every_day_time_utc(h: u32, m: u32, s: u32) -> Self {
    let every_day = every(1).day().at(h, m, s).in_timezone(&Utc);

    PeriodicTimeUtc::EveryDay(every_day, h, m, s)
  }

  pub fn every_min_time_utc(period: u32) -> Self {
    let every_min = every(period).minutes().in_timezone(&Utc);

    PeriodicTimeUtc::EveryMin(every_min, period)
  }

  pub fn perform_task<Fut, F>(self, func: F) -> TaskHandle
  where
    Fut: Future<Output = ()> + Send + 'static,
    F: FnMut() -> Fut + Send + 'static,
  {
    let job = match self {
      PeriodicTimeUtc::EveryDay(t, _, _, _) => t.perform(func),
      PeriodicTimeUtc::EveryMin(t, _) => t.perform(func),
    };
    tokio::spawn(job)
  }
}

impl Display for PeriodicTimeUtc {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      PeriodicTimeUtc::EveryDay(_, h, m, s) => write!(f, "ежедневно в {h:02}:{m:02}:{s:02} UTC"),
      PeriodicTimeUtc::EveryMin(_, period) => write!(f, "каждые {period} минут(ы)"),
    }
  }
}
