//! Ce que le back-office de la vitrine décide avant d'écrire.
//!
//! # Le périmètre d'une diapositive, et pourquoi `null` est la contrainte forte
//!
//! `event_id` nul ne veut pas dire « aucune contrainte » mais « contenu de
//! plateforme » : la diapositive parle au nom de l'IFDD entière, et seule la
//! portée globale peut la toucher. C'est écrit ici, une fois, et appelé par les
//! six écritures — c'est ce qui garantit que la lecture et l'écriture refusent
//! la même chose.
//!
//! # La validation double celle du modèle, elle ne la remplace pas
//!
//! Les cinq premiers codes reprennent une contrainte de `115_content.sql`. Les
//! vérifier ici sert à rendre un refus **exploitable par le formulaire**, posé
//! sur son champ ; la base, elle, reste l'autorité — si un chemin oubliait un
//! contrôle, elle refuserait quand même, et le code traduirait son erreur.

use kernel::auth::AdminScope;
use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::admin::{ShowcaseFormValues, ShowcaseValidationError};

/// Le refus d'un contenu de plateforme, en français exploitable par l'écran.
pub const REFUS_PLATEFORME: &str =
    "Ce contenu s'affiche sur toute la plateforme : sa modification demande la portée globale.";

/// Une diapositive est-elle dans le périmètre de l'appelant ?
///
/// Un hors-périmètre se refuse comme un inexistant (principe IX) — sauf le
/// contenu de plateforme, qui se refuse en le **disant** : l'écran doit pouvoir
/// expliquer pourquoi une ligne visible n'est pas modifiable.
pub fn assurer_le_perimetre(event_id: Option<Uuid>, scope: &AdminScope) -> Result<()> {
    match event_id {
        None if !scope.is_global => Err(ApiError::with_message(
            ErrorCode::Forbidden,
            REFUS_PLATEFORME,
        )),
        None => Ok(()),
        Some(id) if scope.allows(id) => Ok(()),
        Some(_) => Err(ApiError::not_found()),
    }
}

/// Le français est la langue pivot de la base : un champ multilingue sans lui
/// est un champ que la vue publique rendra vide.
fn francais_present(valeur: &Value) -> bool {
    valeur
        .get("fr")
        .and_then(Value::as_str)
        .is_some_and(|texte| !texte.trim().is_empty())
}

fn est_vide(valeur: &Option<Value>) -> bool {
    match valeur {
        None => true,
        Some(Value::Null) => true,
        Some(Value::Object(map)) => map.is_empty(),
        _ => false,
    }
}

/// Les neuf refus du contrat, dans l'ordre où le formulaire les pose.
pub fn valider(valeurs: &ShowcaseFormValues, scope: &AdminScope) -> Vec<ShowcaseValidationError> {
    let mut erreurs = Vec::new();

    if valeurs.nature_code.trim().is_empty() {
        erreurs.push(ShowcaseValidationError::new("nature_code", "required"));
    }

    if !francais_present(&valeurs.title) {
        erreurs.push(ShowcaseValidationError::new("title", "french_required"));
    }

    for (champ, valeur) in [
        ("quote", &valeurs.quote),
        ("body", &valeurs.body),
        ("author_title", &valeurs.author_title),
    ] {
        if let Some(v) = valeur {
            if !est_vide(&Some(v.clone())) && !francais_present(v) {
                erreurs.push(ShowcaseValidationError::new(champ, "french_required"));
            }
        }
    }

    // `ck_highlights_organization_shape` — désignée ET nommée.
    if valeurs.organization_id.is_some()
        && valeurs
            .organization_label
            .as_ref()
            .is_some_and(|l| !l.trim().is_empty())
    {
        erreurs.push(ShowcaseValidationError::new(
            "organization_label",
            "organization_both",
        ));
    }

    // `ck_highlights_link_shape` — un libellé de lien sans lien.
    let lien_absent = valeurs
        .link_url
        .as_ref()
        .is_none_or(|url| url.trim().is_empty());
    if lien_absent && !est_vide(&valeurs.link_label) {
        erreurs.push(ShowcaseValidationError::new(
            "link_label",
            "link_label_without_url",
        ));
    }

    // `platform.url` — adresse absolue http(s).
    if let Some(url) = valeurs.link_url.as_ref().filter(|u| !u.trim().is_empty()) {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            erreurs.push(ShowcaseValidationError::new("link_url", "url_format"));
        }
    }

    // `background_color_hex ~ '^#[0-9a-fA-F]{6}$'`
    if let Some(couleur) = valeurs
        .background_color_hex
        .as_ref()
        .filter(|c| !c.trim().is_empty())
    {
        let forme_valide = couleur.len() == 7
            && couleur.starts_with('#')
            && couleur[1..].chars().all(|c| c.is_ascii_hexdigit());
        if !forme_valide {
            erreurs.push(ShowcaseValidationError::new(
                "background_color_hex",
                "color_format",
            ));
        }
    }

    // `ck_highlights_window` — la fin précède le début.
    if let (Some(debut), Some(fin)) = (valeurs.starts_at, valeurs.ends_at) {
        if fin <= debut {
            erreurs.push(ShowcaseValidationError::new("ends_at", "window_inverted"));
        }
    }

    // Contenu de plateforme sans portée globale.
    if valeurs.event_id.is_none() && !scope.is_global {
        erreurs.push(ShowcaseValidationError::new(
            "event_id",
            "global_scope_required",
        ));
    }

    // Une édition hors périmètre ne se choisit pas — le menu ne l'offre pas, et
    // une requête forgée ne doit pas passer par la fenêtre.
    if let Some(event_id) = valeurs.event_id {
        if !scope.allows(event_id) {
            erreurs.push(ShowcaseValidationError::new(
                "event_id",
                "global_scope_required",
            ));
        }
    }

    erreurs
}
