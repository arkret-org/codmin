use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::StorageEvent;

use crate::api::auth;
use crate::components::dev_mode_banner::DevModeBanner;
use crate::components::header::AppHeader;
use crate::components::keyboard_shortcuts::KeyboardShortcuts;
use crate::components::sidebar::AppSidebar;
use crate::components::ui::toast::Toaster;
use crate::router::Route;

/// Owns a `storage` event listener for the lifetime of the component.
/// `add_event_listener_with_callback` requires the function pointer to
/// stay alive until removal, hence keeping the `Closure` boxed; the
/// `Drop` impl detaches it cleanly when the layout unmounts.
struct StorageListenerGuard {
    closure: Option<Closure<dyn FnMut(StorageEvent)>>,
}

impl Drop for StorageListenerGuard {
    fn drop(&mut self) {
        if let (Some(window), Some(closure)) = (web_sys::window(), self.closure.as_ref()) {
            let _ = window
                .remove_event_listener_with_callback("storage", closure.as_ref().unchecked_ref());
        }
    }
}

#[component]
pub fn AppLayout(children: Element) -> Element {
    let collapsed = use_signal(|| false);
    let mut mobile_sidebar_open = use_signal(|| false);
    let nav = use_navigator();

    // Multi-tab session sync via the `storage` event: it fires in this
    // tab when *another* tab mutates localStorage, so we react to logout
    // immediately instead of polling.
    use_hook(move || {
        let closure = Closure::<dyn FnMut(StorageEvent)>::new(move |event: StorageEvent| {
            // S5: the bearer credential is now in an httpOnly cookie
            // and never lives in localStorage. The only auth-bearing
            // signal we still listen on is the non-secret
            // `session_active` marker — when another tab clears it
            // (logout) we mirror the redirect immediately. `None` is
            // the wholesale `localStorage.clear()` case (still fires).
            let key = event.key();
            let is_auth_key = matches!(key.as_deref(), Some("session_active") | None);
            if is_auth_key && !auth::is_authenticated() {
                nav.replace(Route::LoginPage {
                    logout_warning: None,
                });
            }
        });

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("storage", closure.as_ref().unchecked_ref());
        }

        std::rc::Rc::new(StorageListenerGuard {
            closure: Some(closure),
        })
    });

    let mobile_sidebar_state = *mobile_sidebar_open.read();

    rsx! {
        div { class: "flex h-screen overflow-hidden",
            div {
                class: if mobile_sidebar_state {
                    "sidebar-backdrop sidebar-backdrop-open"
                } else {
                    "sidebar-backdrop"
                },
                onclick: move |_| mobile_sidebar_open.set(false),
            }
            AppSidebar {
                collapsed,
                mobile_open: mobile_sidebar_open,
            }
            div { class: "flex flex-1 flex-col overflow-hidden",
                // T1.4: red, non-dismissible warning when the connected
                // coland reports `development_mode == true`. Rendered
                // ABOVE the header so it stays in the operator's eyeline
                // no matter which page they're on.
                DevModeBanner {}
                AppHeader {
                    collapsed,
                    mobile_sidebar_open,
                }
                main { class: "flex-1 overflow-auto p-4 md:p-6", role: "main",
                    {children}
                }
            }
        }
        Toaster {}
        KeyboardShortcuts {}
    }
}
