pub mod error;
pub mod helpers;
pub mod requests;
pub mod task;

use google_sheets4::{
  api::{GetSpreadsheetByDataFilterRequest, GridData, RowData, Spreadsheet},
  oauth2::{self, authenticator::Authenticator},
  Sheets,
};
use hyper::{client::HttpConnector, Client};
use log::{debug, error, info, trace, warn};

use crate::{
  api::error::{AsyncSheetsHubError as Error, InvalidFetchedData::*},
  api::requests::RequestFactory,
  dashboard::{score_table::entities::Person, score_table::ScoreTable, score_table::ScoreTableRecord, Dashboard},
};

use self::requests::ScoreTableRequest;

async fn create_hub(service_key: &str) -> Result<Sheets<hyper_rustls::HttpsConnector<HttpConnector>>, Error> {
  let connector = hyper_rustls::HttpsConnector::with_native_roots();
  let client = Client::builder().build(connector);
  let auth = self::auth(service_key).await?;

  Ok(Sheets::new(client, auth))
}

async fn auth(service_key: &str) -> Result<Authenticator<google_sheets4::hyper_rustls::HttpsConnector<HttpConnector>>, Error> {
  let secret = oauth2::parse_service_account_key(service_key)?;
  let auth_builder = oauth2::ServiceAccountAuthenticator::builder(secret);
  let authenticator = auth_builder.build().await?;

  Ok(authenticator)
}

pub struct AsyncSheetsHub {
  hub: Sheets<hyper_rustls::HttpsConnector<HttpConnector>>,
  spreadsheet_id: String,
}

impl AsyncSheetsHub {
  pub async fn new(service_key: &str, spreadsheet_id: &str) -> Result<AsyncSheetsHub, Error> {
    let hub = create_hub(service_key).await?;

    Ok(AsyncSheetsHub {
      hub,
      spreadsheet_id: spreadsheet_id.to_string(),
    })
  }

  pub async fn fetch_dashboard(&self) -> Result<Dashboard, Error> {
    // Fetch titles to identify actual sheet_id corresponding to
    // relevant dashboard data.
    debug!("[AsyncHub] Start fetching dashboard data...");
    let sheets = self
      .fetch_spreadsheet(false)
      .await?
      .sheets
      .ok_or(Error::InvalidFetchedData(EmptySheets))?;
    debug!("[AsyncHub] Fetched {:} sheet(s)", sheets.len());

    // Looking for sheet_id for derived title
    let derived_title = helpers::derive_title_name();
    let sheet_id =
      helpers::get_sheet_id_by_title(&sheets, &derived_title).ok_or(Error::InvalidFetchedData(NotFoundSheetId(derived_title.clone())))?;

    let mut tables: Vec<ScoreTable> = Vec::new();
    let mut request = RequestFactory::new(sheet_id).construct_score_table_request(true);

    trace!("[AsyncHub] Score table parsing loop has started ...");
    loop {
      match self.fetch_score_table(sheet_id, &request, false).await {
        Ok(score_table) => {
          info!(
            "[AsyncHub] New score table parsed for person with name '{}'",
            score_table.person().name()
          );
          tables.push(score_table)
        }
        Err(Error::InvalidFetchedData(EmptyPersonNameCell)) => {
          warn!("[AsyncHub] Empty person name cell was reached - finish parsing loop");
          break;
        }
        Err(err) => {
          error!("[AsyncHub] Error has occured while obtaining new score table {:#?}", err);
          return Err(err);
        }
      };
      request.next_table_request();
    }
    trace!(
      "[AsyncHub] Score table parsing loop has finished. Parsed data for {} persons",
      tables.len()
    );

    Ok(Dashboard::from(tables))
  }

  async fn fetch_score_table(&self, sheet_id: i32, request: &ScoreTableRequest, skip_parse_errors: bool) -> Result<ScoreTable, Error> {
    debug!("[AsyncHub] Start fetching a person table from sheet_id={}...", sheet_id);
    let sheets = self
      .fetch_spreadsheet_with_data_filter(request.build())
      .await?
      .sheets
      .ok_or(Error::InvalidFetchedData(EmptySheets))?;

    if sheets.len() != 1 {
      return Err(Error::InvalidFetchedData(InvalidVectorSize));
    }

    let grid_data_vec = if let [first_sheet, ..] = &sheets[..] {
      first_sheet.data.as_ref()
    } else {
      return Err(Error::InvalidFetchedData(EmptySheets));
    };
    let grid_data_vec: &Vec<GridData> = grid_data_vec.ok_or(Error::InvalidFetchedData(EmptyGridData))?;

    let row_data = if let [first_grid_data, ..] = &grid_data_vec[..] {
      first_grid_data.row_data.as_ref()
    } else {
      return Err(Error::InvalidFetchedData(EmptyGridData));
    };
    let row_data: &Vec<RowData> = row_data.ok_or(Error::InvalidFetchedData(EmptyRowData))?;

    let table: Vec<Result<_, Error>> = row_data
      .iter()
      .map(|d| d.values.as_ref().ok_or(Error::InvalidFetchedData(EmptyCellData)))
      .collect();

    let mut table_iter = table.into_iter();
    // Get the name of new person
    let name = match table_iter.next() {
      Some(cell_vec) => {
        if let [first_cell, ..] = &cell_vec?[..] {
          match &first_cell.formatted_value {
            Some(value) => value.trim().to_owned(),
            None => return Err(Error::InvalidFetchedData(EmptyPersonNameCell)),
          }
        } else {
          return Err(Error::InvalidFetchedData(EmptyCellData));
        }
      }
      None => return Err(Error::InvalidFetchedData(EmptyCellData)),
    };

    // Create person
    let person = Person::new(name);
    let mut records: Vec<ScoreTableRecord> = Vec::new();

    // Starts from the second row
    trace!("[AsyncHub] Collecting table records for newly created {:?}", person);
    for row in table_iter {
      let new_record = ScoreTableRecord::from_vec(row?).or_else(|err| {
        error!("[AsyncHub] Parse error (skipped ? {}): {}", skip_parse_errors, err);
        if skip_parse_errors {
          Ok(ScoreTableRecord::default())
        } else {
          Err(err)
        }
      })?;
      trace!("[AsyncHub] New score table record parsed {:?}", new_record);
      records.push(new_record);
    }

    debug!(
      "[AsyncHub] Finish fetching a person({:?}) table with size={} from sheet_id={}",
      person,
      records.len(),
      sheet_id
    );
    Ok(ScoreTable::new(person, records))
  }

  async fn fetch_spreadsheet(&self, include_grid_data: bool) -> Result<Spreadsheet, Error> {
    debug!(
      "[AsyncHub] Start fetching spreadsheet (include_grid_data={:})...",
      include_grid_data
    );
    let request = self
      .hub
      .spreadsheets()
      .get(&self.spreadsheet_id)
      .include_grid_data(include_grid_data);
    let (_body, spreadsheet) = request.doit().await?;
    debug!("[AsyncHub] Finish fetching spreadsheet");
    Ok(spreadsheet)
  }

  async fn fetch_spreadsheet_with_data_filter(&self, filter: GetSpreadsheetByDataFilterRequest) -> Result<Spreadsheet, Error> {
    debug!("[AsyncHub] Start fetching spreadsheet with filter data request...");
    let request = self.hub.spreadsheets().get_by_data_filter(filter, &self.spreadsheet_id);
    let (_body, spreadsheet) = request.doit().await?;
    debug!("[AsyncHub] Finish fetching spreadsheet");
    Ok(spreadsheet)
  }
}
