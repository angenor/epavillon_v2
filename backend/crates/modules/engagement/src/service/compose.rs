//! **Du type au courriel** : le rappel d'une activité.
//!
//! # Quel texte part, dans quel ordre (04/11)
//!
//! 1. le texte propre à l'envoi, écrit dans l'onglet « Programmés » ;
//! 2. le texte du type pour l'édition, même onglet ;
//! 3. un modèle publié (`message_templates`) du type, s'il en existe un ;
//! 4. le texte par défaut, écrit dans le code (`domain::programmes`).
//!
//! 1, 2 et 4 partent dans l'habillage des confirmations d'inscription (logo de
//! l'édition, bouton « Voir l'activité », lien en clair). Un modèle publié garde
//! son propre HTML. La trace porte `template_id` nul hors du cas 3 : aucun
//! modèle publié n'a servi (R27).
//!
//! # Une variable manquante FAIT ÉCHOUER l'envoi, en la nommant
//!
//! Le modèle l'écrit lui-même : *« mieux vaut un job en échec visible qu'un
//! email "Bonjour  ," envoyé à 2 000 personnes »*. Un travail en échec porte son
//! message et se reprend ; un courriel amputé est parti pour toujours.
//!
//! # Le lien mène à la page de l'activité sur le site
//!
//! `/programmations/{édition}/{activité}`, `/en/programmes/…` en anglais
//! (03/11/2026 : l'ancien `/event/…` rendait 404).

use kernel::error::{ApiError, Result};
use kernel::mail::OutgoingMail;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::gabarits::Visuels;
use crate::domain::offsets;
use crate::domain::programmes::{self, langue, CleEnvoi, Nature};
use crate::domain::render;
use crate::repo::cross::{DestinataireCourriel, SeancePourRappel};
use crate::repo::{programmes as programmes_repo, templates};
use crate::service::programmes::{composer, texte_ecrit, RappelProgramme};

/// Ce qui a servi à composer.
#[derive(Debug, Clone)]
pub struct Compose {
    pub mail: OutgoingMail,
    /// Le modèle **et le numéro de révision** réellement servis (FR-089). Nul
    /// hors d'un modèle publié : la trace le **dit** (R27).
    pub template: Option<(Uuid, i16)>,
    /// Vrai : l'habillage des confirmations, dont le logo part en pièce
    /// intégrée.
    pub habille: bool,
}

pub struct RappelACompose<'a> {
    pub message_id: &'a str,
    pub destinataire: &'a DestinataireCourriel,
    pub seance: &'a SeancePourRappel,
    pub session_id: Uuid,
    pub offset_minutes: i32,
    pub type_code: &'a str,
    pub template_id: Option<Uuid>,
    pub app_public_url: &'a str,
    pub visuels: Option<&'a Visuels>,
}

fn en_anglais(locale: &str) -> bool {
    locale.starts_with("en")
}

/// Les cinq variables que `programme.session.reminder` promet dans
/// `notification_types.expected_variables`, celles d'un modèle publié.
fn variables(rappel: &RappelACompose<'_>) -> HashMap<String, String> {
    let anglais = en_anglais(&rappel.destinataire.locale);
    HashMap::from([
        ("prenom".to_owned(), rappel.destinataire.first_name.clone()),
        ("titre_session".to_owned(), rappel.seance.titre.clone()),
        (
            "date_session".to_owned(),
            format!("{} ({})", rappel.seance.debut_local, rappel.seance.timezone),
        ),
        (
            "delai".to_owned(),
            offsets::libelle_delai(rappel.offset_minutes, anglais),
        ),
        (
            "lien_participation".to_owned(),
            programmes::lien(
                rappel.app_public_url,
                Nature::Activite,
                &rappel.seance.event_slug,
                &rappel.seance.slug,
                anglais,
            ),
        ),
    ])
}

/// Compose le rappel dans la langue du destinataire.
pub async fn rappel(pool: &PgPool, rappel: &RappelACompose<'_>) -> Result<Compose> {
    let locale = &rappel.destinataire.locale;
    let cle = CleEnvoi {
        nature: Nature::Activite,
        cible: rappel.session_id,
        offset_minutes: rappel.offset_minutes,
        debut: None,
    };
    let ecrit = texte_ecrit(pool, rappel.seance.event_id, &cle, langue(locale)).await?;

    if ecrit.is_none() {
        if let Some(revision) =
            templates::revision_servie(pool, rappel.template_id, rappel.type_code).await?
        {
            return modele_publie(rappel, &revision);
        }
    }

    let Some(cible) = programmes_repo::cible_activite(pool, rappel.session_id, locale).await?
    else {
        return Err(ApiError::internal("séance du rappel introuvable"));
    };
    let texte =
        ecrit.unwrap_or_else(|| programmes::texte_par_defaut(Nature::Activite, en_anglais(locale)));
    let mail = composer(&RappelProgramme {
        nature: Nature::Activite,
        cible: &cible,
        prenom: &rappel.destinataire.first_name,
        to: &rappel.destinataire.email,
        locale,
        offset_minutes: i64::from(rappel.offset_minutes),
        texte: &texte,
        site: rappel.app_public_url,
        visuels: rappel.visuels,
        message_id: rappel.message_id,
        surligner: false,
    })?;
    Ok(Compose {
        mail,
        template: None,
        habille: true,
    })
}

fn modele_publie(
    rappel: &RappelACompose<'_>,
    revision: &templates::RevisionServie,
) -> Result<Compose> {
    let locale = &rappel.destinataire.locale;
    let valeurs = variables(rappel);
    let sujet = rendre(&textuel(&revision.subject, locale), &valeurs)?;
    let html = rendre(&textuel(&revision.body_html, locale), &valeurs)?;
    let texte = match revision.body_text.as_ref() {
        Some(brut) => rendre(&textuel(brut, locale), &valeurs)?,
        // Un client de messagerie qui ne lit pas le HTML doit tout de même lire
        // quelque chose : le repli est le HTML **dépouillé**, jamais vide.
        None => depouiller(&html),
    };

    Ok(Compose {
        mail: OutgoingMail {
            message_id: rappel.message_id.to_owned(),
            to: rappel.destinataire.email.clone(),
            locale: locale.clone(),
            subject: sujet,
            text: texte,
            html: Some(html),
        },
        template: Some((revision.template_id, revision.version)),
        habille: false,
    })
}

/// La chaîne d'un `platform.i18n_text`, repli sur le français comme
/// `platform.t()`.
fn textuel(valeur: &serde_json::Value, locale: &str) -> String {
    valeur
        .get(locale)
        .and_then(serde_json::Value::as_str)
        .or_else(|| valeur.get("fr").and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .to_owned()
}

fn rendre(gabarit: &str, valeurs: &HashMap<String, String>) -> Result<String> {
    render::rendre(gabarit, valeurs).map_err(|e| ApiError::internal(e.to_string()))
}

/// Le HTML dépouillé de ses balises : un repli texte, pas une conversion.
fn depouiller(html: &str) -> String {
    let mut sortie = String::with_capacity(html.len());
    let mut dans_une_balise = false;
    for caractere in html.chars() {
        match caractere {
            '<' => dans_une_balise = true,
            '>' => {
                dans_une_balise = false;
                sortie.push(' ');
            }
            c if !dans_une_balise => sortie.push(c),
            _ => {}
        }
    }
    sortie.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_repli_texte_garde_les_mots_et_perd_les_balises() {
        assert_eq!(
            depouiller("<p>Bonjour <strong>Awa</strong>,</p><p>À demain.</p>"),
            "Bonjour Awa , À demain."
        );
    }

    #[test]
    fn la_langue_se_replie_sur_le_francais() {
        let valeur = serde_json::json!({ "fr": "Bonjour", "en": "Hello" });
        assert_eq!(textuel(&valeur, "en"), "Hello");
        assert_eq!(textuel(&valeur, "es"), "Bonjour");
    }
}
