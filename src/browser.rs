//! Application-level browser events, retired with the mounted application scope.
use glory::{Cage, Scope};

#[cfg(target_arch = "wasm32")]
pub fn session_sync(scope: &Scope) -> glory::ui::Result {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    let window =
        web_sys::window().ok_or(glory::render::Error::Unsupported("missing browser window"))?;
    if !crate::api::auth::is_authenticated() {
        crate::router::navigate("/login");
    }
    let weak = scope.downgrade();
    let listener =
        Closure::<dyn FnMut(web_sys::StorageEvent)>::new(move |event: web_sys::StorageEvent| {
            if weak.upgrade().is_none() {
                return;
            }
            if matches!(event.key().as_deref(), Some("session_active") | None)
                && !crate::api::auth::is_authenticated()
            {
                crate::router::navigate("/login");
            }
        });
    window
        .add_event_listener_with_callback("storage", listener.as_ref().unchecked_ref())
        .map_err(|_| {
            glory::render::Error::Unsupported("could not register session storage listener")
        })?;
    scope.on_cleanup(move || {
        let _ = window
            .remove_event_listener_with_callback("storage", listener.as_ref().unchecked_ref());
        drop(listener);
    })?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn session_sync(_scope: &Scope) -> glory::ui::Result {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn keyboard_shortcuts(scope: &Scope, show_help: Cage<bool>) -> glory::ui::Result {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    let window =
        web_sys::window().ok_or(glory::render::Error::Unsupported("missing browser window"))?;
    let weak = scope.downgrade();
    let mut pending = None::<f64>;
    let listener =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
            let Some(scope) = weak.upgrade() else {
                return;
            };
            if event.ctrl_key() || event.meta_key() || event.alt_key() || event.repeat() {
                pending = None;
                return;
            }
            if event.key() == "Escape" {
                pending = None;
                if show_help.get_untracked() {
                    event.prevent_default();
                    show_help.revise(|value| *value = false);
                }
                return;
            }
            let active = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.active_element());
            if active.as_ref().is_some_and(|element| {
                matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                    || element
                        .dyn_ref::<web_sys::HtmlElement>()
                        .is_some_and(web_sys::HtmlElement::is_content_editable)
            }) {
                pending = None;
                return;
            }
            let now = js_sys::Date::now();
            let key = event.key();
            let first = pending.take();
            if first.is_some_and(|started| (0.0..=500.0).contains(&(now - started)))
                && let Some(path) = shortcut_path(&key)
            {
                event.prevent_default();
                crate::router::navigate(path);
                return;
            }
            match key.as_str() {
                "g" => pending = Some(now),
                "?" => {
                    event.prevent_default();
                    let _ = scope.batch(|| show_help.revise(|value| *value = !*value));
                }
                _ => {}
            }
        });
    window
        .add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref())
        .map_err(|_| glory::render::Error::Unsupported("could not register keyboard shortcuts"))?;
    scope.on_cleanup(move || {
        let _ = window
            .remove_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref());
        drop(listener);
    })?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn keyboard_shortcuts(_scope: &Scope, _show_help: Cage<bool>) -> glory::ui::Result {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn shortcut_path(key: &str) -> Option<&'static str> {
    Some(match key {
        "d" => "/",
        "a" => "/actors",
        "s" => "/spaces",
        "f" => "/federation",
        "v" => "/devices",
        "c" => "/coauth/accounts",
        "l" => "/audit",
        "p" => "/policy",
        "t" => "/server-status",
        "e" => "/capabilities",
        _ => return None,
    })
}
