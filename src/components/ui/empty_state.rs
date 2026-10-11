//! `EmptyState` has been migrated to yoface
//! (`yoface::ui::empty_state::EmptyState`).
//!
//! Call site rename (handled as part of the migration):
//! `icon: "name".to_string()` → `icon_name: "name".to_string()` (yoface maps
//! lucide kebab-case names to built-in icons and no longer maintains a local
//! string icon registry). The action is still `action_label` + `action_href`
//! (`<a href>`), which none of codmin's existing call sites use.
pub use yoface::ui::empty_state::EmptyState;
