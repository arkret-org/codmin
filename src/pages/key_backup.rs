use arkret_models_crypto::recovery_policy::RecoveryMethod;
use glory::prelude::*;

use crate::pages::presentation as p;
use crate::pages::station::{self, Document};
use crate::utils::i18n::t;
pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    col().class("station-page").fill((
        station::browse_with_description(
            scope,
            theme.clone(),
            t("key_backup.title"),
            t("key_backup.subtitle"),
            vec![("series_id", ""), ("backup_kind", "")],
            |q| async move {
                let series = (!q.field(0).trim().is_empty()).then(|| q.field(0).trim());
                let kind = (!q.field(1).trim().is_empty()).then(|| q.field(1).trim());
                let response = crate::api::key_backup::list_backups(series, kind).await?;
                let rows = response.backups.clone();
                Ok(Document::default()
                    .section("Backups", &response.backups)?
                    .view(move |_, theme| {
                        p::table(
                            theme,
                            &[
                                "backup_id",
                                "actor_id",
                                "backup_kind",
                                "backup_version",
                                "created_at",
                                "ciphertext_digest",
                            ],
                            rows.iter()
                                .map(|row| {
                                    vec![
                                        p::mono(&row.backup_id),
                                        p::mono(&row.actor_id),
                                        p::plain(format!("{:?}", row.backup_kind)),
                                        p::plain(&row.backup_version),
                                        p::plain(row.created_at.to_rfc3339()),
                                        p::mono(&row.ciphertext_digest),
                                    ]
                                })
                                .collect(),
                            "key_backup.no_series",
                        )
                    }))
            },
        ),
        station::browse(
            scope,
            theme,
            t("key_backup.recovery_policies"),
            vec![("principal_id", "")],
            |q| async move {
                let principal = if q.field(0).trim().is_empty() {
                    None
                } else {
                    Some(
                        arkret_identifiers::DidCoreId::new(q.field(0).trim()).map_err(|e| {
                            crate::utils::net::error::HttpError::message(e.to_string())
                        })?,
                    )
                };
                let rows =
                    crate::api::key_backup::list_recovery_policies(principal.as_ref()).await?;
                Ok(Document::default()
                    .section("Recovery policies", &rows)?
                    .view(move |_, theme| {
                        p::table(
                            theme.clone(),
                            &[
                                "policy_id",
                                "version",
                                "trust_domain",
                                "methods",
                                "issued_at",
                                "expires_at",
                            ],
                            rows.iter()
                                .map(|row| {
                                    vec![
                                        p::mono(row.policy_id.as_str()),
                                        p::badge(
                                            theme.clone(),
                                            format!("v{}", row.version),
                                            "neutral",
                                        ),
                                        p::mono(row.trust_domain.as_str()),
                                        p::plain(if row.methods.is_empty() {
                                            "(revoked)".into()
                                        } else {
                                            row.methods
                                                .iter()
                                                .map(RecoveryMethod::kind_str)
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        }),
                                        p::plain(row.issued_at.to_rfc3339()),
                                        p::plain(p::optional(
                                            row.expires_at
                                                .as_ref()
                                                .map(chrono::DateTime::to_rfc3339),
                                        )),
                                    ]
                                })
                                .collect(),
                            "key_backup.no_policies",
                        )
                    }))
            },
        ),
    ))
}
