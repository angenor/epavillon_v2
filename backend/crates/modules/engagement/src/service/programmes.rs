//! **Les courriels programmés** (04/11) : l'onglet « Programmés » de Courriels
//! et la composition des rappels qui partent.
//!
//! Gardé comme les envois : `engagement.mailing.send` sur l'édition, avec le
//! périmètre d'administration (règle n° 8). Un envoi visé par sa clé se garde
//! sur l'édition de sa cible, jamais sur une édition reçue : hors droit, il
//! n'existe pas.
//!
//! Rien n'est recopié dans les travaux : le texte se lit à l'envoi. Une
//! modification vaut donc pour tout ce qui n'est pas encore parti, et aucun
//! travail ne change d'heure.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::mail::OutgoingMail;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::courriel_direct::GABARIT;
use crate::domain::dossier::nom_du_fuseau;
use crate::domain::gabarits::{pour_apercu, Visuels};
use crate::domain::programmes::{
    self, langue, CleEnvoi, CleType, Contexte, Nature, ScheduledMailBatch, ScheduledMailBatchRow,
    ScheduledMailPreview, ScheduledMailPreviewPayload, ScheduledMailScreen, ScheduledMailText,
    ScheduledMailTexts, ScheduledMailTypeTextPayload, LANGUES,
};
use crate::jobs::send_mailing::apparence;
use crate::repo::programmes::{self as repo, CibleRappel, LigneTexte, Portee};
use crate::service::mailings::garder_edition;
use crate::state::EngagementState;

// -----------------------------------------------------------------------------
// La composition, partagée par les travaux et l'aperçu
// -----------------------------------------------------------------------------

/// Le texte écrit pour ce rappel dans cette langue : celui de l'envoi, sinon
/// celui du type. Nul : personne n'en a écrit.
pub async fn texte_ecrit(
    pool: &PgPool,
    event_id: Uuid,
    cle: &CleEnvoi,
    langue: &str,
) -> Result<Option<ScheduledMailText>> {
    let lignes = repo::textes(pool, event_id, cle, Some(langue)).await?;
    let (envoi, type_) = separer(lignes);
    Ok(programmes::retenir(envoi, type_).map(|(t, _)| t))
}

fn separer(lignes: Vec<LigneTexte>) -> (Option<ScheduledMailText>, Option<ScheduledMailText>) {
    let mut envoi = None;
    let mut type_ = None;
    for l in lignes {
        if l.propre {
            envoi = Some(l.texte);
        } else {
            type_ = Some(l.texte);
        }
    }
    (envoi, type_)
}

/// Un rappel tel qu'il part (ou tel que l'aperçu le montre).
pub struct RappelProgramme<'a> {
    pub nature: Nature,
    pub cible: &'a CibleRappel,
    pub prenom: &'a str,
    pub to: &'a str,
    pub locale: &'a str,
    pub offset_minutes: i64,
    pub texte: &'a ScheduledMailText,
    pub site: &'a str,
    pub visuels: Option<&'a Visuels>,
    pub message_id: &'a str,
    pub surligner: bool,
}

pub fn composer(r: &RappelProgramme<'_>) -> Result<OutgoingMail> {
    let anglais = langue(r.locale) == "en";
    let zone = nom_du_fuseau(r.cible.ville.as_deref(), &r.cible.timezone);
    let lien = programmes::lien(
        r.site,
        r.nature,
        &r.cible.event_slug,
        &r.cible.slug,
        anglais,
    );
    programmes::composer(
        r.texte,
        &Contexte {
            nature: r.nature,
            anglais,
            prenom: r.prenom,
            titre: &r.cible.titre,
            edition: &r.cible.edition,
            debut_local: r.cible.debut_local,
            zone: &zone,
            offset_minutes: r.offset_minutes,
            lien: &lien,
        },
        r.visuels,
        r.to,
        r.locale,
        r.message_id,
        r.surligner,
    )
    .map_err(|e| ApiError::internal(e.to_string()))
}

// -----------------------------------------------------------------------------
// Gardes
// -----------------------------------------------------------------------------

async fn cible_de(pool: &PgPool, cle: &CleEnvoi, locale: &str) -> Result<CibleRappel> {
    let cible = match cle.nature {
        Nature::Activite => repo::cible_activite(pool, cle.cible, locale).await?,
        Nature::Journee => {
            let debut = cle.debut_vise().ok_or_else(ApiError::not_found)?;
            repo::cible_journee(pool, Some(cle.cible), None, debut, locale).await?
        }
    };
    cible.ok_or_else(ApiError::not_found)
}

async fn garder_envoi(
    state: &EngagementState,
    acteur: Uuid,
    cle: &str,
    locale: &str,
) -> Result<(CleEnvoi, CibleRappel)> {
    let cle = CleEnvoi::lire(cle).ok_or_else(ApiError::not_found)?;
    let cible = cible_de(state.pool(), &cle, locale).await?;
    garder_edition(state, acteur, cible.event_id).await?;
    Ok((cle, cible))
}

fn valider(nature: Nature, textes: &ScheduledMailTexts) -> Result<ScheduledMailTexts> {
    for (code, texte) in [("fr", &textes.fr), ("en", &textes.en)] {
        programmes::valider(nature, texte)
            .map_err(|(message, champ)| ApiError::validation(message, format!("{code}.{champ}")))?;
    }
    Ok(ScheduledMailTexts {
        fr: programmes::normaliser(&textes.fr),
        en: programmes::normaliser(&textes.en),
    })
}

// -----------------------------------------------------------------------------
// Lectures
// -----------------------------------------------------------------------------

pub async fn ecran(
    state: &EngagementState,
    acteur: Uuid,
    event_id: Uuid,
    locale: &str,
) -> Result<ScheduledMailScreen> {
    garder_edition(state, acteur, event_id).await?;
    let types = programmes::regrouper(repo::envois(state.pool(), event_id, locale).await?);
    let next = types
        .iter()
        .flat_map(|t| t.batches.iter())
        .min_by_key(|e| e.run_at)
        .cloned();
    Ok(ScheduledMailScreen {
        observed_at: OffsetDateTime::now_utc(),
        total: types.iter().map(|t| t.count).sum(),
        next,
        types,
    })
}

pub async fn envoi(
    state: &EngagementState,
    acteur: Uuid,
    cle: &str,
    locale: &str,
) -> Result<ScheduledMailBatch> {
    let (cle, cible) = garder_envoi(state, acteur, cle, locale).await?;
    lire_envoi(state, &cle, cible.event_id, locale).await
}

async fn lire_envoi(
    state: &EngagementState,
    cle: &CleEnvoi,
    event_id: Uuid,
    locale: &str,
) -> Result<ScheduledMailBatch> {
    let pool = state.pool();
    let lignes = repo::envois(pool, event_id, locale).await?;
    let cle_texte = cle.texte();
    let type_key = cle.type_().texte();
    let type_recipients = lignes
        .iter()
        .filter(|(e, _)| e.type_key == type_key)
        .map(|(e, _)| e.recipients)
        .sum();
    let batch: ScheduledMailBatchRow = lignes
        .into_iter()
        .map(|(e, _)| e)
        .find(|e| e.key == cle_texte)
        .ok_or_else(ApiError::not_found)?;
    let premier = repo::premier_destinataire(pool, cle).await?;

    let mut type_fr = None;
    let mut type_en = None;
    let mut propre_fr = None;
    let mut propre_en = None;
    for l in repo::textes(pool, event_id, cle, None).await? {
        let place = match (l.propre, langue(&l.locale)) {
            (false, "fr") => &mut type_fr,
            (false, _) => &mut type_en,
            (true, "fr") => &mut propre_fr,
            (true, _) => &mut propre_en,
        };
        *place = Some(l.texte);
    }
    let type_text_is_custom = type_fr.is_some() || type_en.is_some();
    let type_text = ScheduledMailTexts {
        fr: type_fr.unwrap_or_else(|| programmes::texte_par_defaut(cle.nature, false)),
        en: type_en.unwrap_or_else(|| programmes::texte_par_defaut(cle.nature, true)),
    };
    let own_text = (propre_fr.is_some() || propre_en.is_some()).then(|| ScheduledMailTexts {
        fr: propre_fr.unwrap_or_else(|| type_text.fr.clone()),
        en: propre_en.unwrap_or_else(|| type_text.en.clone()),
    });

    Ok(ScheduledMailBatch {
        observed_at: OffsetDateTime::now_utc(),
        batch,
        type_recipients,
        type_text,
        type_text_is_custom,
        own_text,
        variables: cle
            .nature
            .variables()
            .iter()
            .map(|v| (*v).to_owned())
            .collect(),
        preview_locale: premier
            .map(|p| langue(&p.locale).to_owned())
            .unwrap_or_else(|| "fr".to_owned()),
    })
}

/// Ce qui partira, pour le premier destinataire, avec le texte en cours
/// d'écriture. N'écrit rien, n'envoie rien.
pub async fn apercu(
    state: &EngagementState,
    acteur: Uuid,
    cle: &str,
    p: &ScheduledMailPreviewPayload,
) -> Result<ScheduledMailPreview> {
    let langue = langue(&p.locale);
    let (cle, cible) = garder_envoi(state, acteur, cle, langue).await?;
    let premier = repo::premier_destinataire(state.pool(), &cle)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let texte = ScheduledMailText {
        subject: p.subject.clone(),
        body: p.body.clone(),
    };
    programmes::valider(cle.nature, &texte)
        .map_err(|(message, champ)| ApiError::validation(message, champ))?;
    let site = &state.config().app_public_url;
    let habit = apparence(state.db(), cible.event_id, GABARIT, None, site).await?;
    let mail = composer(&RappelProgramme {
        nature: cle.nature,
        cible: &cible,
        prenom: &premier.first_name,
        to: &premier.email,
        locale: langue,
        offset_minutes: i64::from(cle.offset_minutes),
        texte: &texte,
        site,
        visuels: habit.visuels.as_ref(),
        message_id: "apercu",
        surligner: true,
    })?;
    Ok(ScheduledMailPreview {
        locale: langue.to_owned(),
        subject: mail.subject,
        html: pour_apercu(&mail.html.unwrap_or_default()),
    })
}

// -----------------------------------------------------------------------------
// Écritures
// -----------------------------------------------------------------------------

/// « Enregistrer pour cet envoi » : ses deux langues.
pub async fn ecrire_envoi(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    cle: &str,
    textes: &ScheduledMailTexts,
) -> Result<ScheduledMailBatch> {
    let (cle, cible) = garder_envoi(state, acteur, cle, &ctx.locale).await?;
    let textes = valider(cle.nature, textes)?;
    let mut tx = state.db().write(ctx).await?;
    for code in LANGUES {
        let texte = if code == "en" { &textes.en } else { &textes.fr };
        repo::ecrire(
            &mut tx,
            cible.event_id,
            Portee::Envoi(&cle),
            code,
            texte,
            acteur,
        )
        .await?;
    }
    tx.commit().await?;
    lire_envoi(state, &cle, cible.event_id, &ctx.locale).await
}

/// « Revenir au texte du type ».
pub async fn effacer_envoi(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    cle: &str,
) -> Result<ScheduledMailBatch> {
    let (cle, cible) = garder_envoi(state, acteur, cle, &ctx.locale).await?;
    let mut tx = state.db().write(ctx).await?;
    repo::effacer_envoi(&mut tx, cible.event_id, &cle).await?;
    tx.commit().await?;
    lire_envoi(state, &cle, cible.event_id, &ctx.locale).await
}

/// « Enregistrer pour le type » : tous les envois de ce type dans l'édition,
/// sauf ceux qui ont leur propre texte.
pub async fn ecrire_type(
    state: &EngagementState,
    ctx: &RequestContext,
    acteur: Uuid,
    cle: &str,
    p: &ScheduledMailTypeTextPayload,
) -> Result<ScheduledMailTexts> {
    let cle = CleType::lire(cle).ok_or_else(ApiError::not_found)?;
    garder_edition(state, acteur, p.event_id).await?;
    let textes = valider(
        cle.nature,
        &ScheduledMailTexts {
            fr: p.fr.clone(),
            en: p.en.clone(),
        },
    )?;
    let mut tx = state.db().write(ctx).await?;
    for code in LANGUES {
        let texte = if code == "en" { &textes.en } else { &textes.fr };
        repo::ecrire(&mut tx, p.event_id, Portee::Type(&cle), code, texte, acteur).await?;
    }
    tx.commit().await?;
    Ok(textes)
}
