//! **Les gabarits de courriel personnalisés** (29/10).
//!
//! Gardés comme les envois : `engagement.mailing.send` sur l'édition du
//! gabarit, avec le périmètre d'administration (règle n° 8). Un gabarit commun à
//! toutes les éditions s'écrit avec la permission globale, et se lit avec la
//! permission sur une édition quelconque : il se choisit partout.

use kernel::auth::{administered_events, has_permission, has_permission_anywhere, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use uuid::Uuid;

use crate::domain::gabarits::{self, MiseEnPage, GABARITS};
use crate::domain::mail_template::{MailTemplate, MailTemplatePayload, MailTemplatePreview};
use crate::jobs::send_mailing;
use crate::repo::{cross, mail_templates};
use crate::state::EngagementState;

const PERMISSION: &str = "engagement.mailing.send";

/// Peut-on écrire un gabarit de cette édition (ou commun, `None`) ?
async fn peut_ecrire(state: &EngagementState, acteur: Uuid, edition: Option<Uuid>) -> Result<bool> {
    Ok(match edition {
        Some(e) => {
            administered_events(state.pool(), acteur).await?.allows(e)
                && has_permission(state.pool(), acteur, PERMISSION, Scope::Event(e)).await?
        }
        None => has_permission(state.pool(), acteur, PERMISSION, Scope::Global).await?,
    })
}

/// Peut-on le lire (le choisir) ?
async fn peut_lire(state: &EngagementState, acteur: Uuid, edition: Option<Uuid>) -> Result<bool> {
    match edition {
        Some(_) => peut_ecrire(state, acteur, edition).await,
        None => has_permission_anywhere(state.pool(), acteur, PERMISSION).await,
    }
}

async fn garder_edition(state: &EngagementState, acteur: Uuid, event_id: Uuid) -> Result<()> {
    if !cross::edition_existe(state.pool(), event_id).await? {
        return Err(ApiError::not_found());
    }
    if !administered_events(state.pool(), acteur)
        .await?
        .allows(event_id)
    {
        return Err(ApiError::not_found());
    }
    if !has_permission(state.pool(), acteur, PERMISSION, Scope::Event(event_id)).await? {
        return Err(ApiError::forbidden());
    }
    Ok(())
}

/// Un gabarit visé par son identifiant : hors droit, il n'existe pas.
async fn garder(
    state: &EngagementState,
    acteur: Uuid,
    id: Uuid,
) -> Result<mail_templates::LigneGabarit> {
    match mail_templates::lire(state.pool(), id).await? {
        Some(g) if peut_lire(state, acteur, g.event_id).await? => Ok(g),
        _ => Err(ApiError::not_found()),
    }
}

async fn vers_ecran(
    state: &EngagementState,
    acteur: Uuid,
    g: mail_templates::LigneGabarit,
) -> Result<MailTemplate> {
    let can_edit = peut_ecrire(state, acteur, g.event_id).await?;
    Ok(MailTemplate {
        id: g.id,
        name: g.name,
        layout: g.layout,
        title: g.title,
        subtitle: g.subtitle,
        gradient: g.gradient,
        event_id: g.event_id,
        preselect_for_submitters: g.preselect_for_submitters,
        created_by_name: g.created_by_name,
        created_at: g.created_at,
        updated_at: g.updated_at,
        background: g.background,
        logo: g.logo,
        can_edit,
    })
}

/// Les gabarits utilisables pour une édition : les siens, puis les communs.
pub async fn lister(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
) -> Result<Vec<MailTemplate>> {
    garder_edition(state, acteur, event_id).await?;
    let mut liste = Vec::new();
    for g in mail_templates::lister(state.pool(), event_id).await? {
        liste.push(vers_ecran(state, acteur, g).await?);
    }
    Ok(liste)
}

pub async fn detail(state: &EngagementState, acteur: Uuid, id: Uuid) -> Result<MailTemplate> {
    let g = garder(state, acteur, id).await?;
    vers_ecran(state, acteur, g).await
}

/// Texte facultatif : vide, il n'y en a pas ; borné par `ck_mail_templates_texts`.
fn texte(valeur: &Option<String>, champ: &str) -> Result<Option<String>> {
    let t = valeur.as_deref().map(str::trim).unwrap_or_default();
    if t.chars().count() > 200 {
        return Err(ApiError::validation("200 caractères au plus.", champ));
    }
    Ok((!t.is_empty()).then(|| t.to_owned()))
}

struct Valide {
    name: String,
    title: Option<String>,
    subtitle: Option<String>,
}

async fn valider(
    state: &EngagementState,
    p: &MailTemplatePayload,
    sauf: Option<Uuid>,
) -> Result<Valide> {
    let name = p.name.trim();
    if name.is_empty() {
        return Err(ApiError::validation("Donnez un nom au gabarit.", "name"));
    }
    if name.chars().count() > 120 {
        return Err(ApiError::validation("120 caractères au plus.", "name"));
    }
    if !GABARITS.contains(&p.layout.as_str()) {
        return Err(ApiError::validation(
            format!("Mise en page inconnue : « {} ».", p.layout),
            "layout",
        ));
    }
    if p.preselect_for_submitters {
        let Some(edition) = p.event_id else {
            return Err(ApiError::validation(
                "Rattachez le gabarit à un événement pour le présélectionner.",
                "preselect_for_submitters",
            ));
        };
        if mail_templates::autre_preselectionne(state.pool(), edition, sauf).await? {
            return Err(ApiError::validation(
                "Un autre gabarit est déjà présélectionné pour les déposants de cet événement.",
                "preselect_for_submitters",
            ));
        }
    }
    Ok(Valide {
        name: name.to_owned(),
        title: texte(&p.title, "title")?,
        subtitle: texte(&p.subtitle, "subtitle")?,
    })
}

fn valeurs<'a>(v: &'a Valide, p: &'a MailTemplatePayload) -> mail_templates::ValeursGabarit<'a> {
    mail_templates::ValeursGabarit {
        name: &v.name,
        layout: &p.layout,
        title: v.title.as_deref(),
        subtitle: v.subtitle.as_deref(),
        gradient: p.gradient,
        event_id: p.event_id,
        preselect_for_submitters: p.preselect_for_submitters,
    }
}

/// Exige le droit d'écrire sur cette édition : 404 hors périmètre, 403 sans la
/// permission (ou sans la permission globale pour un gabarit commun).
async fn exiger_lecriture(
    state: &EngagementState,
    acteur: Uuid,
    edition: Option<Uuid>,
) -> Result<()> {
    if let Some(e) = edition {
        garder_edition(state, acteur, e).await?;
    }
    if !peut_ecrire(state, acteur, edition).await? {
        return Err(ApiError::forbidden());
    }
    Ok(())
}

pub async fn creer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    p: &MailTemplatePayload,
) -> Result<MailTemplate> {
    exiger_lecriture(state, acteur, p.event_id).await?;
    let v = valider(state, p, None).await?;
    let mut tx = state.db().write(ctx).await?;
    let id = mail_templates::creer(&mut tx, acteur, &valeurs(&v, p)).await?;
    tx.commit().await?;
    detail(state, acteur, id).await
}

pub async fn modifier(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
    p: &MailTemplatePayload,
) -> Result<MailTemplate> {
    let g = garder(state, acteur, id).await?;
    exiger_lecriture(state, acteur, g.event_id).await?;
    if p.event_id != g.event_id {
        exiger_lecriture(state, acteur, p.event_id).await?;
    }
    let v = valider(state, p, Some(id)).await?;
    let mut tx = state.db().write(ctx).await?;
    mail_templates::modifier(&mut tx, id, &valeurs(&v, p)).await?;
    tx.commit().await?;
    detail(state, acteur, id).await
}

pub async fn supprimer(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
) -> Result<()> {
    let g = garder(state, acteur, id).await?;
    exiger_lecriture(state, acteur, g.event_id).await?;
    let mut tx = state.db().write(ctx).await?;
    mail_templates::supprimer(&mut tx, id).await?;
    tx.commit().await
}

/// Le corps d'exemple de l'aperçu d'un gabarit.
const EXEMPLE: &str = "<p>Bonjour Prénom,</p><p>Voici l'aperçu de votre gabarit : \
le texte du courriel prend place ici, entre l'en-tête et le pied de page.</p>\
<p>Bien cordialement,</p>";

/// L'aperçu, rendu par la fonction du worker sur un texte d'exemple, avec les
/// visuels de l'édition du gabarit (ou de celle demandée, pour un gabarit
/// commun) sous les siens.
pub async fn apercu(
    state: &EngagementState,
    acteur: Uuid,
    id: Uuid,
    event_id: Option<Uuid>,
) -> Result<MailTemplatePreview> {
    let g = garder(state, acteur, id).await?;
    let edition = match g.event_id.or(event_id) {
        Some(e) => e,
        None => {
            return Err(ApiError::validation(
                "Choisissez l'édition de l'aperçu.",
                "event_id",
            ))
        }
    };
    if g.event_id.is_none() {
        garder_edition(state, acteur, edition).await?;
    }
    let site = &state.config().app_public_url;
    let apparence = send_mailing::apparence(state.db(), edition, &g.layout, Some(&g), site).await?;
    let sujet = g.name.clone();
    let lien = format!("{}/", site.trim_end_matches('/'));
    let html = gabarits::mettre_en_page(
        EXEMPLE,
        &MiseEnPage {
            gabarit: &apparence.gabarit,
            sujet: &sujet,
            action: Some(("Bouton d'action", &lien)),
            repondre: None,
            visuels: apparence.visuels.as_ref(),
        },
    );
    Ok(MailTemplatePreview {
        subject: sujet,
        html: gabarits::pour_apercu(&html),
    })
}
