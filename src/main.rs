// `HttpError` is the canonical SPA error type and is wired through every
// `api/*` module's `Result<T, HttpError>` signature. Its size (a string
// message, optional envelope, optional retry hint) is intentional and
// boxing every call site would add noise without measurable benefit.
#![allow(clippy::result_large_err)]

mod api;
mod components;
mod pages;
mod router;
mod types;
mod utils;

use dioxus::prelude::*;

fn main() {
    let _ = dioxus_logger::init(dioxus_logger::tracing::Level::INFO);

    components::theme::apply_theme();
    utils::i18n::sync_document_language();

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Design token injection order:
        //   1. yoface default tokens.
        //   2. Soft Orbit overrides for codmin's brand aliases.
        //   3. codmin utility tokens.
        // See the top of yoface_tokens.css for the full rationale.
        style { {yoface::TOKENS_CSS} }
        style { {include_str!("./yoface_tokens.css")} }
        style { {include_str!("./style.css")} }
        router::AppRouter {}
    }
}
