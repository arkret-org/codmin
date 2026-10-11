//! Realm `media_service` admin surface — contract-authoritative types.
//!
//! The shared `coland_contracts::admin` types back
//! `GET /_coland/admin/realms/{id}/media-service`, which projects the
//! effective `ak.component.realm.media_service.v1` cell. This surface is
//! read-only in codmin: media_service writes strand through events /
//! inkson, not the operations console.

pub use coland_contracts::admin::AdminRealmMediaService;
