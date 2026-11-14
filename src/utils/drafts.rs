//! Document-local draft protection; no input values are persisted by this module.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    Clean,
    Unsaved,
    Busy,
}

type Probe = Box<dyn Fn() -> State>;
thread_local! {
    static PROBES: RefCell<BTreeMap<u64, Probe>> = RefCell::new(BTreeMap::new());
    static NEXT: Cell<u64> = const { Cell::new(0) };
    static PERMIT: Cell<bool> = const { Cell::new(false) };
}

pub fn track(scope: &glory::Scope, probe: impl Fn() -> State + 'static) {
    let id = NEXT.with(|next| {
        let id = next.get().wrapping_add(1);
        next.set(id);
        id
    });
    let owner = scope.downgrade();
    PROBES.with(|probes| {
        probes.borrow_mut().insert(
            id,
            Box::new(move || {
                if owner.upgrade().is_some() {
                    probe()
                } else {
                    State::Clean
                }
            }),
        );
    });
    if let Err(error) = scope.on_cleanup(move || {
        PROBES.with(|probes| {
            probes.borrow_mut().remove(&id);
        });
    }) {
        PROBES.with(|probes| {
            probes.borrow_mut().remove(&id);
        });
        log::error!("draft protection owner unavailable: {error}");
    }
}

pub fn state() -> State {
    PROBES.with(|probes| {
        probes
            .borrow()
            .values()
            .map(|probe| probe())
            .max()
            .unwrap_or(State::Clean)
    })
}

pub fn permit_navigation() {
    PERMIT.with(|permit| permit.set(true));
}

#[cfg(target_arch = "wasm32")]
pub fn protect_navigation(scope: &glory::Scope) -> glory::ui::Result {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else {
        return Ok(());
    };
    let listener = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent)>::new(
        |event: web_sys::BeforeUnloadEvent| {
            if PERMIT.with(|permit| permit.replace(false)) || !crate::api::auth::is_authenticated()
            {
                return;
            }
            if state() != State::Clean {
                event.prevent_default();
                event.set_return_value("");
            }
        },
    );
    window
        .add_event_listener_with_callback("beforeunload", listener.as_ref().unchecked_ref())
        .map_err(|_| glory::render::Error::Unsupported("could not protect unsaved admin input"))?;
    scope.on_cleanup(move || {
        let _ = window
            .remove_event_listener_with_callback("beforeunload", listener.as_ref().unchecked_ref());
        drop(listener);
    })?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn protect_navigation(_scope: &glory::Scope) -> glory::ui::Result {
    Ok(())
}
