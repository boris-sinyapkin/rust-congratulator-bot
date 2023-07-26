use self::{
  analyzer::DashboardAnalyzer,
  score_table::ScoreTableRecord,
  score_table::{entities::Person, ScoreTable},
};
use log::{error, trace};

pub mod analyzer;
pub mod score_table;

#[derive(Default)]
pub struct Dashboard {
  score_tables: Option<Vec<ScoreTable>>,
}

impl Dashboard {
  pub fn from(score_tables: Vec<ScoreTable>) -> Self {
    Self {
      score_tables: Some(score_tables),
    }
  }

  pub fn initialize(&mut self, score_tables: Vec<ScoreTable>) -> bool {
    if let None = self.score_tables {
      trace!("[Dashboard] Initialization done. (tables amount = {})", score_tables.len());
      self.score_tables = Some(score_tables);
      return true;
    }
    error!("[Dashboard] Initialization failed. (Already initialized)");
    false
  }

  pub fn tables(&self) -> Option<&Vec<ScoreTable>> {
    self.score_tables.as_ref()
  }

  pub fn find_table(&self, person: &Person) -> Option<&ScoreTable> {
    self.build_analyzer().find_table(person)
  }

  pub fn get_person_by_name(&self, name: &str) -> Option<&Person> {
    self.build_analyzer().get_person_by_name(name)
  }

  pub fn last_filled_score_table_record(&self, person: &Person) -> Option<&ScoreTableRecord> {
    self.build_analyzer().last_filled_score_table_record(person)
  }

  /// Return list of the participants
  pub fn participants(&self) -> Option<Vec<&Person>> {
    self.build_analyzer().participants()
  }

  pub fn participants_names(&self) -> Option<Vec<&str>> {
    self.participants().map(|persons| persons.iter().map(|p| p.name()).collect())
  }

  pub fn build_analyzer(&self) -> DashboardAnalyzer {
    DashboardAnalyzer::new(self)
  }
}
