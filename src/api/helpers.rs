use chrono::{DateTime, Datelike, Utc};
use google_sheets4::api::Sheet;
use log::{debug, info, trace};

struct Month {
  ru_name: &'static str,
  eng_name: &'static str,
  number: u8,
}

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

pub fn get_sheet_id_by_title(sheets: &Vec<Sheet>, title: &str) -> Option<i32> {
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
  format!("{}", current_time().format(format))
}

pub fn current_time() -> DateTime<Utc> {
  chrono::Utc::now()
}

pub fn derive_title_name() -> String {
  let current_time = current_time();
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
