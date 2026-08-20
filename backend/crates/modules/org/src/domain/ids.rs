//! Identifiants typés, un par agrégat.
//!
//! Même raison qu'en B1 : un `Uuid` nu se passe partout sans que rien ne
//! proteste, et c'est ainsi qu'un identifiant d'adhésion finit dans un
//! paramètre attendant une organisation.

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
    OrganizationId,
    OrganizationNameId,
    OrganizationDomainId,
    MembershipId,
    DuplicatePairId,
    PersonId,
);
