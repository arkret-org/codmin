use glory::prelude::*;

use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    col().class("station-page").fill((
  text().content("Recovery policies are read only: publishing requires the principal's signed auth_data transcript."),
  station::browse(scope,theme.clone(),t("key_backup.title"),vec![("Backup series ID",""),("Backup kind","")],|q|async move {
   let series=(!q.field(0).trim().is_empty()).then(||q.field(0).trim());
   let kind=(!q.field(1).trim().is_empty()).then(||q.field(1).trim());
   Document::default().section("Backup series, heads and chain rejection counters",&crate::api::key_backup::list_backups(series,kind).await?)
  }),
  station::browse(scope,theme,"Recovery policies".into(),vec![("Principal DID Core ID","")],|q|async move {
   let principal=if q.field(0).trim().is_empty(){None}else{Some(arkret_identifiers::DidCoreId::new(q.field(0).trim()).map_err(|e|crate::utils::net::error::HttpError::message(format!("Invalid principal ID: {e}")))?)};
   Document::default().section("Recovery policy history",&crate::api::key_backup::list_recovery_policies(principal.as_ref()).await?)
  }),
 ))
}
