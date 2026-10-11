//! `InfoRow` — migrated to yoface (`yoface::ui::info_row::InfoRow`).
//!
//! codmin's local version and the yoface version have exactly the same
//! signature (`label` / `value`), so this now re-exports the yoface component
//! directly: every `crate::components::ui::info_row::InfoRow` call site needs
//! zero changes, and rendering goes through yoface's css_module + the Soft
//! Orbit tokens.
pub use yoface::ui::info_row::InfoRow;
