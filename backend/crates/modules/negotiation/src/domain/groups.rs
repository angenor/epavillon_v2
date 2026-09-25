//! Les groupes de négociation qu'une personne suit — des codes, jamais des
//! libellés, comme les thématiques (R13). « Aucun groupe » est un choix.

use serde::{Deserialize, Serialize};

use crate::domain::themes::empreinte_des_codes;

/// `MyGroups` — ce que `GET /negotiation/me/groups` rend.
#[derive(Debug, Clone, Serialize)]
pub struct MyGroups {
    pub groups: Vec<String>,
    pub etag: String,
}

impl MyGroups {
    pub fn de(groups: Vec<String>) -> Self {
        let etag = empreinte_des_codes(groups.iter().map(String::as_str));
        Self { groups, etag }
    }
}

/// `GroupsPayload` — la liste entière des codes suivis, vide permise.
#[derive(Debug, Clone, Deserialize)]
pub struct GroupsPayload {
    pub groups: Vec<String>,
}
