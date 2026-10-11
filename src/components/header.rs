use dioxus::prelude::*;

use crate::components::theme::{get_resolved_theme, set_theme};
use crate::components::ui::icons::Icon;
use crate::router::Route;
use crate::utils::fmt::date::format_timestamp;
use crate::utils::i18n::{Language, current_language, set_language, t};
use crate::utils::net::session;

#[component]
pub fn AppHeader(collapsed: Signal<bool>, mobile_sidebar_open: Signal<bool>) -> Element {
    let mut is_collapsed = collapsed;
    let mut is_mobile_sidebar_open = mobile_sidebar_open;
    let mut dark_mode = use_signal(|| get_resolved_theme() == "dark");
    let mut logout_in_progress = use_signal(|| false);
    let mut logout_error = use_signal(|| None::<String>);
    let nav = use_navigator();

    rsx! {
        header { class: "flex h-14 items-center border-b px-4 lg:px-6",
            button {
                class: "inline-flex h-9 w-9 items-center justify-center rounded-lg text-sm font-medium hover:bg-accent hover:text-accent-foreground touch-target",
                onclick: move |_| {
                    let is_mobile_view = web_sys::window()
                        .and_then(|window| window.inner_width().ok())
                        .and_then(|width| width.as_f64())
                        .map(|width| width < 768.0)
                        .unwrap_or(false);

                    if is_mobile_view {
                        let current = *is_mobile_sidebar_open.read();
                        is_mobile_sidebar_open.set(!current);
                    } else {
                        let current = *is_collapsed.read();
                        is_collapsed.set(!current);
                    }
                },
                svg {
                    class: "h-5 w-5",
                    xmlns: "http://www.w3.org/2000/svg",
                    width: "24", height: "24",
                    view_box: "0 0 24 24",
                    fill: "none", stroke: "currentColor",
                    stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                    line { x1: "4", x2: "20", y1: "12", y2: "12" }
                    line { x1: "4", x2: "20", y1: "6", y2: "6" }
                    line { x1: "4", x2: "20", y1: "18", y2: "18" }
                }
            }
            div { class: "flex-1" }
            // Language selector + Theme toggle + Notification bell + User info
            div { class: "app-header-controls",
                // Language selector
                select {
                    class: "app-header-language h-9 rounded-lg border bg-background px-3 text-xs text-foreground touch-target",
                    value: "{current_language().code()}",
                    onchange: move |evt: FormEvent| {
                        if let Some(lang) = Language::from_code(&evt.value()) {
                            set_language(lang);
                        }
                    },
                    for lang in Language::all().iter() {
                        option {
                            value: "{lang.code()}",
                            selected: current_language() == *lang,
                            "{lang.label()}"
                        }
                    }
                }

                button {
                    class: "inline-flex h-9 w-9 items-center justify-center rounded-lg hover:bg-accent hover:text-accent-foreground touch-target",
                    title: if *dark_mode.read() { t("header.switch_light") } else { t("header.switch_dark") },
                    onclick: move |_| {
                        let new_dark = !*dark_mode.read();
                        dark_mode.set(new_dark);
                        set_theme(if new_dark { "dark" } else { "light" });
                    },
                    Icon {
                        name: if *dark_mode.read() { "sun".to_string() } else { "moon".to_string() },
                        class: "h-4 w-4".to_string(),
                    }
                }

                // The notification center was removed: it had no producer
                // (nothing ever pushed into the global signal), so the bell,
                // unread badge, and dropdown were a permanently-empty, dead
                // UI. Reintroduce alongside the coland notification feed when
                // that lands (recoverable from git history).

                {
                    let user = session::current_user();
                    let display_name = user.display_name.clone();
                    let user_id = user.id.clone();
                    let avatar_url = user.avatar_url.clone();
                    let label = user_id
                        .clone()
                        .or_else(|| display_name.clone())
                        .unwrap_or_default();
                    let full = match (display_name.as_deref(), user_id.as_deref()) {
                        (Some(name), Some(id)) => format!("{name} ({id})"),
                        (Some(name), None) => name.to_string(),
                        (None, Some(id)) => id.to_string(),
                        _ => String::new(),
                    };
                    let initial_source = display_name
                        .as_deref()
                        .or(user_id.as_deref())
                        .unwrap_or("")
                        .trim_start_matches('@');
                    let initial = initial_source
                        .chars()
                        .next()
                        .map(|c| c.to_uppercase().to_string())
                        .unwrap_or_default();
                    let token_expiry = crate::api::auth::token_expiry_ms()
                        .map(format_timestamp)
                        .unwrap_or_else(|| "session expiry unknown".to_string());
                    rsx! {
                        div { class: "app-header-session flex-wrap justify-end",
                            div { class: "app-header-user flex items-center gap-2", title: "{full}",
                                div { class: "flex h-8 w-8 items-center justify-center rounded-full bg-primary/10 text-xs font-semibold text-primary overflow-hidden",
                                    if let Some(url) = avatar_url.as_deref() {
                                        if !url.is_empty() {
                                            img { src: "{url}", alt: "{full}", class: "h-full w-full object-cover" }
                                        } else {
                                            span { "{initial}" }
                                        }
                                    } else {
                                        span { "{initial}" }
                                    }
                                }
                                div { class: "min-w-0",
                                    div { class: "truncate text-xs font-mono text-muted-foreground", "{label}" }
                                    div { class: "truncate text-[10px] text-muted-foreground/70", "token: {token_expiry}" }
                                }
                            }
                            button {
                                class: "app-header-logout inline-flex h-9 items-center justify-center rounded-lg border bg-background px-3 text-xs font-medium text-foreground hover:bg-accent hover:text-accent-foreground touch-target",
                                disabled: *logout_in_progress.read(),
                                onclick: move |_| {
                                    let nav = nav;
                                    logout_error.set(None);
                                    logout_in_progress.set(true);
                                    spawn(async move {
                                        let report = crate::api::auth::logout().await;
                                        if report.local_cookie_cleared() {
                                            let logout_warning = report.login_warning_code();
                                            if !report.fully_confirmed() {
                                                log::warn!(
                                                    "local logout succeeded with incomplete upstream cleanup: {report:?}"
                                                );
                                            }
                                            logout_in_progress.set(false);
                                            nav.replace(Route::LoginPage { logout_warning });
                                        } else {
                                            logout_error.set(Some(report.retry_message()));
                                            logout_in_progress.set(false);
                                        }
                                    });
                                },
                                if *logout_in_progress.read() {
                                    "Signing out..."
                                } else if logout_error.read().is_some() {
                                    "Retry sign-out"
                                } else {
                                    {t("nav.logout")}
                                }
                            }
                            if let Some(message) = logout_error.read().as_ref() {
                                p {
                                    class: "basis-full max-w-md text-right text-xs text-destructive",
                                    role: "alert",
                                    "{message}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
