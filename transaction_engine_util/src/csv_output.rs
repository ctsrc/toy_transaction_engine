//! Helper for outputting CSV data.

use serde::Serialize;

/// Helper struct for serialization of account data to the CSV format
/// specified in the spec.
#[derive(Serialize, Debug)]
pub struct AccountOutputCSVRecord {
  pub client: u16,
  pub available: String,
  pub held: String,
  pub total: String,
  pub locked: bool,
}
