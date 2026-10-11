//! UI-facing names for coauth's shared device-admin contract.
//!
//! The wire DTOs live in `coauth-admin-types`; this module only preserves the
//! existing codmin names and presentation helpers.

use chrono::{DateTime, Utc};
pub use coauth_admin_types::{
    DeviceMfaState as CoauthDeviceMfaState, DeviceRecord as CoauthDeviceRow,
    DeviceRiskLevel as CoauthDeviceRiskLevel,
};

pub fn render_optional_timestamp(value: Option<&DateTime<Utc>>) -> String {
    value
        .map(|value| arkret_canonical::format_timestamp_canonical(value.to_owned()))
        .unwrap_or_else(|| "-".to_owned())
}

pub trait CoauthDeviceLabel {
    fn label(&self) -> &'static str;
}

impl CoauthDeviceLabel for CoauthDeviceRiskLevel {
    fn label(&self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Unknown => "Unknown",
        }
    }
}

impl CoauthDeviceLabel for CoauthDeviceMfaState {
    fn label(&self) -> &'static str {
        match self {
            Self::Verified => "Verified",
            Self::Required => "Required",
            Self::Unknown => "Unknown",
        }
    }
}
