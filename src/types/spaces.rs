//! Space container admin surface — contract-authoritative row plus the
//! SPA-local hierarchy aggregate.
//!
//! The row is the shared `coland_contracts::admin::AdminSpaceView`
//! (`GET /_coland/admin/spaces`). Per-space hierarchy is assembled
//! client-side from the snapshot's `parent_space_id` fields; coland exposes
//! no hierarchy endpoint, so [`SpaceHierarchy`] never appears on the wire.

pub use coland_contracts::admin::{AdminSpaceView, SpaceHealth};
use serde::{Deserialize, Serialize};

/// Display helpers for the shared [`SpaceHealth`].
pub trait SpaceHealthExt {
    /// i18n key for the lifecycle label. Render via
    /// `crate::utils::i18n::t(health.label())` at the call site.
    fn label(&self) -> &'static str;
}

impl SpaceHealthExt for SpaceHealth {
    fn label(&self) -> &'static str {
        match self {
            SpaceHealth::Active => "spaces.health_active",
            SpaceHealth::Archived => "spaces.health_archived",
            SpaceHealth::Tombstoned => "spaces.health_tombstoned",
        }
    }
}

/// One node in the hierarchy tree derived from the admin Spaces snapshot.
/// `parent` is at most one step up; `children` is the full set of
/// immediate children found in the loaded snapshot.
///
/// SPA-local aggregate: it is assembled from [`AdminSpaceView`]s and never
/// crosses the wire.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpaceHierarchy {
    /// The Space this hierarchy is centered on.
    pub space_id: String,
    /// Display name for the centered Space.
    pub name: String,
    /// Single parent (Spaces have at most one immediate parent).
    pub parent: Option<SpaceHierarchyNode>,
    /// Direct children (no transitive descendants).
    pub children: Vec<SpaceHierarchyNode>,
}

/// A single neighbouring node — kept compact since the full Space row
/// lives behind the detail page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SpaceHierarchyNode {
    pub space_id: String,
    pub name: String,
    pub member_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_health_wire_is_strict_and_maps_to_i18n_keys() {
        for (wire, expected, label_key) in [
            ("active", SpaceHealth::Active, "spaces.health_active"),
            ("archived", SpaceHealth::Archived, "spaces.health_archived"),
            (
                "tombstoned",
                SpaceHealth::Tombstoned,
                "spaces.health_tombstoned",
            ),
        ] {
            let health: SpaceHealth =
                serde_json::from_str(&format!("\"{wire}\"")).expect("known variant");
            assert_eq!(health, expected);
            assert_eq!(health.label(), label_key);
        }
        assert!(serde_json::from_str::<SpaceHealth>("\"nope\"").is_err());
    }

    #[test]
    fn space_row_requires_the_full_producer_shape() {
        let base = serde_json::json!({
            "id": "ak:space:AaRjkifqF_BCAMvNiejGgtBBGiw19WNl2qDz_8ro1FF3",
            "name": "Space",
            "realm_id": "ak:realm:AW9pPoBBLuc_y-_fP1sxa6uOtCeZjiCax_YHAkXb9-2H",
            "kind": "list",
            "member_count": 1,
            "health": "archived",
            "created_at": "2026-08-14T00:00:00.000Z",
            "parent_space_id": null,
        });
        let row: AdminSpaceView = serde_json::from_value(base.clone()).expect("row parses");
        assert_eq!(row.health, SpaceHealth::Archived);
        assert_eq!(
            row.realm_id,
            "ak:realm:AW9pPoBBLuc_y-_fP1sxa6uOtCeZjiCax_YHAkXb9-2H"
        );

        let mut missing = base;
        missing
            .as_object_mut()
            .expect("fixture is an object")
            .remove("realm_id");
        assert!(serde_json::from_value::<AdminSpaceView>(missing).is_err());
    }
}
