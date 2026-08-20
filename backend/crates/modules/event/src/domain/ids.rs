//! Identifiants typés, un par agrégat.
//!
//! Même raison qu'en B1 et B2 : un `Uuid` nu se passe partout sans que rien ne
//! proteste, et c'est ainsi qu'un identifiant de salle finit dans un paramètre
//! attendant un lieu. Ce module en manipule neuf sortes.

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
    SeriesId,
    EventId,
    EventDayId,
    TrackId,
    VenueId,
    RoomId,
    ChannelId,
    CallId,
    CriterionId,
    PersonId,
);
