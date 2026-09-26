//! Les réglages de notification de Guide Négo : l'accord « Notifications »
//! (courriel) et les thématiques dont on veut être prévenu.

use serde::{Deserialize, Serialize};

/// `NotificationSettings` — `GET`/`PUT /negotiation/me/notifications`.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationSettings {
    pub email: bool,
    /// La version de la politique de confidentialité servie.
    pub version: String,
}

/// `NotificationSettingsPayload`.
#[derive(Debug, Clone, Deserialize)]
pub struct NotificationSettingsPayload {
    pub email: bool,
}

/// `ThemeNotificationsPayload` — la liste entière des thématiques allumées.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeNotificationsPayload {
    pub codes: Vec<String>,
}
