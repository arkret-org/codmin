#[cfg(target_arch = "wasm32")]
use gloo_storage::{LocalStorage, Storage};

#[cfg(target_arch = "wasm32")]
pub fn get_item(key: &str) -> Option<String> {
    LocalStorage::get::<String>(key).ok()
}

#[cfg(target_arch = "wasm32")]
pub fn set_item(key: &str, value: &str) {
    let _ = LocalStorage::set(key, value.to_string());
}

#[cfg(target_arch = "wasm32")]
pub fn remove_item(key: &str) {
    LocalStorage::delete(key);
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static MEMORY: std::cell::RefCell<std::collections::BTreeMap<String, String>> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}
#[cfg(not(target_arch = "wasm32"))]
pub fn get_item(key: &str) -> Option<String> {
    MEMORY.with(|values| values.borrow().get(key).cloned())
}
#[cfg(not(target_arch = "wasm32"))]
pub fn set_item(key: &str, value: &str) {
    MEMORY.with(|values| values.borrow_mut().insert(key.to_owned(), value.to_owned()));
}
#[cfg(not(target_arch = "wasm32"))]
pub fn remove_item(key: &str) {
    MEMORY.with(|values| values.borrow_mut().remove(key));
}
