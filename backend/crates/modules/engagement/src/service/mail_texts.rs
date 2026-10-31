//! **Les textes types** (01/11) : objet et corps prêts à l'emploi.
//!
//! Gardés exactement comme les gabarits personnalisés, dont ils reprennent les
//! contrôles : `engagement.mailing.send` sur l'édition du texte, avec le
//! périmètre d'administration (règle n° 8) ; un texte commun s'écrit avec la
//! permission globale et se choisit partout.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::mail_text::{MailText, MailTextPayload};
use crate::domain::sanitize;
use crate::repo::mail_texts;
use crate::service::mail_templates::{exiger_lecriture, garder_edition, peut_ecrire, peut_lire};
use crate::state::EngagementState;

/// Un texte visé par son identifiant : hors droit, il n'existe pas.
async fn garder(state: &EngagementState, acteur: Uuid, id: Uuid) -> Result<mail_texts::LigneTexte> {
    match mail_texts::lire(state.pool(), id).await? {
        Some(t) if peut_lire(state, acteur, t.event_id).await? => Ok(t),
        _ => Err(ApiError::not_found()),
    }
}

async fn vers_ecran(
    state: &EngagementState,
    acteur: Uuid,
    t: mail_texts::LigneTexte,
) -> Result<MailText> {
    let can_edit = peut_ecrire(state, acteur, t.event_id).await?;
    Ok(MailText {
        id: t.id,
        name: t.name,
        category: t.category,
        subject: t.subject,
        body_html: t.body_html,
        sort_order: t.sort_order,
        event_id: t.event_id,
        created_by_name: t.created_by_name,
        created_at: t.created_at,
        updated_at: t.updated_at,
        can_edit,
    })
}

/// Les textes utilisables pour une édition : les siens et les communs.
pub async fn lister(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
) -> Result<Vec<MailText>> {
    garder_edition(state, acteur, event_id).await?;
    let mut liste = Vec::new();
    for t in mail_texts::lister(state.pool(), event_id).await? {
        liste.push(vers_ecran(state, acteur, t).await?);
    }
    Ok(liste)
}

pub async fn detail(state: &EngagementState, acteur: Uuid, id: Uuid) -> Result<MailText> {
    let t = garder(state, acteur, id).await?;
    vers_ecran(state, acteur, t).await
}

struct Valide {
    name: String,
    category: Option<String>,
    subject: String,
    body_html: String,
}

/// Bornes de `ck_mail_texts_*`, rendues sur le champ. Les variables inconnues
/// ne bloquent pas l'enregistrement automatique : l'écran les signale, l'envoi
/// les refuse.
fn valider(p: &MailTextPayload) -> Result<Valide> {
    let name = p.name.trim();
    if name.is_empty() {
        return Err(ApiError::validation("Donnez un nom au texte.", "name"));
    }
    if name.chars().count() > 120 {
        return Err(ApiError::validation("120 caractères au plus.", "name"));
    }
    let category = p
        .category
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty());
    if category.is_some_and(|c| c.chars().count() > 60) {
        return Err(ApiError::validation("60 caractères au plus.", "category"));
    }
    let subject = p.subject.trim();
    if subject.chars().count() > 250 {
        return Err(ApiError::validation("250 caractères au plus.", "subject"));
    }
    Ok(Valide {
        name: name.to_owned(),
        category: category.map(str::to_owned),
        subject: subject.to_owned(),
        body_html: sanitize::assainir(&p.body_html),
    })
}

fn valeurs<'a>(v: &'a Valide, p: &MailTextPayload) -> mail_texts::ValeursTexte<'a> {
    mail_texts::ValeursTexte {
        name: &v.name,
        category: v.category.as_deref(),
        subject: &v.subject,
        body_html: &v.body_html,
        sort_order: p.sort_order,
        event_id: p.event_id,
    }
}

pub async fn creer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    p: &MailTextPayload,
) -> Result<MailText> {
    exiger_lecriture(state, acteur, p.event_id).await?;
    let v = valider(p)?;
    let mut tx = state.db().write(ctx).await?;
    let id = mail_texts::creer(&mut tx, acteur, &valeurs(&v, p)).await?;
    tx.commit().await?;
    detail(state, acteur, id).await
}

pub async fn modifier(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
    p: &MailTextPayload,
) -> Result<MailText> {
    let t = garder(state, acteur, id).await?;
    exiger_lecriture(state, acteur, t.event_id).await?;
    if p.event_id != t.event_id {
        exiger_lecriture(state, acteur, p.event_id).await?;
    }
    let v = valider(p)?;
    let mut tx = state.db().write(ctx).await?;
    mail_texts::modifier(&mut tx, id, &valeurs(&v, p)).await?;
    tx.commit().await?;
    detail(state, acteur, id).await
}

pub async fn supprimer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<()> {
    let t = garder(state, acteur, id).await?;
    exiger_lecriture(state, acteur, t.event_id).await?;
    let mut tx = state.db().write(ctx).await?;
    mail_texts::supprimer(&mut tx, id).await?;
    tx.commit().await
}

/// Le texte type nommé par un brouillon : il doit exister et servir l'édition
/// de l'envoi (la sienne, ou commun). Chaîne vide : aucun.
pub(crate) async fn verifier_pour_edition(
    state: &EngagementState,
    brut: &str,
    edition: Uuid,
) -> Result<Option<Uuid>> {
    let brut = brut.trim();
    if brut.is_empty() {
        return Ok(None);
    }
    let refus = || {
        ApiError::validation(
            "Ce texte type n'existe pas ou ne sert pas cette édition.",
            "mail_text_id",
        )
    };
    let id = Uuid::parse_str(brut).map_err(|_| refus())?;
    if sert_ledition(state, id, edition).await? {
        Ok(Some(id))
    } else {
        Err(refus())
    }
}

/// Le texte existe-t-il, et sert-il cette édition (le sien, ou commun) ?
pub(crate) async fn sert_ledition(
    state: &EngagementState,
    id: Uuid,
    edition: Uuid,
) -> Result<bool> {
    Ok(mail_texts::lire(state.pool(), id)
        .await?
        .is_some_and(|t| t.event_id.is_none_or(|e| e == edition)))
}
