//! Identifiants typés, un par agrégat.
//!
//! Même raison qu'en B1, B2, B3, B4 et B5 : un `Uuid` nu se passe partout sans
//! que rien ne proteste.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! identifiant {
    ($( $nom:ident ),* $(,)?) => { $(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $nom(pub Uuid);

        impl $nom {
            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $nom {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$nom> for Uuid {
            fn from(id: $nom) -> Self {
                id.0
            }
        }

        impl fmt::Display for $nom {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )* };
}

identifiant!(
    ReminderRuleId,
    ScheduledReminderId,
    NotificationId,
    TemplateId,
    // Ces trois-là appartiennent à d'autres schémas. Ils sont typés ici parce
    // que ce module les manipule, pas parce qu'il les possède : un identifiant
    // de séance et un identifiant d'édition se confondent sans cela.
    SessionId,
    EventId,
    PersonId,
);
