use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

#[derive(Debug)]
pub struct Person {
  id: u64,
  name: String,
}

impl Person {
  pub fn new(name: String) -> Person {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    Person { id: hasher.finish(), name }
  }

  pub fn name(&self) -> &str {
    &self.name
  }
}

impl PartialEq for Person {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id && self.name == other.name
  }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Percentage {
  value: i32,
}

impl Percentage {
  pub fn value(&self) -> i32 {
    self.value
  }

  pub fn from(value: i32) -> Percentage {
    Percentage { value }
  }
}

impl std::fmt::Display for Percentage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}%", self.value)
  }
}

#[derive(Debug)]
pub struct Scores {
  sport: f64,
  professional_growth: f64,
  health: f64,
  spiritual_growth: f64,
  foreign_language: f64,
  personal_dev: f64,
}

impl Scores {
  pub const UNITITIALIZED_SCORE: f64 = 0.;

  pub fn set_sport(&mut self, value: f64) {
    self.sport = value;
  }

  pub fn set_professional_growth(&mut self, value: f64) {
    self.professional_growth = value;
  }

  pub fn set_health(&mut self, value: f64) {
    self.health = value;
  }

  pub fn set_spiritual_growth(&mut self, value: f64) {
    self.spiritual_growth = value;
  }

  pub fn set_foreign_language(&mut self, value: f64) {
    self.foreign_language = value;
  }

  pub fn set_personal_dev(&mut self, value: f64) {
    self.personal_dev = value;
  }

  pub fn total(&self) -> f64 {
    self.sport + self.personal_dev + self.health + self.spiritual_growth + self.foreign_language + self.personal_dev
  }
}

impl Default for Scores {
  fn default() -> Self {
    Scores {
      sport: Scores::UNITITIALIZED_SCORE,
      professional_growth: Scores::UNITITIALIZED_SCORE,
      health: Scores::UNITITIALIZED_SCORE,
      spiritual_growth: Scores::UNITITIALIZED_SCORE,
      foreign_language: Scores::UNITITIALIZED_SCORE,
      personal_dev: Scores::UNITITIALIZED_SCORE,
    }
  }
}

impl std::fmt::Display for Scores {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(
      f,
      "🏅 Спорт: {}\n\
       👨‍💻 Проф. рост: {}\n\
       🌿 Здоровье: {}\n\
       🛐 Дух. рост: {}\n\
       📚 Ин. языки: {}\n\
       🤸 Свое: {}",
      self.sport, self.professional_growth, self.health, self.spiritual_growth, self.foreign_language, self.personal_dev
    )
  }
}
