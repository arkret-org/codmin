use dioxus::prelude::*;

use crate::api::federation;
use crate::components::ui::auto_refresh::{self, AutoRefreshPicker, RefreshInterval};
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::input::SearchInput;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::pagination::CursorPagination;
use crate::components::ui::table::*;
use crate::router::Route;
use crate::types::federation::AdminFederationOperationExt;
use crate::utils::i18n::t;

const PAGE_SIZE: u64 = 25;
const AUTOREFRESH_STORAGE_KEY: &str = "codmin.federation.autorefresh";

#[component]
pub fn FederationList() -> Element {
    let mut op_cursors = use_signal(|| vec![None::<String>]);
    let mut search = use_signal(String::new);
    let mut autorefresh = use_signal(|| auto_refresh::load(AUTOREFRESH_STORAGE_KEY));

    let op_cursor = op_cursors.read().last().cloned().unwrap_or(None);
    let search_val = search.read().clone();

    let mut operations = use_resource(move || {
        let c = op_cursor.clone();
        let s = search_val.clone();
        async move { federation::list_federation_operations(c.as_deref(), PAGE_SIZE, &s).await }
    });

    let mut interval_handle = use_signal::<Option<gloo_timers::callback::Interval>>(|| None);
    use_effect(move || {
        let choice = *autorefresh.read();
        interval_handle.set(None);
        if let Some(ms) = choice.millis() {
            let mut operations = operations;
            let handle = gloo_timers::callback::Interval::new(ms, move || {
                operations.restart();
            });
            interval_handle.set(Some(handle));
        }
    });

    rsx! {
        div { class: "space-y-8",
            PageHeader {
                title: t("federation.title"),
                description: t("federation.subtitle"),
            }

            div { class: "flex items-center gap-4 flex-wrap",
                div { class: "flex-1 min-w-[240px]",
                    SearchInput {
                        placeholder: t("federation.search"),
                        value: search.read().clone(),
                        oninput: move |evt: FormEvent| {
                            search.set(evt.value());
                            op_cursors.set(vec![None::<String>]);
                        },
                    }
                    p { class: "mt-1 text-xs text-muted-foreground", {t("federation.search_current_page_hint")} }
                }
                AutoRefreshPicker {
                    storage_key: AUTOREFRESH_STORAGE_KEY.to_string(),
                    value: *autorefresh.read(),
                    onchange: move |v: RefreshInterval| autorefresh.set(v),
                }
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| operations.restart(),
                    {t("common.refresh")}
                }
            }

            match &*operations.read() {
                Some(Ok(data)) => {
                    let next_cursor = data.next_cursor.clone();
                    let depth = op_cursors.read().len();
                    rsx! {
                        div { class: "rounded-md border",
                            Table {
                                TableHeader {
                                    TableRow {
                                        TableHead { {t("federation.operation_id")} }
                                        TableHead { {t("federation.realm_id")} }
                                        TableHead { {t("federation.operation_kind")} }
                                        TableHead { {t("federation.canonical_kind")} }
                                        TableHead { {t("federation.created_at")} }
                                    }
                                }
                                TableBody {
                                    if data.data.is_empty() {
                                        TableRow {
                                            TableCell { class: "text-center text-muted-foreground py-8".to_string(), colspan: 99,
                                                {t("federation.no_operations")}
                                            }
                                        }
                                    } else {
                                        for op in data.data.iter() {
                                            {
                                                let operation_id = op.operation_id.as_str().to_owned();
                                                let realm_id = op.realm_id.as_str().to_owned();
                                                let operation_kind = op.operation_kind_label();
                                                let canonical_kind = op.canonical_kind_label();
                                                let created_at = op.created_at_display();
                                                rsx! {
                                                    TableRow {
                                                        key: "{operation_id}",
                                                        TableCell {
                                                            Link {
                                                                to: Route::FederationShow { operation_id: urlencoding::encode(&operation_id).to_string() },
                                                                class: "font-medium text-primary hover:underline",
                                                                "{operation_id}"
                                                            }
                                                        }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{realm_id}" }
                                                        TableCell { Badge { variant: BadgeVariant::Secondary, "{operation_kind}" } }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{canonical_kind}" }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{created_at}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        CursorPagination {
                            depth,
                            has_next: next_cursor.is_some(),
                            on_prev: move |_| {
                                let mut new_stack = op_cursors.read().clone();
                                if new_stack.len() > 1 {
                                    new_stack.pop();
                                    op_cursors.set(new_stack);
                                }
                            },
                            on_next: move |_| {
                                if let Some(c) = next_cursor.clone() {
                                    let mut new_stack = op_cursors.read().clone();
                                    new_stack.push(Some(c));
                                    op_cursors.set(new_stack);
                                }
                            },
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        errcode: e.body.as_ref().map(|b| b.code.clone()),
                        request_id: e.request_id.clone(),
                        retry_after_ms: e.retry_after_ms,
                        on_retry: move |_| operations.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}
