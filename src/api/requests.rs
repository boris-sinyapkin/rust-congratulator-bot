use google_sheets4::api::{DataFilter, GetSpreadsheetByDataFilterRequest, GridRange};
use log::{debug, trace};

// Build requests for a particular sheet in Google Spreadsheet
pub struct RequestFactory {
  sheet_id: i32,
}

impl RequestFactory {
  pub fn new(sheet_id: i32) -> Self {
    RequestFactory { sheet_id }
  }

  pub fn construct_score_table_request(&self, include_grid_data: bool) -> ScoreTableRequest {
    trace!("[RequestFactory] Building new ScoreTableRequest request");
    let req = ScoreTableRequest::new(self.sheet_id, include_grid_data);
    trace!("[RequestFactory] ScoreTableRequest data {:#?}", req);
    req
  }
}

#[derive(Debug)]
pub struct ScoreTableRequest {
  start_column_index: i32,
  end_column_index: i32,
  start_row_index: i32,
  end_row_index: i32,
  sheet_id: i32,
  include_grid_data: bool,
}

impl ScoreTableRequest {
  const COLUMN_OFFSET: i32 = 10;
  const INITIAL_START_COLUMN_INDEX: i32 = 1;
  const INITIAL_END_COLUMN_INDEX: i32 = 10;
  const INITIAL_START_ROW_INDEX: i32 = 3;
  const INITIAL_END_ROW_INDEX: i32 = 35;

  fn new(sheet_id: i32, include_grid_data: bool) -> Self {
    Self {
      start_column_index: ScoreTableRequest::INITIAL_START_COLUMN_INDEX,
      end_column_index: ScoreTableRequest::INITIAL_END_COLUMN_INDEX,
      start_row_index: ScoreTableRequest::INITIAL_START_ROW_INDEX,
      end_row_index: ScoreTableRequest::INITIAL_END_ROW_INDEX,
      sheet_id,
      include_grid_data,
    }
  }

  /// Applies an offset to the current request coordinates
  pub fn next_table_request(&mut self) {
    self.start_column_index += ScoreTableRequest::COLUMN_OFFSET;
    self.end_column_index += ScoreTableRequest::COLUMN_OFFSET;
    trace!("[ScoreTableRequest] Updated ScoreTableRequest data {:#?}", self);
  }

  pub fn build(&self) -> GetSpreadsheetByDataFilterRequest {
    let grid_range = GridRange {
      end_column_index: Some(self.end_column_index),
      end_row_index: Some(self.end_row_index),
      sheet_id: Some(self.sheet_id),
      start_column_index: Some(self.start_column_index),
      start_row_index: Some(self.start_row_index),
    };

    let data_filter = DataFilter {
      a1_range: None,
      developer_metadata_lookup: None,
      grid_range: Some(grid_range),
    };

    GetSpreadsheetByDataFilterRequest {
      data_filters: Some(vec![data_filter]),
      include_grid_data: Some(self.include_grid_data),
    }
  }
}
