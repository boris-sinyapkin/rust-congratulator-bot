use super::{score_table::entities::Person, Dashboard, ScoreTable, ScoreTableRecord};
use crate::helpers;

pub struct DashboardAnalyzer<'a> {
  dashboard: &'a Dashboard,
}

impl<'a> DashboardAnalyzer<'a> {
  pub fn new(dashboard: &'a Dashboard) -> Self {
    Self { dashboard }
  }

  pub fn participants(&self) -> Option<Vec<&'a Person>> {
    if let Some(tables) = self.dashboard.tables() {
      return Some(tables.iter().map(|t| t.person()).collect());
    }
    None
  }

  pub fn last_filled_score_table_record(&self, person: &Person) -> Option<&'a ScoreTableRecord> {
    match self.find_table(person) {
      Some(table) => table.last_filled_record(),
      _ => None,
    }
  }

  pub fn today_filled_score_table_record(&self, person: &Person) -> Option<&'a ScoreTableRecord> {
    match self.find_table(person) {
      Some(table) => table
        .by_date(&helpers::current_time().date_naive())
        .filter(|&record| record.has_total()),
      _ => None,
    }
  }

  pub fn get_person_by_name(&self, name: &str) -> Option<&'a Person> {
    if let Some(tables) = self.dashboard.tables() {
      return tables
        .iter()
        .find(|t| t.person().name() == name)
        .map(|found_table| found_table.person());
    }
    None
  }

  pub fn find_table(&self, person: &Person) -> Option<&'a ScoreTable> {
    if let Some(tables) = self.dashboard.tables() {
      return tables.iter().find(|t| t.person() == person);
    }
    None
  }
}
