use chrono::NaiveDate;
use google_sheets4::api::{CellData, NumberFormat};

use self::entities::{Percentage, Person, Scores};
use self::error::{Empty::*, InvalidCell::*, ParseError::*, ScoreTableRecordError as Error};

pub mod entities;
pub mod error;

pub struct ScoreTable {
  person: Person,
  table: Vec<ScoreTableRecord>,
}

impl ScoreTable {
  pub fn new(person: Person, table: Vec<ScoreTableRecord>) -> ScoreTable {
    ScoreTable { person, table }
  }

  pub fn person(&self) -> &Person {
    &self.person
  }

  pub fn last_record(&self) -> Option<&ScoreTableRecord> {
    self.table.last()
  }

  pub fn last_filled_record(&self) -> Option<&ScoreTableRecord> {
    self.table.iter().rev().find(|rec| rec.has_total())
  }

  pub fn by_date(&self, date: &NaiveDate) -> Option<&ScoreTableRecord> {
    self.table.iter().rev().find(|rec| rec.date == *date)
  }
}

#[derive(Debug, Default)]
pub struct ScoreTableRecord {
  date: NaiveDate,
  scores: Scores,
  total_score: f64,
  percent: Percentage,
}

impl ScoreTableRecord {
  pub fn new(date: NaiveDate, scores: Scores, total_score: f64, percent: Percentage) -> ScoreTableRecord {
    ScoreTableRecord {
      date,
      scores,
      total_score,
      percent,
    }
  }

  pub fn has_total(&self) -> bool {
    self.total_score != Scores::UNITITIALIZED_SCORE
  }

  pub fn percent(&self) -> &Percentage {
    &self.percent
  }

  pub fn from_vec(row: &[CellData]) -> Result<ScoreTableRecord, Error> {
    let mut date: NaiveDate = NaiveDate::default();
    let mut scores = Scores::default();
    let mut total_score = Scores::UNITITIALIZED_SCORE;
    let mut percent = Percentage::from(0);

    // Iterate over cells in a row
    for (i, cell) in row.iter().enumerate() {
      match i {
        0 => date = ScoreTableRecord::parse_date(cell)?,
        1..=7 => {
          let score = ScoreTableRecord::parse_score(cell, i)?;
          match i {
            1 => scores.set_sport(score),
            2 => scores.set_professional_growth(score),
            3 => scores.set_health(score),
            4 => scores.set_spiritual_growth(score),
            5 => scores.set_foreign_language(score),
            6 => scores.set_personal_dev(score),
            7 => total_score = score,
            _ => panic!("Should not reach here"),
          };
        }
        8 => percent = ScoreTableRecord::parse_percentage(cell)?,
        _ => return Err(Error::UnexpectedFieldIndex(i)),
      }
    }

    Ok(ScoreTableRecord::new(date, scores, total_score, percent))
  }

  fn parse_date(cell: &CellData) -> Result<NaiveDate, Error> {
    let cell_format = cell.effective_format.as_ref().ok_or(Error::Empty(EmptyEffectiveFormat(0)))?;

    if let Some(NumberFormat { pattern: _, type_ }) = &cell_format.number_format {
      let date = match type_.as_ref().unwrap().as_str() {
        "DATE" => {
          let formatted_value = match cell.formatted_value.as_ref() {
            Some(value) => value,
            None => return Err(Error::InvalidCell(InvalidDateCell("can't be empty formatted value for date"))),
          };
          // TODO: date format should be derived dynamically from 'pattern'
          let parsed_date = match NaiveDate::parse_from_str(formatted_value.as_str(), "%d.%m.%Y") {
            Ok(date) => date,
            Err(parse_err) => return Err(Error::ParseError(DateParseError(parse_err.kind()))),
          };
          parsed_date
        }
        _ => return Err(Error::InvalidCell(InvalidDateCell("google API cell type is other than DATE"))),
      };
      Ok(date)
    } else {
      Err(Error::InvalidCell(InvalidDateCell("date cell should have NumberFormat")))
    }
  }

  fn parse_percentage(cell: &CellData) -> Result<Percentage, Error> {
    let percent_value: Result<i32, _> = match &cell.formatted_value {
      Some(value) => {
        if !value.ends_with('%') {
          return Err(Error::InvalidCell(InvalidPercentCell("percent cell should end up with %")));
        } else {
          value[..value.len() - 1].parse()
        }
      }
      None => Ok(0),
    };
    let percent_value = match percent_value {
      Ok(value) => Percentage::from(value),
      Err(erro) => return Err(Error::ParseError(PercentParseError(erro.kind().clone()))),
    };
    Ok(percent_value)
  }

  fn parse_score(cell: &CellData, index: usize) -> Result<f64, Error> {
    let score = match &cell.formatted_value {
      Some(value) => value.parse::<f64>(),
      None => Ok(Scores::UNITITIALIZED_SCORE),
    };
    let score = match score {
      Ok(value) => value,
      Err(erro) => {
        let derived_error = Error::ParseError(ScoreParseError(index, erro));
        if let Some(effective_value) = &cell.effective_value {
          return effective_value.number_value.ok_or(derived_error);
        }
        return Err(derived_error);
      }
    };
    Ok(score)
  }
}

impl std::fmt::Display for ScoreTableRecord {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "🗓️ __Дата__: {}\n\n{}\n\
       ✅ *Total*: {}\n\
       💯 *Rate*: {} {}\n",
      self.date.format("%d.%m.%Y"),
      self.scores,
      self.total_score,
      self.percent,
      self.percent.emoji()
    )
  }
}
