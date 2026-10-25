#![allow(clippy::result_large_err)]
#![allow(dead_code)]
mod api;
mod appearance;
mod browser;
mod components;
mod logging;
mod pages;
mod router;
mod types;
mod utils;

#[cfg(target_arch = "wasm32")]
thread_local! { static APP: std::cell::RefCell<Option<glory::web::WebApp>> = const { std::cell::RefCell::new(None) }; }

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    logging::init();
    components::theme::apply_theme();
    utils::i18n::sync_document_language();
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("missing document")?;
    let root = document
        .get_element_by_id("app")
        .ok_or("missing app root")?;
    appearance::install_styles(&document)?;
    let theme = appearance::theme(components::theme::get_resolved_theme() == "dark");
    let app = glory::web::launch_with(root, move |scope| router::app(scope, theme))
        .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))?;
    // Keep application rules after the host's semantic reset in the cascade.
    if let Some(stylesheet) = document.query_selector("link[href='/app.css']")?
        && let Some(parent) = stylesheet.parent_node()
    {
        parent.append_child(&stylesheet)?;
    }
    APP.with(|slot| *slot.borrow_mut() = Some(app));
    Ok(())
}
