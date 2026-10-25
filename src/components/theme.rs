use crate::utils::storage;

#[derive(Debug, Clone, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Theme {
    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
}

fn get_system_theme() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    let matches = web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
        })
        .map(|m| m.matches())
        .unwrap_or(false);
    #[cfg(not(target_arch = "wasm32"))]
    let matches = false;
    if matches { "dark" } else { "light" }
}

pub fn get_resolved_theme() -> String {
    let stored = storage::get_item("theme").unwrap_or_else(|| "system".to_string());
    let theme = Theme::from_str(&stored);
    match theme {
        Theme::Light => "light".to_string(),
        Theme::Dark => "dark".to_string(),
        Theme::System => get_system_theme().to_string(),
    }
}

pub fn apply_theme() {
    let resolved = get_resolved_theme();
    if let Some(document) = web_sys::window().and_then(|w| w.document())
        && let Some(root) = document.document_element()
    {
        let _ = root.class_list().remove_2("light", "dark");
        let _ = root.class_list().add_1(&resolved);
    }
}

pub fn set_theme(theme: &str) {
    storage::set_item("theme", theme);
    apply_theme();
}
