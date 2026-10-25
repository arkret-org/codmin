use glory::prelude::*;

use crate::components::ui::{self, Danger, Field, Resource, View};
use crate::pages::{presentation as p, station};
use crate::types::policy::{AdminPolicy, AdminPolicyListOutcome, CreatePolicyRequest};
use crate::utils::i18n::t;
use crate::utils::net::error::HttpError;

pub fn page(scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let filters = scope.cage(crate::api::policy::PolicyListFilter::default());
    let data = ui::resource(scope, move || {
        let filter = filters.get();
        async move { crate::api::policy::list_policies(filter).await }
    });
    let filter_reload = data.clone();
    let edit_reload = data.clone();
    let body_theme = theme.clone();
    col().class("station-page").fill((
        ui::heading(t("policy.title")),
        p::description("policy.subtitle"),
        ui::action_form(
            scope,
            theme.clone(),
            "Apply policy filters",
            vec![
                Field::new("Scope", ""),
                Field::new("Subject reference", ""),
                Field::new("Include inactive (true / false)", "false"),
            ],
            None,
            move |values| async move {
                let include_inactive = values[2]
                    .trim()
                    .parse::<bool>()
                    .map_err(|_| HttpError::message("Include inactive must be true or false"))?;
                filters.revise(|value| {
                    *value = crate::api::policy::PolicyListFilter {
                        scope: values[0].clone(),
                        subject_ref: values[1].clone(),
                        include_inactive,
                    }
                });
                Ok(())
            },
            move || filter_reload.reload(),
        ),
        ui::compact_action(
            theme.clone(),
            t("common.create"),
            editor(scope, theme.clone(), None, data.clone()),
        ),
        ui::resource_view(scope, theme, data, move |scope, response| {
            let rows = response
                .data
                .into_iter()
                .map(|policy| {
                    let guardrails = p::stack(vec![
                        p::badge(
                            body_theme.clone(),
                            format!(
                                "{} {}",
                                policy.guardrails.obligations.len(),
                                t("policy.obligations")
                            ),
                            "neutral",
                        ),
                        if policy.safety.read_only {
                            p::badge(body_theme.clone(), t("policy.read_only"), "neutral")
                        } else {
                            p::plain(t("policy.guardrails_none"))
                        },
                    ]);
                    let mut actions = Vec::<View>::new();
                    if policy.safety.read_only {
                        actions.push(ui::compact_action(
                            body_theme.clone(),
                            t("common.view"),
                            p::stack(vec![
                                p::plain(
                                    policy
                                        .safety
                                        .read_only_reason
                                        .clone()
                                        .unwrap_or_else(|| t("policy.read_only")),
                                ),
                                p::plain(t("policy.typed_evidence_unavailable")),
                            ]),
                        ));
                    } else {
                        actions.push(ui::compact_action(
                            body_theme.clone(),
                            t("common.edit"),
                            editor(
                                scope,
                                body_theme.clone(),
                                Some(policy.clone()),
                                edit_reload.clone(),
                            ),
                        ));
                        let id = policy.id.clone();
                        let reload = edit_reload.clone();
                        actions.push(ui::compact_action(
                            body_theme.clone(),
                            t("common.delete"),
                            ui::action_form(
                                scope,
                                body_theme.clone(),
                                t("common.delete"),
                                vec![],
                                Some(Danger::new(station::suffix(&id)).without_reason()),
                                move |_| {
                                    let id = id.clone();
                                    async move { crate::api::policy::delete_policy(&id).await }
                                },
                                move || reload.reload(),
                            ),
                        ));
                    }
                    vec![
                        p::plain(&policy.id),
                        p::plain(&policy.name),
                        p::plain(p::optional(policy.policy_kind.as_ref())),
                        p::plain(p::optional(policy.scope.as_ref())),
                        p::mono(p::optional(policy.subject_ref.as_ref())),
                        p::badge(
                            body_theme.clone(),
                            p::yes(policy.is_enabled),
                            if policy.is_enabled {
                                "success"
                            } else {
                                "neutral"
                            },
                        ),
                        p::plain(policy.priority),
                        guardrails,
                        p::plain(p::optional(policy.updated_at.as_ref())),
                        p::stack(actions),
                    ]
                })
                .collect();
            p::table(
                body_theme.clone(),
                &[
                    "policy.id",
                    "policy.name",
                    "policy.policy_kind",
                    "policy.scope",
                    "Subject",
                    "policy.enabled",
                    "policy.priority",
                    "policy.guardrails",
                    "policy.updated_at",
                    "common.actions",
                ],
                rows,
                "policy.no_policies",
            )
        }),
    ))
}
fn editor(
    scope: &Scope,
    theme: sapid::Theme,
    existing: Option<AdminPolicy>,
    reload: Resource<AdminPolicyListOutcome>,
) -> View {
    let initial = existing.clone();
    let field = |get: fn(&AdminPolicy) -> String, default: &str| {
        initial
            .as_ref()
            .map(get)
            .unwrap_or_else(|| default.to_owned())
    };
    let fields = vec![
        Field::new("Name", field(|p| p.name.clone(), "")),
        Field::new(
            "Policy kind",
            field(|p| p.policy_kind.clone().unwrap_or_default(), ""),
        ),
        Field::new(
            "Description",
            field(|p| p.description.clone().unwrap_or_default(), ""),
        ),
        Field::new("Scope", field(|p| p.scope.clone().unwrap_or_default(), "*")),
        Field::new(
            "Subject reference",
            field(|p| p.subject_ref.clone().unwrap_or_default(), "*"),
        ),
        Field::new("Priority", field(|p| p.priority.to_string(), "0")),
        Field::new(
            "Enabled (true / false)",
            field(|p| p.is_enabled.to_string(), "true"),
        ),
    ];
    let title = if existing.is_some() {
        t("common.save")
    } else {
        t("common.create")
    };
    ui::action_form(
        scope,
        theme,
        title,
        fields,
        None,
        move |values| {
            let existing = existing.clone();
            async move {
                if existing
                    .as_ref()
                    .is_some_and(|policy| policy.safety.read_only)
                {
                    return Err(HttpError::message("This policy is read only"));
                }
                let optional = |index: usize| {
                    let value = values[index].trim();
                    (!value.is_empty()).then(|| value.to_owned())
                };
                let request = CreatePolicyRequest {
                    name: values[0].trim().to_owned(),
                    policy_kind: optional(1),
                    description: optional(2),
                    scope: optional(3),
                    subject_ref: optional(4),
                    rules: existing.as_ref().and_then(|p| p.rules.clone()),
                    is_enabled: values[6]
                        .trim()
                        .parse()
                        .map_err(|_| HttpError::message("Enabled must be true or false"))?,
                    priority: values[5]
                        .trim()
                        .parse()
                        .map_err(|_| HttpError::message("Priority must be a signed integer"))?,
                };
                if request.name.is_empty() {
                    return Err(HttpError::message("Policy name is required"));
                }
                if crate::api::policy::request_targets_pin_policy(&request) {
                    return Err(HttpError::message(t("policy.pin_edit_unavailable")));
                }
                match existing {
                    Some(policy) => crate::api::policy::update_policy(&policy.id, &request)
                        .await
                        .map(|_| ()),
                    None => crate::api::policy::create_policy(&request)
                        .await
                        .map(|_| ()),
                }
            }
        },
        move || reload.reload(),
    )
}
