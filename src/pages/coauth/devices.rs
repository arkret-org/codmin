use glory::prelude::*;

use super::common;
use crate::api::coauth_devices;
use crate::components::ui::{self, Danger};
use crate::types::coauth_devices::{
    CoauthDeviceLabel, CoauthDeviceMfaState, CoauthDeviceRiskLevel, render_optional_timestamp,
};
use crate::utils::i18n::t;
const HEADERS: &[&str] = &[
    "coauth_devices.device_id",
    "coauth_devices.display_name",
    "coauth_devices.risk_level",
    "coauth_devices.mfa_state",
    "coauth_devices.registered_at",
    "coauth_devices.revoked_at",
    "common.actions",
];
pub fn page(scope: &Scope, theme: sapid::Theme, account_id: String) -> impl Widget + use<> {
    let title = format!("{} · {}", t("coauth_devices.title"), account_id);
    let mutations = ui::MutationController::new(scope);
    let account = account_id.clone();
    let data = ui::resource(scope, move || {
        let account = account.clone();
        async move { coauth_devices::list_account_devices(&account).await }
    });
    let changed = data.clone();
    let row_theme = theme.clone();
    let body = common::resource_body(theme.clone(), data.clone(), move |scope, devices| {
        if devices.is_empty() {
            return Box::new(
                view().class("table-empty").fill((
                    text()
                        .kind(TextKind::Heading(2))
                        .content(t("coauth_devices.empty_title")),
                    text()
                        .class("text-muted")
                        .content(t("coauth_devices.empty_subtitle")),
                )),
            );
        }
        let rows = devices
            .into_iter()
            .map(|device| {
                let action = if device.is_revocable() {
                    let id = device.id.clone();
                    let account = account_id.clone();
                    let changed = changed.clone();
                    let form = ui::action_form_controlled(
                        scope,
                        &mutations,
                        device.id.clone(),
                        row_theme.clone(),
                        t("coauth_devices.revoke"),
                        vec![],
                        Some(Danger::new(common::suffix(&id))),
                        move |values| {
                            let id = id.clone();
                            let account = account.clone();
                            async move {
                                coauth_devices::revoke_account_device(&account, &id, &values[0])
                                    .await
                            }
                        },
                        move || changed.reload(),
                    );
                    ui::compact_action(
                        row_theme.clone(),
                        t("coauth_devices.revoke"),
                        common::stack(
                            row_theme.clone(),
                            vec![common::muted(t("coauth_devices.revoke_confirm_body")), form],
                        ),
                    )
                } else {
                    common::muted("—")
                };
                let risk_kind = match &device.risk_level {
                    CoauthDeviceRiskLevel::Low => "success",
                    CoauthDeviceRiskLevel::Medium => "warning",
                    CoauthDeviceRiskLevel::High => "danger",
                    CoauthDeviceRiskLevel::Unknown => "neutral",
                };
                let mfa_kind = match &device.mfa_state {
                    CoauthDeviceMfaState::Verified => "success",
                    CoauthDeviceMfaState::Required => "danger",
                    CoauthDeviceMfaState::Unknown => "neutral",
                };
                vec![
                    common::mono(device.id),
                    common::cell(common::optional(device.display_name)),
                    common::badge(row_theme.clone(), device.risk_level.label(), risk_kind),
                    common::badge(row_theme.clone(), device.mfa_state.label(), mfa_kind),
                    common::muted(render_optional_timestamp(device.registered_at.as_ref())),
                    common::muted(render_optional_timestamp(device.revoked_at.as_ref())),
                    action,
                ]
            })
            .collect();
        common::table(
            row_theme.clone(),
            HEADERS,
            rows,
            "coauth_devices.empty_title",
        )
    });
    common::page_heading(
        title,
        t("coauth_devices.subtitle"),
        vec![common::refresh(theme, data)],
        vec![body],
    )
}
