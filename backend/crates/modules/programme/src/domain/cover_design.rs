//! L'atelier de l'image de couverture : ce que la base garde des choix, et ce
//! qu'elle en refuse, sans base (070 § 4 ter).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use utoipa::ToSchema;

/// `CoverDesignOptions` : ce qui figure sur l'image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct CoverDesignOptions {
    pub show_date: bool,
    pub show_speakers: bool,
    pub show_logos: bool,
    pub show_event_banner: bool,
    pub show_event_icon: bool,
}

const CLES: [&str; 5] = [
    "show_date",
    "show_speakers",
    "show_logos",
    "show_event_banner",
    "show_event_icon",
];

impl CoverDesignOptions {
    /// Une clé absente, ou qui n'est pas un booléen, vaut faux : une ligne
    /// écrite avant l'ajout d'une option se relit sans reprise.
    pub fn depuis_json(valeur: &Value) -> Self {
        let lire = |cle: &str| valeur.get(cle).and_then(Value::as_bool).unwrap_or(false);
        Self {
            show_date: lire("show_date"),
            show_speakers: lire("show_speakers"),
            show_logos: lire("show_logos"),
            show_event_banner: lire("show_event_banner"),
            show_event_icon: lire("show_event_icon"),
        }
    }

    /// L'objet complet, toutes les clés écrites.
    pub fn en_json(self) -> Value {
        let valeurs = [
            self.show_date,
            self.show_speakers,
            self.show_logos,
            self.show_event_banner,
            self.show_event_icon,
        ];
        Value::Object(
            CLES.iter()
                .zip(valeurs)
                .map(|(cle, v)| ((*cle).to_owned(), Value::Bool(v)))
                .collect::<Map<_, _>>(),
        )
    }
}

/// `CoverDesign` : les choix enregistrés.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CoverDesign {
    pub template_code: String,
    pub accent: String,
    pub options: CoverDesignOptions,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// `CoverStudioContext` : tout ce que l'atelier lit, en une fois. Les images
/// ont la forme `AttachedImage` de `media.attached_image()`, nulles sans image
/// servable.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CoverStudioContext {
    /// Nul tant que l'atelier n'a jamais été enregistré pour ce dossier.
    pub design: Option<CoverDesign>,
    pub source_photo: Option<Value>,
    /// Rôle `cover` (16:9) de l'édition : la photo de fond par défaut.
    pub edition_cover: Option<Value>,
    pub edition_banner: Option<Value>,
    pub edition_thumbnail: Option<Value>,
    /// Rôle `logo` de l'édition : l'icône le préfère à la vignette.
    pub edition_logo: Option<Value>,
    pub organization_logo: Option<Value>,
}

/// La forme exigée par `ck_proposal_cover_designs_template` et `_accent` :
/// `^[a-z][a-z0-9-]{1,40}$`. Le vocabulaire lui-même vit dans le site.
pub fn code_valide(code: &str) -> bool {
    let mut caracteres = code.chars();
    let Some(premier) = caracteres.next() else {
        return false;
    };
    let reste = caracteres.as_str();
    premier.is_ascii_lowercase()
        && (1..=40).contains(&reste.len())
        && reste
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn la_forme_des_codes_suit_la_contrainte_de_la_base() {
        for bon in ["bandeau", "ab", "affiche-2", "a1-b2"] {
            assert!(code_valide(bon), "{bon}");
        }
        let long = format!("a{}", "b".repeat(40));
        assert!(code_valide(&long));
        let trop_long = format!("a{}", "b".repeat(41));
        for mauvais in [
            "",
            "a",
            "1abc",
            "-abc",
            "Bandeau",
            "ban deau",
            "bandé",
            "ban_deau",
            trop_long.as_str(),
        ] {
            assert!(!code_valide(mauvais), "{mauvais}");
        }
    }

    #[test]
    fn une_option_absente_ou_mal_typee_vaut_faux() {
        let lues = CoverDesignOptions::depuis_json(&json!({
            "show_date": true,
            "show_logos": "oui",
            "inconnue": true
        }));
        assert_eq!(
            lues,
            CoverDesignOptions {
                show_date: true,
                ..CoverDesignOptions::default()
            }
        );
        assert_eq!(
            CoverDesignOptions::depuis_json(&json!([])),
            CoverDesignOptions::default()
        );
    }

    #[test]
    fn les_options_secrivent_completes() {
        let ecrites = CoverDesignOptions {
            show_speakers: true,
            ..CoverDesignOptions::default()
        }
        .en_json();
        assert_eq!(
            ecrites,
            json!({
                "show_date": false,
                "show_speakers": true,
                "show_logos": false,
                "show_event_banner": false,
                "show_event_icon": false
            })
        );
    }
}
