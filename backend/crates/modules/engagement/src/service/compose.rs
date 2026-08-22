//! **Du type au courriel** — modèle publié s'il existe, texte de secours sinon.
//!
//! # Un type sans modèle publié part quand même, et le dit
//!
//! Rien ne sème de modèle (écart n° 131). Échouer laisserait **tous** les
//! rappels à terre sur une base neuve ; envoyer sans le dire empêcherait de
//! découvrir qu'un modèle manque. Le module compose donc un texte de secours
//! dérivé du libellé du type, et la trace porte `template_id` **nul** — c'est
//! elle qui dit que personne n'a écrit ce courriel (R27).
//!
//! # Une variable manquante FAIT ÉCHOUER l'envoi, en la nommant
//!
//! Le modèle l'écrit lui-même : *« mieux vaut un job en échec visible qu'un
//! email "Bonjour  ," envoyé à 2 000 personnes »*. Un travail en échec porte son
//! message et se reprend ; un courriel amputé est parti pour toujours.
//!
//! # Le lien mène à un écran du SITE, et cet écran n'existe pas encore
//!
//! Aucune page de séance ne vit dans le front (écart n° 138) : le lien pointe
//! donc la page de l'édition, ancrée sur la séance. Le jour où la page existe,
//! **seule cette fonction change** — c'est la raison d'être de son isolement.

use kernel::error::{ApiError, Result};
use kernel::mail::OutgoingMail;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::domain::offsets;
use crate::domain::render;
use crate::repo::cross::{DestinataireCourriel, SeancePourRappel};
use crate::repo::templates;

/// Ce qui a servi à composer.
#[derive(Debug, Clone)]
pub struct Compose {
    pub mail: OutgoingMail,
    /// Le modèle **et le numéro de révision** réellement servis (FR-089). Nul
    /// quand le texte de secours a servi : personne n'a écrit ce courriel, et
    /// la trace le **dit** plutôt que de laisser croire à un modèle (R27).
    pub template: Option<(uuid::Uuid, i16)>,
}

pub struct RappelACompose<'a> {
    pub message_id: &'a str,
    pub destinataire: &'a DestinataireCourriel,
    pub seance: &'a SeancePourRappel,
    pub offset_minutes: i32,
    pub type_code: &'a str,
    pub template_id: Option<uuid::Uuid>,
    pub app_public_url: &'a str,
}

fn en_anglais(locale: &str) -> bool {
    locale.starts_with("en")
}

/// Les cinq variables que `programme.session.reminder` promet dans
/// `notification_types.expected_variables`. Les nommer ici plutôt que de les
/// deviner est ce qui rend le refus de rendu exploitable.
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
        ("lien_participation".to_owned(), lien(rappel)),
    ])
}

fn lien(rappel: &RappelACompose<'_>) -> String {
    format!(
        "{}/event/{}#{}",
        rappel.app_public_url.trim_end_matches('/'),
        rappel.seance.event_slug,
        rappel.seance.slug
    )
}

/// Compose le rappel dans la langue du destinataire.
pub async fn rappel(pool: &PgPool, rappel: &RappelACompose<'_>) -> Result<Compose> {
    let locale = &rappel.destinataire.locale;
    let valeurs = variables(rappel);

    let Some(revision) =
        templates::revision_servie(pool, rappel.template_id, rappel.type_code).await?
    else {
        tracing::info!(
            type_code = %rappel.type_code,
            "aucune révision publiée : le rappel part avec le texte de secours"
        );
        return Ok(Compose {
            mail: secours(pool, rappel, &valeurs).await?,
            template: None,
        });
    };

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
    })
}

/// **Le texte de secours** — dérivé du libellé du type et des variables.
///
/// Il ne cherche pas à ressembler à un modèle : il porte ce que le destinataire
/// doit savoir, et rien de plus. Son existence est ce qui permet de découvrir
/// qu'un modèle manque plutôt que de constater qu'aucun rappel n'est parti.
async fn secours(
    pool: &PgPool,
    rappel: &RappelACompose<'_>,
    valeurs: &HashMap<String, String>,
) -> Result<OutgoingMail> {
    let locale = &rappel.destinataire.locale;
    let anglais = en_anglais(locale);

    let libelle = templates::type_davis(pool, rappel.type_code, locale)
        .await?
        .map(|t| t.label)
        .unwrap_or_else(|| {
            if anglais {
                "Session reminder".to_owned()
            } else {
                "Rappel de séance".to_owned()
            }
        });

    let vide = String::new();
    let lire = |cle: &str| valeurs.get(cle).unwrap_or(&vide).clone();

    // Le libellé du type est déjà résolu dans la langue du destinataire : le
    // sujet n'a rien à traduire de plus.
    let sujet = format!("{libelle} — {}", lire("titre_session"));

    let texte = if anglais {
        format!(
            "Hello {},\n\n« {} » starts {} — {}.\n\n{}\n\nePavillon — IFDD",
            lire("prenom"),
            lire("titre_session"),
            lire("delai"),
            lire("date_session"),
            lire("lien_participation"),
        )
    } else {
        format!(
            "Bonjour {},\n\n« {} » commence {} — {}.\n\n{}\n\nePavillon — IFDD",
            lire("prenom"),
            lire("titre_session"),
            lire("delai"),
            lire("date_session"),
            lire("lien_participation"),
        )
    };

    Ok(OutgoingMail {
        message_id: rappel.message_id.to_owned(),
        to: rappel.destinataire.email.clone(),
        locale: locale.clone(),
        subject: sujet,
        text: texte,
        html: None,
    })
}

/// La chaîne d'un `platform.i18n_text`, repli sur le français comme
/// `platform.t()`. La résolution se fait ici et non en SQL : le gabarit est déjà
/// en mémoire, et un aller-retour de plus pour choisir une clé d'objet serait
/// gratuit.
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

/// Le HTML dépouillé de ses balises — un repli texte, pas une conversion.
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
