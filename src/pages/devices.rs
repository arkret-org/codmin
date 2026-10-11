use dioxus::prelude::*;

use crate::api::devices;
use crate::components::ui::auto_refresh::{self, AutoRefreshPicker, RefreshInterval};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::input::SearchInput;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::pagination::CursorPagination;
use crate::components::ui::table::*;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::types::AdminDeviceExt;
use crate::utils::fmt::csv::{build_csv, export_to_csv};
use crate::utils::i18n::t;

const PAGE_SIZE: u64 = 25;
const AUTOREFRESH_STORAGE_KEY: &str = "codmin.devices.autorefresh";

#[component]
pub fn DeviceList() -> Element {
    let mut search = use_signal(String::new);
    let mut cursor_stack = use_signal(|| vec![None::<String>]);
    let mut autorefresh = use_signal(|| auto_refresh::load(AUTOREFRESH_STORAGE_KEY));

    let cursor_snapshot = cursor_stack.read().last().cloned().unwrap_or(None);
    let search_val = search.read().clone();

    let mut data = use_resource(move || {
        let s = search_val.clone();
        let cursor = cursor_snapshot.clone();
        async move { devices::list_devices(cursor.as_deref(), PAGE_SIZE, &s).await }
    });

    // Auto-refresh timer — re-installs whenever the chosen interval
    // changes. The previous handle is stored in a signal and
    // explicitly dropped (which cancels the underlying JS interval)
    // before a new one is created, so changing cadence doesn't leak
    // timers.
    let mut interval_handle = use_signal::<Option<gloo_timers::callback::Interval>>(|| None);
    use_effect(move || {
        // Read the signal INSIDE the effect closure so Dioxus tracks
        // the dependency and re-runs us whenever the picker changes.
        let choice = *autorefresh.read();
        // Drop any previous interval first — explicit so the timer
        // actually stops firing rather than just losing our handle.
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
                title: t("devices.title"),
                description: t("devices.subtitle"),
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    onclick: move |_| {
                            if let Some(Ok(resp)) = data.read().as_ref() {
                                let rows: Vec<Vec<String>> = resp.devices.iter()
                                    .map(|d| vec![
                                d.id.clone(),
                                d.actor_id.clone().unwrap_or_default(),
                                d.display_name.clone().unwrap_or_default(),
                                d.verification_label().unwrap_or_default().to_string(),
                                d.created_at_display().unwrap_or_default(),
                                d.updated_at_display().unwrap_or_default(),
                                d.revoked_at_display().unwrap_or_default(),
                                    ]).collect();
                                let csv = build_csv(
                                    &["id", "actor_id", "display_name", "verification_state", "created_at", "updated_at", "revoked_at"],
                                    &rows,
                                );
                                export_to_csv("devices.csv", &csv);
                                show_toast(&t("devices.toast_csv_downloaded"), ToastVariant::Success);
                            }
                    },
                    {t("common.export_csv")}
                }
            }

            div { class: "flex items-center gap-4 flex-wrap",
                div { class: "flex-1 min-w-[240px]",
                    SearchInput {
                        placeholder: t("devices.search"),
                        value: search.read().clone(),
                        oninput: move |evt: FormEvent| {
                            search.set(evt.value());
                            cursor_stack.set(vec![None::<String>]);
                        },
                    }
                }
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
                        div { class: "rounded-md border",
                            Table {
                                TableHeader {
                                    TableRow {
                                        TableHead { {t("devices.id")} }
                                        TableHead { {t("devices.actor_id")} }
                                        TableHead { {t("devices.display_name")} }
                                        TableHead { {t("devices.verification_status")} }
                                        TableHead { {t("devices.col_created")} }
                                        TableHead { {t("devices.col_updated")} }
                                        TableHead { {t("devices.col_revoked")} }
                                    }
                                }
                                TableBody {
                                    if resp.devices.is_empty() {
                                        TableRow {
                                            TableCell { class: "text-center text-muted-foreground py-8".to_string(), colspan: 99,
                                                {t("devices.no_devices")}
                                            }
                                        }
                                    } else {
                                        for device in resp.devices.iter() {
                                            {
                                                let id = device.id.clone();
                                                let actor_id = device.actor_id.clone().unwrap_or_else(|| "-".to_string());
                                                let display_name = device.display_name.clone().unwrap_or_else(|| "-".to_string());
                                                let verification = device.verification_label().unwrap_or("-").to_string();
                                                let created_at = device.created_at_display().unwrap_or_else(|| "-".to_string());
                                                let updated_at = device.updated_at_display().unwrap_or_else(|| "-".to_string());
                                                let revoked_at = device.revoked_at_display().unwrap_or_else(|| "-".to_string());

                                                rsx! {
                                                    TableRow {
                                                        key: "{id}",
                                                        TableCell { class: "font-medium".to_string(), "{id}" }
                                                        TableCell { class: "max-w-[200px] truncate".to_string(), "{actor_id}" }
                                                        TableCell { "{display_name}" }
                                                        TableCell { "{verification}" }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{created_at}" }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{updated_at}" }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{revoked_at}" }
                                                    }
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
