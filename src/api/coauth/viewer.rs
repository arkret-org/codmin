//! coauth admin viewer (current operator identity).

use arkret_models_collaboration::account_operations::AccountView;

use crate::api::client::{NO_BODY, api_client};
use crate::utils::net::error::HttpError;

const VIEWER_PATH: &str = "/_coland/admin/viewer";

pub async fn get_viewer() -> Result<AccountView, HttpError> {
    api_client(VIEWER_PATH, "GET", NO_BODY).await
}
