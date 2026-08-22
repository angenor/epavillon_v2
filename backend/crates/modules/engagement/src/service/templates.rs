//! **Une faute d'orthographe se corrige sans redéploiement, dans les deux
//! langues.**
//!
//! # Le corps est assaini À L'ÉCRITURE, jamais à l'affichage
//!
//! Un contenu stocké propre se rend partout ; un contenu filtré à l'affichage
//! doit l'être dans chaque écran, chaque courriel et chaque export — et le
//! premier oubli est une injection. C'est la règle de B4, réemployée avec une
//! **liste blanche différente** : un gabarit de courriel a besoin de tableaux et
//! de styles en ligne, les clients de messagerie ignorant les feuilles de style.
//!
//! **Le piège est le lien qui porte une variable** : `href="{{lien}}"` est une
//! URL relative aux yeux d'un analyseur, et la normaliser détruirait la
//! variable — un lien mort que seule la réception révèle. La politique est
//! réglée sur le laisser-passer, et un test le prouve (R26).
//!
//! # Écrire et publier sont deux gestes
//!
//! Une révision écrite n'est **pas** servie. Sans cette séparation, enregistrer
//! une correction à moitié faite l'enverrait à deux mille personnes. Publier
//! fait avancer un pointeur ; **republier une révision antérieure est le retour
//! arrière**, et rien n'est jamais effacé (FR-081).
//!
//! # La publication refuse une variable que le type ne promet pas, EN LA NOMMANT
//!
//! `notification_types.expected_variables` est le contrat de l'émetteur.
//! Un gabarit qui cite autre chose partirait avec un trou — « Bonjour  , » — et
//! le trou ne se verrait qu'à la réception. Le refus arrive donc à la
//! publication, pas à l'envoi : à l'envoi, il serait trop tard pour corriger
//! sans que personne n'ait rien reçu (FR-083).

use kernel::auth::{has_permission, Scope};
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::RequestContext;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::render;
use crate::domain::sanitize;
use crate::domain::template::{
    MessageTemplateRow, RenderedMail, TemplateDetail, TemplateVersion, TemplateVersionPayload,
};
use crate::repo::templates;
use crate::state::EngagementState;

/// La permission qui gouverne les cinq routes. Testée **par permission**, jamais
/// par nom de rôle. Sa portée est globale : un modèle de message sert toutes les
/// éditions à la fois, et le borner à l'une d'elles n'aurait pas de sens.
const PERMISSION: &str = "engagement.template.manage";

/// Les deux langues servies. Le repli sur le français est celui de
/// `platform.t()`, et il vaut pour l'aperçu comme pour l'envoi (FR-087).
pub const LANGUES: [&str; 2] = ["fr", "en"];

// -----------------------------------------------------------------------------
// Lectures
// -----------------------------------------------------------------------------

pub async fn lister(state: &EngagementState, acteur: Uuid) -> Result<Vec<MessageTemplateRow>> {
    exiger_le_droit(state, acteur).await?;
    templates::lister(state.pool()).await
}

/// Le détail d'un modèle : ses révisions, celle qui est servie, et **les
/// variables que son type promet**.
///
/// Sans cette dernière liste, l'écran ne pourrait annoncer les variables
/// disponibles qu'en les devinant — et un administrateur découvrirait le refus
/// à la publication, après avoir écrit son gabarit.
pub async fn detail(
    state: &EngagementState,
    acteur: Uuid,
    template_id: Uuid,
) -> Result<TemplateDetail> {
    exiger_le_droit(state, acteur).await?;
    composer_le_detail(state, template_id).await
}

async fn composer_le_detail(state: &EngagementState, template_id: Uuid) -> Result<TemplateDetail> {
    let Some(template) = templates::par_id(state.pool(), template_id).await? else {
        return Err(ApiError::not_found());
    };
    let versions = templates::revisions(state.pool(), template_id).await?;

    let current = template
        .current_version
        .and_then(|servie| versions.iter().find(|v| v.version == servie).cloned())
        // Une révision dépubliée à la main ne doit pas se faire passer pour
        // servie : le pointeur du modèle et l'instant de publication doivent
        // dire la même chose.
        .filter(|v| v.published_at.is_some());

    let promised_variables = match template.type_code.as_deref() {
        Some(code) => templates::variables_promises(state.pool(), code)
            .await?
            .unwrap_or_default(),
        None => Vec::new(),
    };

    Ok(TemplateDetail {
        template,
        versions,
        current,
        promised_variables,
    })
}

// -----------------------------------------------------------------------------
// Écriture d'une révision
// -----------------------------------------------------------------------------

/// Écrit une révision, **assainie**, et non publiée.
pub async fn ecrire_revision(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    template_id: Uuid,
    payload: &TemplateVersionPayload,
) -> Result<TemplateVersion> {
    exiger_le_droit(state, acteur).await?;
    if templates::par_id(state.pool(), template_id)
        .await?
        .is_none()
    {
        return Err(ApiError::not_found());
    }

    let subject = exiger_un_texte(&payload.subject, "subject")?;
    let body_html = assainir_par_langue(exiger_un_texte(&payload.body_html, "body_html")?);
    let body_text = match payload.body_text.as_ref() {
        Some(brut) => Some(exiger_un_texte(brut, "body_text")?),
        None => None,
    };

    let valeurs = templates::ValeursDeRevision {
        // Les variables citées sont **relevées**, jamais déclarées : une liste
        // saisie à la main divergerait du gabarit au premier ajustement, et
        // c'est elle qui sert au contrôle de publication.
        variables: citees(&subject, &body_html, body_text.as_ref())
            .into_iter()
            .collect(),
        subject,
        body_html,
        body_text,
        created_by: acteur,
    };

    let mut tx = state.db().write(ctx).await?;
    let numero = templates::prochain_numero(&mut tx, template_id).await?;
    let revision = match templates::ecrire_revision(&mut tx, template_id, numero, &valeurs).await {
        Ok(revision) => revision,
        Err(erreur) => {
            tx.rollback().await?;
            return Err(ApiError::from(erreur));
        }
    };
    tx.commit().await?;

    Ok(revision)
}

// -----------------------------------------------------------------------------
// Publication, et retour arrière
// -----------------------------------------------------------------------------

/// Publie une révision — ou **republie une révision antérieure**, ce qui est le
/// retour arrière (FR-081).
pub async fn publier(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    template_id: Uuid,
    version: i16,
) -> Result<TemplateDetail> {
    exiger_le_droit(state, acteur).await?;

    let Some(template) = templates::par_id(state.pool(), template_id).await? else {
        return Err(ApiError::not_found());
    };
    let revisions = templates::revisions(state.pool(), template_id).await?;
    let Some(revision) = revisions.iter().find(|v| v.version == version) else {
        return Err(ApiError::new(ErrorCode::EngagementTemplateVersionUnknown));
    };

    verifier_les_variables(state, template.type_code.as_deref(), revision).await?;

    let mut tx = state.db().write(ctx).await?;
    let publiee = templates::publier(&mut tx, template_id, version).await?;
    if !publiee {
        tx.rollback().await?;
        return Err(ApiError::new(ErrorCode::EngagementTemplateVersionUnknown));
    }
    tx.commit().await?;

    composer_le_detail(state, template_id).await
}

/// **Le refus nomme la variable**, et la première dans l'ordre alphabétique — un
/// ordre stable rend le message reproductible d'un essai à l'autre, ce qu'un
/// ordre d'apparition ne garantit pas quand le gabarit change.
async fn verifier_les_variables(
    state: &EngagementState,
    type_code: Option<&str>,
    revision: &TemplateVersion,
) -> Result<()> {
    // Un modèle qui ne sert aucun type ne promet rien, et rien ne peut donc lui
    // être reproché : c'est le cas des campagnes d'infolettre, hors périmètre.
    let Some(code) = type_code else {
        return Ok(());
    };
    let Some(promises) = templates::variables_promises(state.pool(), code).await? else {
        return Err(ApiError::new(ErrorCode::EngagementNotificationTypeUnknown).field("type_code"));
    };

    let citees = citees(
        &revision.subject,
        &revision.body_html,
        revision.body_text.as_ref(),
    );
    if let Some(inconnue) = citees.iter().find(|nom| !promises.contains(nom)) {
        return Err(ApiError::new(ErrorCode::EngagementTemplateVariableUnknown)
            .field("body_html")
            .detail(format!(
                "« {inconnue} » n'est pas fournie par « {code} » ; ce type promet : {}",
                promises.join(", ")
            )));
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Aperçu — il n'envoie rien
// -----------------------------------------------------------------------------

/// Ce qu'un aperçu demande.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PreviewPayload {
    /// Absente : la révision servie, ou la plus récente si aucune n'est publiée
    /// — un brouillon doit se relire avant d'être publié, c'est même son objet.
    pub version: Option<i16>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

/// L'aperçu, **dans les deux langues et sans rien envoyer**.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Preview {
    pub fr: RenderedMail,
    pub en: RenderedMail,
}

/// Rend un aperçu.
///
/// **Une variable absente ne fait pas échouer l'aperçu**, contrairement à
/// l'envoi : elle prend une valeur d'exemple visible. Un aperçu sert à regarder
/// une mise en page, et refuser de la montrer parce qu'un exemple manque le
/// rendrait inutile — alors qu'à l'envoi, un trou part chez deux mille
/// personnes et l'échec est la bonne réponse (FR-084, FR-085).
pub async fn apercu(
    state: &EngagementState,
    acteur: Uuid,
    template_id: Uuid,
    payload: &PreviewPayload,
) -> Result<Preview> {
    exiger_le_droit(state, acteur).await?;

    let detail = composer_le_detail(state, template_id).await?;
    let revision = match payload.version {
        Some(numero) => detail.versions.iter().find(|v| v.version == numero),
        None => detail.current.as_ref().or_else(|| detail.versions.first()),
    }
    .ok_or_else(|| ApiError::new(ErrorCode::EngagementTemplateVersionUnknown))?;

    let valeurs = valeurs_dexemple(revision, &detail.promised_variables, &payload.variables);

    Ok(Preview {
        fr: rendre_dans(revision, "fr", &valeurs)?,
        en: rendre_dans(revision, "en", &valeurs)?,
    })
}

/// Les valeurs fournies, complétées par un exemple **visible** pour tout ce qui
/// manque. `« prenom »` se repère à l'œil dans un aperçu ; une chaîne vide se
/// confondrait avec une mise en page ratée.
fn valeurs_dexemple(
    revision: &TemplateVersion,
    promises: &[String],
    fournies: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut valeurs = fournies.clone();
    let citees = citees(
        &revision.subject,
        &revision.body_html,
        revision.body_text.as_ref(),
    );
    for nom in citees.iter().chain(promises.iter()) {
        valeurs
            .entry(nom.clone())
            .or_insert_with(|| format!("« {nom} »"));
    }
    valeurs
}

fn rendre_dans(
    revision: &TemplateVersion,
    locale: &str,
    valeurs: &HashMap<String, String>,
) -> Result<RenderedMail> {
    let html = rendre(&textuel(&revision.body_html, locale), valeurs)?;
    let texte = match revision.body_text.as_ref() {
        Some(brut) => rendre(&textuel(brut, locale), valeurs)?,
        None => String::new(),
    };

    Ok(RenderedMail {
        subject: rendre(&textuel(&revision.subject, locale), valeurs)?,
        body_html: html,
        body_text: texte,
    })
}

// -----------------------------------------------------------------------------
// Outils communs
// -----------------------------------------------------------------------------

/// Les variables citées par les trois champs, toutes langues confondues.
///
/// Toutes langues : un gabarit anglais qui citerait une variable de plus doit
/// être refusé comme le français. Les relever langue par langue laisserait
/// passer celle qu'on ne relit pas.
fn citees(
    subject: &serde_json::Value,
    body_html: &serde_json::Value,
    body_text: Option<&serde_json::Value>,
) -> BTreeSet<String> {
    let mut noms = BTreeSet::new();
    for valeur in [Some(subject), Some(body_html), body_text]
        .into_iter()
        .flatten()
    {
        if let Some(objet) = valeur.as_object() {
            for texte in objet.values().filter_map(serde_json::Value::as_str) {
                noms.extend(render::variables_citees(texte));
            }
        }
    }
    noms
}

/// **Assainit chaque langue.** Assainir la valeur entière comme un texte
/// détruirait la structure JSON ; n'assainir que le français laisserait passer
/// l'anglais.
fn assainir_par_langue(valeur: serde_json::Value) -> serde_json::Value {
    match valeur {
        serde_json::Value::Object(objet) => serde_json::Value::Object(
            objet
                .into_iter()
                .map(|(langue, texte)| match texte.as_str() {
                    Some(brut) => (langue, serde_json::Value::String(sanitize::assainir(brut))),
                    None => (langue, texte),
                })
                .collect(),
        ),
        autre => autre,
    }
}

/// Un `platform.i18n_text` porte au moins le français : le type le vérifie déjà
/// en base, et le vérifier ici pose le refus **sur le champ**.
fn exiger_un_texte(valeur: &serde_json::Value, champ: &str) -> Result<serde_json::Value> {
    let a_du_texte = valeur.as_object().is_some_and(|o| {
        o.values()
            .any(|v| v.as_str().is_some_and(|t| !t.is_empty()))
    });

    if a_du_texte {
        Ok(valeur.clone())
    } else {
        Err(ApiError::new(ErrorCode::ValidationFailed)
            .field(champ)
            .detail("un texte multilingue est attendu, portant au moins le français"))
    }
}

/// Repli sur le français, comme `platform.t()` (FR-087).
fn textuel(valeur: &serde_json::Value, locale: &str) -> String {
    valeur
        .get(locale)
        .and_then(serde_json::Value::as_str)
        .filter(|texte| !texte.is_empty())
        .or_else(|| valeur.get("fr").and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .to_owned()
}

fn rendre(gabarit: &str, valeurs: &HashMap<String, String>) -> Result<String> {
    render::rendre(gabarit, valeurs).map_err(|e| {
        ApiError::new(ErrorCode::EngagementTemplateVariableUnknown)
            .field("variables")
            .detail(e.to_string())
    })
}

/// La permission, sur la portée **globale**.
async fn exiger_le_droit(state: &EngagementState, acteur: Uuid) -> Result<()> {
    if has_permission(state.pool(), acteur, PERMISSION, Scope::Global).await? {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}
