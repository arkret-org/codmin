use dioxus::prelude::*;

use crate::api::auth;
use crate::components::ui::button::Button;
use crate::components::ui::loading::Spinner;
use crate::utils::i18n::t;

#[component]
pub fn LoginPage(logout_warning: Option<String>) -> Element {
    let mut loading = use_signal(|| false);
    let mut ready = use_signal(|| false);
    let mut config_error = use_signal::<Option<String>>(|| None);
    let mut login_error = use_signal::<Option<String>>(|| None);

    use_effect(move || {
        spawn(async move {
            match crate::utils::net::config::load_runtime_config().await {
                Ok(cfg) => {
                    crate::utils::storage::set_item("oauth_client_id", &cfg.oauth_client_id);
                    crate::utils::storage::set_item("coauth_public_url", &cfg.coauth_public_url);
                    // Propagate the optional telemetry endpoint
                    // into localStorage so `utils::net::telemetry` can pick
                    // it up the first time a report fires.
                    crate::utils::net::telemetry::set_endpoint(&cfg.telemetry_endpoint);
                    config_error.set(None);
                    ready.set(true);
                }
                Err(err) => {
                    config_error.set(Some(err.to_string()));
                    ready.set(true);
                }
            }
        });
    });

    let handle_login = move |_evt: MouseEvent| {
        loading.set(true);
        login_error.set(None);
        spawn(async move {
            if let Err(err) = auth::start_oauth_login().await {
                login_error.set(Some(err.to_string()));
            }
            loading.set(false);
        });
    };

    let is_ready = *ready.read();
    let is_loading = *loading.read();
    let error_message = config_error.read().clone();
    let oauth_error = login_error.read().clone();
    let logout_warning = logout_warning.as_deref().and_then(logout_warning_message);

    if let Some(message) = error_message {
        return rsx! { ConfigErrorPanel { message } };
    }

    rsx! {
        div { class: "flex min-h-screen items-center justify-center bg-background p-4", role: "main",
            div { class: "w-full max-w-md space-y-6",
                div { class: "text-center space-y-2",
                    div { class: "mx-auto h-16 w-16 rounded-full bg-primary/10 flex items-center justify-center",
                        crate::components::ui::icons::Icon {
                            name: "shield".to_string(),
                            class: "h-8 w-8 text-primary".to_string(),
                        }
                    }
                    h1 { class: "text-2xl font-bold tracking-tight", "Arkret Admin" }
                    p { class: "text-muted-foreground", {t("auth.sign_in_subtitle")} }
                }

                div { class: "rounded-lg border glass-panel p-6 shadow-sm space-y-4",
                    if let Some(message) = logout_warning {
                        div {
                            class: "rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-300",
                            role: "alert",
                            "{message}"
                        }
                    }
                    if let Some(message) = oauth_error {
                        div { class: "rounded-md bg-destructive/10 p-3 text-sm text-destructive",
                            "{message}"
                        }
                    }
                    p { class: "text-sm text-center text-muted-foreground",
                        {t("auth.oauth_hint")}
                    }

                    Button {
                        class: "w-full".to_string(),
                        disabled: !is_ready || is_loading,
                        onclick: handle_login,
                        if is_loading {
                            Spinner { class: "mr-2".to_string() }
                        }
                        {t("auth.sign_in")}
                    }
                }

                p { class: "text-center text-xs text-muted-foreground",
                    {t("auth.footer")}
                }
            }
        }
    }
}

fn logout_warning_message(code: &str) -> Option<String> {
    let mut stages = Vec::new();
    for item in code.split(',') {
        match item {
            "principal_logout" => stages.push("Station session cleanup"),
            "oauth_revoke" => stages.push("OAuth token revocation"),
            _ => {}
        }
    }
    if stages.is_empty() {
        return None;
    }
    Some(format!(
        "This browser session was closed, but {} was not confirmed. Sign in again and retry sign-out if remote cleanup is still required.",
        stages.join(" and ")
    ))
}

/// Hard error surface shown when `/config.json` is unreachable or missing
/// `coauth_public_url`. Sign-in is impossible without this URL — the
/// OAuth redirect would point to a relative path and fail — so we
/// short-circuit with a deployment-ops-facing message instead of a dead
/// sign-in button.
#[component]
fn ConfigErrorPanel(message: String) -> Element {
    rsx! {
        div { class: "flex min-h-screen items-center justify-center bg-background p-4", role: "main",
            div { class: "w-full max-w-lg space-y-6",
                div { class: "text-center space-y-2",
                    div { class: "mx-auto h-16 w-16 rounded-full bg-destructive/10 flex items-center justify-center",
                        crate::components::ui::icons::Icon {
                            name: "alert-triangle".to_string(),
                            class: "h-8 w-8 text-destructive".to_string(),
                        }
                    }
                    h1 { class: "text-2xl font-bold tracking-tight",
                        {t("config.error.title")}
                    }
                    p { class: "text-muted-foreground",
                        {t("config.error.subtitle")}
                    }
                }

                div { class: "rounded-lg border border-destructive/40 bg-destructive/5 p-4 text-sm",
                    p { class: "font-mono whitespace-pre-wrap break-all", "{message}" }
                }

                div { class: "rounded-lg border glass-panel p-4 text-sm space-y-2",
                    p { class: "font-medium", {t("config.error.hint_title")} }
                    ul { class: "list-disc list-inside text-muted-foreground space-y-1",
                        li { {t("config.error.hint_serve_path")} }
                        li { {t("config.error.hint_required_fields")} }
                        li { {t("config.error.hint_check_proxy")} }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::logout_warning_message;

    #[test]
    fn logout_warning_codes_are_fixed_and_non_sensitive() {
        let message = logout_warning_message("principal_logout,oauth_revoke").unwrap();
        assert!(message.contains("Station session cleanup"));
        assert!(message.contains("OAuth token revocation"));
        assert!(logout_warning_message("server body here").is_none());
    }
}
