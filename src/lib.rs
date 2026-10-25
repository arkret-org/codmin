#![allow(clippy::result_large_err)]
#![allow(dead_code)]
mod api;
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
    let theme = if components::theme::get_resolved_theme() == "dark" {
        sapid::Theme::dark()
    } else {
        sapid::Theme::light()
    };
    let app = glory::web::launch_with(root, move |scope| router::app(scope, theme))
        .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))?;
    APP.with(|slot| *slot.borrow_mut() = Some(app));
    Ok(())
}
