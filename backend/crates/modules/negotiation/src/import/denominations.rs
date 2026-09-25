//! Trier et rattacher une réunion par les `metadata` des vocabulaires
//! `negotiation_meeting_type` et `negotiation_group` (research R4) : aucune
//! liste de noms dans le code.

use serde_json::Value;
use uuid::Uuid;

/// Le seul type dont les réunions se rattachent à un groupe.
pub const COORDINATION: &str = "group_coordination";

/// Un terme tel que lu en base, dans l'ordre de `sort_order`.
pub struct TermeLu {
    pub id: Uuid,
    pub code: String,
    pub metadata: Value,
}

pub struct TypeReunion {
    pub id: Uuid,
    pub code: String,
    categories: Vec<String>,
    denominations: Vec<String>,
    exige_titre: bool,
    par_defaut_pour: Vec<String>,
}

pub struct Groupe {
    pub id: Uuid,
    pub code: String,
    denominations: Vec<String>,
}

pub struct Vocabulaires {
    types: Vec<TypeReunion>,
    groupes: Vec<Groupe>,
}

impl Vocabulaires {
    /// L'ordre reçu est l'ordre de résolution : le plus précis d'abord.
    pub fn depuis(types: Vec<TermeLu>, groupes: Vec<TermeLu>) -> Self {
        Self {
            types: types
                .into_iter()
                .map(|t| TypeReunion {
                    categories: liste(&t.metadata, "source_categories"),
                    denominations: liste(&t.metadata, "denominations"),
                    exige_titre: t.metadata["requires_title_match"].as_bool() == Some(true),
                    par_defaut_pour: liste(&t.metadata, "default_for"),
                    id: t.id,
                    code: t.code,
                })
                .collect(),
            groupes: groupes
                .into_iter()
                .map(|g| Groupe {
                    denominations: liste(&g.metadata, "denominations"),
                    id: g.id,
                    code: g.code,
                })
                .collect(),
        }
    }

    /// Nul : aucune catégorie de la réunion n'est admise, elle est écartée.
    pub fn type_de(&self, categories: &[String], titre: &str) -> Option<&TypeReunion> {
        let categories: Vec<String> = categories.iter().map(|c| normaliser(c)).collect();
        let admise = |liste: &[String]| liste.iter().any(|c| categories.contains(c));
        let titre = normaliser(titre);

        self.types
            .iter()
            .find(|t| {
                admise(&t.categories) && t.denominations.iter().any(|d| contient_mots(&titre, d))
            })
            .or_else(|| {
                self.types
                    .iter()
                    .find(|t| !t.exige_titre && admise(&t.par_defaut_pour))
            })
    }

    pub fn groupe_de(&self, titre: &str) -> Option<&Groupe> {
        let titre = normaliser(titre);
        self.groupes
            .iter()
            .find(|g| g.denominations.iter().any(|d| contient_mots(&titre, d)))
    }

    pub fn code_du_type(&self, id: Uuid) -> Option<&str> {
        self.types
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.code.as_str())
    }
}

fn liste(metadata: &Value, cle: &str) -> Vec<String> {
    metadata[cle]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(Value::as_str)
                .map(normaliser)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Minuscules, sans accents ni ponctuation, espaces resserrés.
pub fn normaliser(texte: &str) -> String {
    let mut sortie = String::with_capacity(texte.len());
    for c in texte.chars().flat_map(char::to_lowercase) {
        match sans_accent(c) {
            Some(s) => sortie.push_str(s),
            None if c.is_alphanumeric() => sortie.push(c),
            None => sortie.push(' '),
        }
    }
    sortie.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sans_accent(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
        'ç' => "c",
        'è' | 'é' | 'ê' | 'ë' => "e",
        'ì' | 'í' | 'î' | 'ï' => "i",
        'ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' => "o",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'ý' | 'ÿ' => "y",
        'œ' => "oe",
        'æ' => "ae",
        _ => return None,
    })
}

/// En mots entiers : « eig » n'est pas dans « sovereign », ni « ldc » dans
/// « lldcs ».
pub fn contient_mots(texte_normalise: &str, fragment_normalise: &str) -> bool {
    !fragment_normalise.is_empty()
        && format!(" {texte_normalise} ").contains(&format!(" {fragment_normalise} "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn terme(code: &str, metadata: Value) -> TermeLu {
        TermeLu {
            id: Uuid::now_v7(),
            code: code.into(),
            metadata,
        }
    }

    /// Un extrait du semis de `020_reference.sql`, dans son ordre.
    fn vocabulaires() -> Vocabulaires {
        let types = vec![
            terme(
                "plenary",
                json!({"source_categories":["Plenary"],"denominations":["plenary"],"requires_title_match":false,"default_for":["Plenary"]}),
            ),
            terme(
                "heads_of_delegation",
                json!({"source_categories":["Negotiations"],"denominations":["hods","heads of delegation"],"requires_title_match":false,"default_for":[]}),
            ),
            terme(
                "presidency_consultation",
                json!({"source_categories":["Negotiations","Presidency event"],"denominations":["presidency consultation"],"requires_title_match":true,"default_for":[]}),
            ),
            terme(
                "informal_informals",
                json!({"source_categories":["Negotiations"],"denominations":["informal informals"],"requires_title_match":false,"default_for":[]}),
            ),
            terme(
                "informal_consultations",
                json!({"source_categories":["Negotiations"],"denominations":["informal consultation","informal consultations"],"requires_title_match":false,"default_for":[]}),
            ),
            terme(
                "group_coordination",
                json!({"source_categories":["Coordination meetings"],"denominations":[],"requires_title_match":false,"default_for":["Coordination meetings"]}),
            ),
            terme(
                "negotiation_other",
                json!({"source_categories":["Negotiations"],"denominations":[],"requires_title_match":false,"default_for":["Negotiations"]}),
            ),
        ];
        let groupes = vec![
            terme(
                "african_group",
                json!({"denominations":["African Group","African Group of Negotiators","AGN"]}),
            ),
            terme(
                "ldc",
                json!({"denominations":["LDC","LDCs","Pays les moins avancés"]}),
            ),
            terme("g77_china", json!({"denominations":["G77 & China","G77"]})),
            terme("eig", json!({"denominations":["EIG"]})),
        ];
        Vocabulaires::depuis(types, groupes)
    }

    fn type_de(v: &Vocabulaires, categorie: &str, titre: &str) -> Option<String> {
        v.type_de(&[categorie.to_owned()], titre)
            .map(|t| t.code.clone())
    }

    fn groupe_de(v: &Vocabulaires, titre: &str) -> Option<String> {
        v.groupe_de(titre).map(|g| g.code.clone())
    }

    #[test]
    fn normaliser_ote_casse_accents_et_ponctuation() {
        assert_eq!(normaliser("G77 & China"), "g77 china");
        assert_eq!(
            normaliser("  Réunion  des PMA — Œuvre "),
            "reunion des pma oeuvre"
        );
        assert_eq!(
            normaliser("HoDs draftng group CMA 11 (a) -NCQG"),
            "hods draftng group cma 11 a ncqg"
        );
    }

    #[test]
    fn une_categorie_non_admise_est_ecartee() {
        let v = vocabulaires();
        assert_eq!(
            type_de(&v, "Side events", "Informal consultation on oceans"),
            None
        );
        assert_eq!(
            type_de(&v, "Press conferences", "COP 30 Presidency Press Briefing"),
            None
        );
    }

    #[test]
    fn le_titre_nomme_le_type_sinon_le_defaut_de_la_categorie() {
        let v = vocabulaires();
        assert_eq!(
            type_de(
                &v,
                "Negotiations",
                "CMA 8 (a) Global goal on adaptation - Informal Consultation"
            )
            .as_deref(),
            Some("informal_consultations")
        );
        assert_eq!(
            type_de(&v, "Negotiations", "HoDs draftng group CMA 11 (a) -NCQG").as_deref(),
            Some("heads_of_delegation")
        );
        assert_eq!(
            type_de(
                &v,
                "Negotiations",
                "Mutirao Mobilization for the Belem package"
            )
            .as_deref(),
            Some("negotiation_other")
        );
        assert_eq!(
            type_de(&v, "Plenary", "High-level segment (HLS) of COP 30").as_deref(),
            Some("plenary")
        );
    }

    #[test]
    fn un_type_qui_exige_son_nom_n_admet_que_les_titres_qui_le_nomment() {
        let v = vocabulaires();
        assert_eq!(
            type_de(
                &v,
                "Presidency event",
                "COP Presidency Consultation on Mountains"
            )
            .as_deref(),
            Some("presidency_consultation")
        );
        assert_eq!(
            type_de(&v, "Presidency event", "Youth on Action Agenda"),
            None
        );
    }

    #[test]
    fn une_coordination_se_rattache_en_mots_entiers() {
        let v = vocabulaires();
        assert_eq!(
            type_de(
                &v,
                "Coordination meetings",
                "YOUNGO Morning Coordination Meeting"
            )
            .as_deref(),
            Some("group_coordination")
        );
        assert_eq!(
            groupe_de(&v, "EIG Coordination Meeting").as_deref(),
            Some("eig")
        );
        assert_eq!(
            groupe_de(&v, "AGN Gender Coordination").as_deref(),
            Some("african_group")
        );
        assert_eq!(
            groupe_de(&v, "G77 & China - Breakout room 5").as_deref(),
            Some("g77_china")
        );
        assert_eq!(
            groupe_de(&v, "Coordination des PAYS LES MOINS AVANCES").as_deref(),
            Some("ldc")
        );
        assert_eq!(groupe_de(&v, "Sovereign debt coordination"), None);
        assert_eq!(groupe_de(&v, "LLDCs coordination"), None);
        assert_eq!(groupe_de(&v, "YOUNGO Morning Coordination Meeting"), None);
    }
}
