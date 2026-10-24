use dioxus::prelude::*;

use crate::api::media;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::input::SearchInput;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::pagination::Pagination;
use crate::components::ui::table::*;
use crate::utils::i18n::t;

const PAGE_SIZE: u64 = 25;

#[component]
pub fn MediaList() -> Element {
    let mut search = use_signal(String::new);
    let mut page = use_signal(|| 1u64);
    let page_val = *page.read();
    let search_val = search.read().clone();

    let mut stats_data = use_resource(move || async move { media::get_media_statistics().await });
    let mut actor_media_data =
        use_resource(move || async move { media::list_media_by_actor().await });
    let mut media_data = use_resource(move || {
        let search = search_val.clone();
        async move { media::list_media(page_val, PAGE_SIZE, &search).await }
    });

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("media.title"),
                description: t("media.subtitle"),
            }

            match &*stats_data.read() {
                Some(Ok(stats)) => rsx! {
                    div { class: "grid gap-3 md:grid-cols-4",
                        {metric_tile(&t("media.blobs"), stats.total_blobs.to_string())}
                        {metric_tile(&t("media.stored"), yoface::utils::format::format_bytes(stats.total_size))}
                        {metric_tile(&t("media.encrypted"), stats.encrypted_count.to_string())}
                        {metric_tile(&t("media.quarantined"), stats.quarantined_count.to_string())}
                    }
                },
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        errcode: e.body.as_ref().map(|body| body.code.clone()),
                        request_id: e.request_id.clone(),
                        retry_after_ms: e.retry_after_ms,
                        on_retry: move |_| stats_data.restart(),
                    }
                },
                None => rsx! {
                    div { class: "grid gap-3 md:grid-cols-4",
                        for _ in 0..4 {
                            div { class: "h-20 rounded-md border bg-muted/30" }
                        }
                    }
                },
            }

            match &*actor_media_data.read() {
                Some(Ok(resp)) => actor_media_section(resp),
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        errcode: e.body.as_ref().map(|body| body.code.clone()),
                        request_id: e.request_id.clone(),
                        retry_after_ms: e.retry_after_ms,
                        on_retry: move |_| actor_media_data.restart(),
                    }
                },
                None => rsx! { div { class: "h-24 rounded-md border bg-muted/30" } },
            }

            SearchInput {
                placeholder: t("media.search"),
                value: search.read().clone(),
                oninput: move |evt: FormEvent| {
                    search.set(evt.value());
                    page.set(1);
                },
            }
            p { class: "-mt-5 text-xs text-muted-foreground", {t("media.search_all_pages_hint")} }

            match &*media_data.read() {
                Some(Ok(data)) => {
                    rsx! {
                        div { class: "rounded-md border",
                            Table {
                                TableHeader {
                                    TableRow {
                                        TableHead { {t("media.col_filename")} }
                                        TableHead { {t("media.col_type")} }
                                        TableHead { {t("media.col_realm")} }
                                        TableHead { {t("media.col_uploaded_by")} }
                                        TableHead { {t("media.col_size")} }
                                        TableHead { {t("media.encrypted")} }
                                        TableHead { {t("media.col_created")} }
                                    }
                                }
                                TableBody {
                                    if data.data.is_empty() {
                                        TableRow {
                                            TableCell { class: "text-center text-muted-foreground py-8".to_string(), colspan: 99,
                                                {t("media.no_media")}
                                            }
                                        }
                                    } else {
                                        for row in data.data.iter() {
                                            {
                                                let filename = row.filename.clone().unwrap_or_else(|| "-".to_string());
                                                let media_type = row.media_type.clone();
                                                let realm_id = row.realm_id.clone().unwrap_or_else(|| "-".to_string());
                                                let uploaded_by = row.uploaded_by.clone();
                                                let size = yoface::utils::format::format_bytes(row.size_bytes);
                                                let encrypted = if row.encrypted {
                                                    t("media.encrypted_yes")
                                                } else {
                                                    t("media.encrypted_no")
                                                };
                                                let created = row.created_at.to_rfc3339();
                                                rsx! {
                                                    TableRow {
                                                        TableCell { "{filename}" }
                                                        TableCell { "{media_type}" }
                                                        TableCell { class: "font-mono text-xs max-w-[220px] truncate".to_string(), "{realm_id}" }
                                                        TableCell { class: "font-mono text-xs max-w-[220px] truncate".to_string(), "{uploaded_by}" }
                                                        TableCell { "{size}" }
                                                        TableCell { "{encrypted}" }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{created}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(total) = data.total {
                            Pagination {
                                page: page_val,
                                total,
                                per_page: PAGE_SIZE,
                                on_page_change: move |p| page.set(p),
                            }
                        } else {
                            p { class: "text-sm text-muted-foreground", {t("pagination.total_unknown")} }
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        errcode: e.body.as_ref().map(|body| body.code.clone()),
                        request_id: e.request_id.clone(),
                        retry_after_ms: e.retry_after_ms,
                        on_retry: move |_| media_data.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}

fn metric_tile(label: &str, value: String) -> Element {
    rsx! {
        div { class: "rounded-md border p-3",
            div { class: "text-xs uppercase text-muted-foreground", "{label}" }
            div { class: "mt-1 text-lg font-semibold", "{value}" }
        }
    }
}

fn actor_media_section(resp: &crate::types::AdminMediaByActorList) -> Element {
    rsx! {
        div { class: "rounded-md border",
            Table {
                TableHeader {
                    TableRow {
                        TableHead { {t("media.col_actor")} }
                        TableHead { {t("media.col_display_name")} }
                        TableHead { {t("media.blobs")} }
                        TableHead { {t("media.stored")} }
                    }
                }
                TableBody {
                    if resp.data.is_empty() {
                        TableRow {
                            TableCell { class: "text-center text-muted-foreground py-6".to_string(), colspan: 99,
                                {t("media.no_media")}
                            }
                        }
                    } else {
                        for row in resp.data.iter() {
                            {
                                let actor_id = row.actor_id.clone();
                                let display_name = row.display_name.clone().unwrap_or_else(|| "-".to_string());
                                let blob_count = row.blob_count.to_string();
                                let total_size = yoface::utils::format::format_bytes(row.total_size);
                                rsx! {
                                    TableRow {
                                        key: "{actor_id}",
                                        TableCell { class: "font-mono text-xs max-w-[260px] truncate".to_string(), "{actor_id}" }
                                        TableCell { "{display_name}" }
                                        TableCell { "{blob_count}" }
                                        TableCell { "{total_size}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
