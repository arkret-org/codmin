use dioxus::prelude::*;

use crate::api::realms;
use crate::components::ui::auto_refresh::{self, AutoRefreshPicker, RefreshInterval};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::icons::Icon;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::pagination::CursorPagination;
use crate::components::ui::table::*;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::router::Route;
use crate::types::realms::AdminRealmItemExt;
use crate::utils::fmt::csv::{build_csv, export_to_csv};
use crate::utils::i18n::t;

const PAGE_SIZE: u64 = 20;
const AUTOREFRESH_STORAGE_KEY: &str = "codmin.realms.autorefresh";

#[component]
pub fn RealmList() -> Element {
    let mut cursor_stack = use_signal(|| vec![None::<String>]);
    let mut autorefresh = use_signal(|| auto_refresh::load(AUTOREFRESH_STORAGE_KEY));

    let cursor_snapshot = cursor_stack.read().last().cloned().unwrap_or(None);

    let mut data = use_resource(move || {
        let cursor = cursor_snapshot.clone();
        async move { realms::list_realms(cursor.as_deref(), PAGE_SIZE).await }
    });

    let mut interval_handle = use_signal::<Option<gloo_timers::callback::Interval>>(|| None);
    use_effect(move || {
        let choice = *autorefresh.read();
        interval_handle.set(None);
        if let Some(ms) = choice.millis() {
            let mut data = data;
            let handle = gloo_timers::callback::Interval::new(ms, move || {
                data.restart();
            });
            interval_handle.set(Some(handle));
        }
    });

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("nav.realms"),
                description: t("realms.description"),
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    onclick: move |_| {
                        if let Some(Ok(resp)) = data.read().as_ref() {
                            let rows: Vec<Vec<String>> = resp.data.iter().map(|s| vec![
                                s.id.clone(),
                                s.title.clone(),
                                s.type_label().to_owned(),
                                s.member_count.to_string(),
                                if s.is_encrypted { "true".into() } else { "false".into() },
                                if s.is_blocked { "blocked".into() } else { "active".into() },
                                s.created_at_display().unwrap_or_default(),
                            ]).collect();
                            let csv = build_csv(
                                &["id", "title", "realm_class", "member_count", "encrypted", "status", "created_at"],
                                &rows,
                            );
                            export_to_csv("realms.csv", &csv);
                            show_toast(&t("realms.toast_csv_downloaded"), ToastVariant::Success);
                        }
                    },
                    {t("common.export_csv")}
                }
            }

            div { class: "flex items-center gap-4 flex-wrap justify-end",
                AutoRefreshPicker {
                    storage_key: AUTOREFRESH_STORAGE_KEY.to_string(),
                    value: *autorefresh.read(),
                    onchange: move |v: RefreshInterval| autorefresh.set(v),
                }
            }

            match &*data.read() {
                Some(Ok(resp)) => {
                    let next_cursor = resp.next_cursor.clone();
                    let stack_depth = cursor_stack.read().len();
                    rsx! {
                        div { class: "rounded-lg border glass-panel overflow-hidden",
                            Table {
                                TableHeader {
                                    TableRow {
                                        TableHead { {t("realms.id")} }
                                        TableHead { {t("realms.name")} }
                                        TableHead { {t("realms.type")} }
                                        TableHead { {t("realms.members")} }
                                        TableHead { {t("realms.encrypted")} }
                                        TableHead { {t("realms.status")} }
                                        TableHead { {t("realms.created_at")} }
                                    }
                                }
                                TableBody {
                                    for realm in resp.data.iter() {
                                        {
                                            let rid = realm.id.clone();
                                            rsx! {
                                                TableRow {
                                                    key: "{realm.id}",
                                                    TableCell { class: "font-mono text-xs",
                                                        Link {
                                                            to: Route::RealmShow { realm_id: rid.clone() },
                                                            class: "hover:underline",
                                                            "{realm.id}"
                                                        }
                                                    }
                                                    TableCell { {realm.title.clone()} }
                                                    TableCell { {realm.type_label()} }
                                                    TableCell { "{realm.member_count}" }
                                                    TableCell {
                                                        if realm.is_encrypted {
                                                            Icon { name: "lock".to_string(), class: "h-4 w-4 text-green-500".to_string() }
                                                        }
                                                    }
                                                    TableCell {
                                                        if realm.is_blocked {
                                                            {t("realms.blocked")}
                                                        } else {
                                                            {t("realms.active")}
                                                        }
                                                    }
                                                    TableCell { {realm.created_at_display().unwrap_or_else(|| "-".to_string())} }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        CursorPagination {
                            depth: stack_depth,
                            has_next: next_cursor.is_some(),
                            on_prev: move |_| {
                                let mut new_stack = cursor_stack.read().clone();
                                if new_stack.len() > 1 {
                                    new_stack.pop();
                                    cursor_stack.set(new_stack);
                                }
                            },
                            on_next: move |_| {
                                if let Some(c) = next_cursor.clone() {
                                    let mut new_stack = cursor_stack.read().clone();
                                    new_stack.push(Some(c));
                                    cursor_stack.set(new_stack);
                                }
                            },
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        errcode: e.body.as_ref().map(|b| b.errcode.clone()),
                        request_id: e.request_id.clone(),
                        retry_after_ms: e.retry_after_ms,
                        on_retry: move |_| data.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}
