//! Le jour J d'une activité retenue : qui joindre sur place, les besoins
//! techniques, les publications à relayer.
//!
//! # Qui écrit, et jusqu'à quand
//!
//! L'organisation porteuse (adhésion active, ou la déposante en attente pour
//! son propre dossier) et l'équipe qui décide sur l'édition
//! (`programme.proposal.decide`, dans le périmètre). Seulement sur un dossier
//! retenu. Le point focal et les besoins servent à préparer l'activité : ils
//! se ferment avec l'édition. Les publications se complètent après, le bilan
//! se faisant après.
//!
//! La lecture est celle du reste de la fiche (`perimeter::acces_au_dossier`).

use kernel::auth::Scope;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::day_plan::{
    self, ProposalDayPlan, SocialPost, TechnicalNeeds, POINTS_FOCAUX_MAX, PUBLICATIONS_MAX,
};
use crate::domain::ids::ProposalId;
use crate::domain::{ownership, permissions};
use crate::repo::cross;
use crate::repo::day_plan::{self as repo, NouveauPointFocal};
use crate::service::perimeter;
use crate::state::ProgrammeState;

/// `FocalPointInput`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FocalPointInput {
    #[serde(default)]
    pub speaker_id: Option<Uuid>,
    #[serde(default)]
    pub person_id: Option<Uuid>,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub whatsapp: String,
}

/// `SaveFocalPointsPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SaveFocalPointsPayload {
    pub focal_points: Vec<FocalPointInput>,
}

/// `SaveTechnicalNeedsPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SaveTechnicalNeedsPayload {
    #[serde(default)]
    pub video: bool,
    #[serde(default)]
    pub remote: bool,
    #[serde(default)]
    pub remote_details: String,
    #[serde(default)]
    pub accessibility: bool,
    #[serde(default)]
    pub accessibility_details: String,
    #[serde(default)]
    pub other: String,
}

/// `AddSocialPostPayload`.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddSocialPostPayload {
    pub url: String,
}

/// Ce que l'écriture prépare : avant la fin de l'édition seulement, ou aussi
/// après.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Geste {
    Preparation,
    Bilan,
}

/// Le jour J du dossier, pour qui accède à sa fiche.
pub async fn lire(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
) -> Result<ProposalDayPlan> {
    perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;
    composer(state, dossier).await
}

/// Remplacer la liste des points focaux.
pub async fn enregistrer_points_focaux(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    payload: SaveFocalPointsPayload,
) -> Result<ProposalDayPlan> {
    garde_ecriture(state, acteur, dossier, Geste::Preparation).await?;

    if payload.focal_points.is_empty() {
        return Err(ApiError::validation(
            "Gardez au moins une personne à joindre sur place le jour de l'activité.",
            "focal_points",
        ));
    }
    if payload.focal_points.len() > POINTS_FOCAUX_MAX {
        return Err(ApiError::validation(
            format!("Indiquez {POINTS_FOCAUX_MAX} personnes à joindre au plus."),
            "focal_points",
        ));
    }

    let mut points = Vec::with_capacity(payload.focal_points.len());
    for (rang, saisi) in payload.focal_points.iter().enumerate() {
        points.push(resoudre_point_focal(state, dossier, rang, saisi).await?);
    }
    refuser_les_doublons(&points)?;

    let mut tx = state.db().write(ctx).await?;
    repo::remplacer_points_focaux(&mut tx, dossier, &points).await?;
    tx.commit().await?;

    composer(state, dossier).await
}

/// Déclarer les besoins techniques.
pub async fn enregistrer_besoins(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    payload: SaveTechnicalNeedsPayload,
) -> Result<ProposalDayPlan> {
    garde_ecriture(state, acteur, dossier, Geste::Preparation).await?;

    // Le détail suit sa case : décochée, l'écran le cache, et un texte resté
    // derrière dirait à l'équipe technique un besoin que l'organisation a retiré.
    let besoins = TechnicalNeeds {
        video: payload.video,
        remote: payload.remote,
        remote_details: detail_si(payload.remote, &payload.remote_details),
        accessibility: payload.accessibility,
        accessibility_details: detail_si(payload.accessibility, &payload.accessibility_details),
        other: payload.other.trim().to_owned(),
        updated_at: None,
    };
    borner(&besoins.remote_details, 2000, "remote_details")?;
    borner(
        &besoins.accessibility_details,
        2000,
        "accessibility_details",
    )?;
    borner(&besoins.other, 4000, "other")?;

    let mut tx = state.db().write(ctx).await?;
    repo::ecrire_besoins(&mut tx, dossier, acteur, &besoins).await?;
    tx.commit().await?;

    composer(state, dossier).await
}

/// Ajouter le lien d'une publication.
pub async fn ajouter_publication(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    payload: AddSocialPostPayload,
) -> Result<ProposalDayPlan> {
    garde_ecriture(state, acteur, dossier, Geste::Bilan).await?;

    let url = day_plan::normaliser_url(&payload.url).ok_or_else(|| {
        ApiError::validation(
            "Collez l'adresse complète de la publication, commençant par https://.",
            "url",
        )
    })?;
    if repo::nombre_de_publications(state.pool(), dossier).await? >= PUBLICATIONS_MAX {
        return Err(ApiError::validation(
            format!("Cette activité compte déjà {PUBLICATIONS_MAX} publications : retirez-en une avant d'en ajouter."),
            "url",
        ));
    }

    let mut tx = state.db().write(ctx).await?;
    let ajoutee = repo::ajouter_publication(&mut tx, dossier, acteur, &url).await?;
    tx.commit().await?;

    if !ajoutee {
        return Err(ApiError::with_message(
            ErrorCode::Conflict,
            "Ce lien figure déjà parmi les publications de l'activité.",
        )
        .field("url"));
    }
    composer(state, dossier).await
}

/// Retirer une publication.
pub async fn retirer_publication(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    dossier: ProposalId,
    publication: Uuid,
) -> Result<ProposalDayPlan> {
    garde_ecriture(state, acteur, dossier, Geste::Bilan).await?;

    let mut tx = state.db().write(ctx).await?;
    let retiree = repo::retirer_publication(&mut tx, dossier, publication).await?;
    tx.commit().await?;

    if !retiree {
        return Err(ApiError::not_found());
    }
    composer(state, dossier).await
}

/// Qui présente une pièce : un intervenant de CE dossier, ou personne.
pub async fn verifier_presentateur(
    state: &ProgrammeState,
    dossier: ProposalId,
    speaker_id: Option<Uuid>,
) -> Result<()> {
    let Some(speaker_id) = speaker_id else {
        return Ok(());
    };
    if repo::intervenant(state.pool(), dossier, speaker_id)
        .await?
        .is_none()
    {
        return Err(ApiError::with_message(
            ErrorCode::ProposalUnknownReference,
            "Cet intervenant ne figure pas dans ce dossier.",
        )
        .field("presented_by_speaker_id"));
    }
    Ok(())
}

// -----------------------------------------------------------------------------

async fn composer(state: &ProgrammeState, dossier: ProposalId) -> Result<ProposalDayPlan> {
    let pool = state.pool();

    let mut focal_points = repo::points_focaux(pool, dossier).await?;
    if focal_points.is_empty() {
        focal_points.extend(repo::deposant(pool, dossier).await?);
    }

    let technical_needs = repo::besoins(pool, dossier).await?.unwrap_or_default();

    let social_posts = repo::publications(pool, dossier)
        .await?
        .into_iter()
        .map(|(id, url, created_at)| SocialPost {
            id,
            platform: day_plan::plateforme_de(&url),
            url,
            created_at,
        })
        .collect();

    Ok(ProposalDayPlan {
        focal_points,
        technical_needs,
        social_posts,
        needs_deadline: repo::date_limite_des_besoins(pool, dossier).await?,
    })
}

/// Organisation porteuse ou équipe qui décide, sur un dossier retenu ; le
/// refus d'accès est celui d'un dossier inexistant.
async fn garde_ecriture(
    state: &ProgrammeState,
    acteur: Uuid,
    dossier: ProposalId,
    geste: Geste,
) -> Result<()> {
    let ligne = porteuse_ou_equipe(state, acteur, dossier).await?;

    if ligne.status != "accepted" {
        return Err(ApiError::with_message(
            ErrorCode::ProposalNotEditable,
            "Seule une activité retenue se prépare ici : ce dossier n'est pas retenu.",
        ));
    }
    if geste == Geste::Preparation && ligne.edition_terminee {
        return Err(ApiError::with_message(
            ErrorCode::ProposalNotEditable,
            "Cette édition est terminée : le point focal et les besoins techniques ne se modifient plus.",
        ));
    }
    Ok(())
}

/// Qui écrit sur la fiche : l'organisation porteuse (adhésion active, ou la
/// déposante en attente pour son propre dossier), ou l'équipe qui décide sur
/// l'édition, dans son périmètre. Le refus est celui d'un dossier inexistant.
/// L'état du dossier reste à la garde de chaque geste.
pub(crate) async fn porteuse_ou_equipe(
    state: &ProgrammeState,
    acteur: Uuid,
    dossier: ProposalId,
) -> Result<repo::DossierDuJourJ> {
    let pool = state.pool();
    let ligne = repo::dossier(pool, dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let adhesion = cross::adhesion(pool, ligne.organization_id, acteur).await?;
    let par_lorganisation = ownership::peut_agir_sur(adhesion, ligne.submitted_by == acteur);
    let par_lequipe = !par_lorganisation
        && kernel::auth::has_permission(
            pool,
            acteur,
            permissions::PROPOSAL_DECIDE,
            Scope::Event(ligne.event_id),
        )
        .await?
        && kernel::auth::administered_events(pool, acteur)
            .await?
            .allows(ligne.event_id);
    if !par_lorganisation && !par_lequipe {
        return Err(ApiError::not_found());
    }
    Ok(ligne)
}

/// Un point focal saisi, complété depuis la base. L'intervenant et la personne
/// sont relus : le corps ne désigne pas n'importe qui.
async fn resoudre_point_focal(
    state: &ProgrammeState,
    dossier: ProposalId,
    rang: usize,
    saisi: &FocalPointInput,
) -> Result<NouveauPointFocal> {
    let champ = |nom: &str| format!("focal_points[{rang}].{nom}");
    let nom_saisi = saisi.full_name.trim();
    let phone = saisi.phone.trim().to_owned();
    let whatsapp = saisi.whatsapp.trim().to_owned();

    let (speaker_id, person_id, nom_connu, phone_connu) = if let Some(speaker_id) = saisi.speaker_id
    {
        let intervenant = repo::intervenant(state.pool(), dossier, speaker_id)
            .await?
            .ok_or_else(|| {
                ApiError::with_message(
                    ErrorCode::ProposalUnknownReference,
                    "Cet intervenant ne figure pas dans ce dossier.",
                )
                .field(champ("speaker_id"))
            })?;
        (
            Some(speaker_id),
            intervenant.person_id,
            Some(intervenant.nom),
            intervenant.phone,
        )
    } else if let Some(person_id) = saisi.person_id {
        let (nom, tel) = repo::personne_connue(state.pool(), dossier, person_id)
            .await?
            .ok_or_else(|| {
                ApiError::with_message(
                    ErrorCode::ProposalUnknownReference,
                    "Cette personne n'est ni le déposant, ni le contact, ni un membre de l'organisation.",
                )
                .field(champ("person_id"))
            })?;
        (None, Some(person_id), Some(nom), tel)
    } else {
        (None, None, None, None)
    };

    let full_name = match (nom_saisi, nom_connu) {
        ("", Some(connu)) => connu,
        ("", None) => {
            return Err(ApiError::validation(
                "Indiquez le nom de la personne à joindre sur place.",
                champ("full_name"),
            ))
        }
        (saisi, _) => saisi.to_owned(),
    };
    borner(&full_name, 200, &champ("full_name"))?;

    // Un numéro laissé vide reprend celui de la fiche de la personne.
    let phone = if phone.is_empty() {
        phone_connu.unwrap_or_default().trim().to_owned()
    } else {
        phone
    };
    borner(&phone, 50, &champ("phone"))?;
    borner(&whatsapp, 50, &champ("whatsapp"))?;

    Ok(NouveauPointFocal {
        speaker_id,
        person_id,
        full_name,
        phone,
        whatsapp,
    })
}

fn refuser_les_doublons(points: &[NouveauPointFocal]) -> Result<()> {
    for (rang, point) in points.iter().enumerate() {
        let deja = points[..rang].iter().any(|avant| {
            (point.speaker_id.is_some() && avant.speaker_id == point.speaker_id)
                || (point.person_id.is_some() && avant.person_id == point.person_id)
        });
        if deja {
            return Err(ApiError::validation(
                format!(
                    "{} figure deux fois parmi les personnes à joindre.",
                    point.full_name
                ),
                format!("focal_points[{rang}]"),
            ));
        }
    }
    Ok(())
}

fn detail_si(coche: bool, texte: &str) -> String {
    if coche {
        texte.trim().to_owned()
    } else {
        String::new()
    }
}

fn borner(texte: &str, max: usize, champ: &str) -> Result<()> {
    if texte.chars().count() > max {
        return Err(ApiError::validation(
            format!("Ce texte dépasse {max} caractères."),
            champ,
        ));
    }
    Ok(())
}
