//! **Un fichier se rattache à ce qu'il illustre, et jamais à autre chose.**
//!
//! # Les cinq refus du modèle sont TRADUITS, jamais réimplémentés
//!
//! `media.tg_validate_attachment()` refuse une combinaison non déclarée, un
//! type non accepté, un poids dépassé, une forme non respectée, un objet en
//! quarantaine et un rôle exclusif déjà pourvu. Il les lève par
//! `RAISE EXCEPTION` **sans nom de contrainte**, et trois d'entre eux partagent
//! `integrity_constraint_violation`.
//!
//! Les distinguer par le texte du message serait la faute que B3 a nommée : un
//! message français se périme au premier ajustement du SQL. **Ce service les
//! distingue par ce qu'il a lui-même vérifié en amont** — la règle de la table
//! blanche est lue, le type, le poids et la forme sont comparés, et le refus
//! sort avec son code avant même l'écriture. Ce qui remonte malgré tout du
//! déclencheur sort sous le seul cas restant.
//!
//! Le déclencheur garde le dernier mot : deux poses concurrentes sur un rôle
//! exclusif ne peuvent être départagées que par `ux_attachments_exclusive_role`.
//!
//! # Le texte alternatif d'un usage ne touche pas celui de l'objet
//!
//! La déduplication fait qu'un même fichier sert plusieurs fiches ; le texte
//! pertinent n'y est pas le même. `alt_text_override` vit donc sur le
//! **rattachement**, et `media.attached_image()` résout le repli en base
//! (FR-040). Aucune écriture d'ici ne touche `media.assets`.
//!
//! # Détacher ne détruit rien
//!
//! Un objet peut illustrer deux entités — c'est précisément ce que la
//! déduplication produit. Retirer un rattachement retire **une ligne de
//! liaison**, et la réponse le dit, parce que c'est la question qu'on se pose en
//! la lisant.

use kernel::error::{ApiError, ErrorCode, Result};
use kernel::RequestContext;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::attachment::{
    AttachedMedia, AttachmentAssignment, AttachmentBatch, AttachmentPayload,
};
use crate::domain::rules::{self, AttachableRoleRule};
use crate::repo::assets::ObjetARattacher;
use crate::repo::{assets, attachments};
use crate::service::authz::{self, Porteuse};
use crate::state::MediaState;

// -----------------------------------------------------------------------------
// Lectures
// -----------------------------------------------------------------------------

/// Les règles de la table blanche pour une entité — **avec la forme attendue et
/// sa tolérance**, que le contrat du front ne porte pas encore.
///
/// Sans elles, l'écran ne peut pas annoncer qu'un bandeau se veut panoramique :
/// il l'apprend par le refus, après le téléversement.
pub async fn roles(
    state: &MediaState,
    owner_schema: &str,
    owner_table: &str,
) -> Result<Vec<AttachableRoleRule>> {
    attachments::regles_de(state.pool(), owner_schema, owner_table).await
}

/// Les médias d'une entité. La garde est celle de l'écriture : ce que l'on peut
/// changer, on peut le lire.
pub async fn lister(
    state: &MediaState,
    acteur: Uuid,
    porteuse: Porteuse<'_>,
    role: Option<&str>,
) -> Result<Vec<AttachedMedia>> {
    authz::exiger_le_droit(state.pool(), acteur, porteuse).await?;
    attachments::par_entite(
        state.pool(),
        porteuse.owner_schema,
        porteuse.owner_table,
        porteuse.owner_id,
        role,
    )
    .await
}

// -----------------------------------------------------------------------------
// Poser
// -----------------------------------------------------------------------------

/// Ajoute un objet à un rôle. Sur un rôle **exclusif** déjà pourvu, le refus est
/// explicite : c'est un remplacement qu'il faut demander, et le message le dit.
pub async fn poser(
    state: &MediaState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: &AttachmentPayload,
) -> Result<AttachedMedia> {
    let porteuse = Porteuse {
        owner_schema: &payload.owner_schema,
        owner_table: &payload.owner_table,
        owner_id: payload.owner_id,
    };
    authz::exiger_le_droit(state.pool(), acteur, porteuse).await?;

    let regle = exiger_la_regle(state, porteuse, &payload.role).await?;
    let objet = exiger_lobjet(state, payload.asset_id).await?;
    verifier_la_forme(&regle, &objet, &payload.role)?;

    let mut tx = state.db().write(ctx).await?;

    let ordre = match payload.sort_order {
        Some(ordre) => ordre,
        None => {
            attachments::prochain_ordre(
                &mut tx,
                porteuse.owner_schema,
                porteuse.owner_table,
                porteuse.owner_id,
                &payload.role,
            )
            .await?
        }
    };

    let pose = attachments::poser(
        &mut tx,
        &attachments::NouveauRattachement {
            owner_schema: porteuse.owner_schema,
            owner_table: porteuse.owner_table,
            owner_id: porteuse.owner_id,
            asset_id: payload.asset_id,
            role: &payload.role,
            sort_order: ordre,
            alt_text_override: payload.alt_text_override.clone(),
            created_by: acteur,
        },
    )
    .await;

    let attachment_id = match pose {
        Ok(id) => id,
        Err(erreur) => {
            tx.rollback().await?;
            return Err(traduire(erreur, &payload.role));
        }
    };
    tx.commit().await?;

    attachments::par_entite(
        state.pool(),
        porteuse.owner_schema,
        porteuse.owner_table,
        porteuse.owner_id,
        Some(&payload.role),
    )
    .await?
    .into_iter()
    .find(|m| m.attachment_id == attachment_id)
    .ok_or_else(|| ApiError::internal("rattachement introuvable juste après sa pose"))
}

// -----------------------------------------------------------------------------
// L'écriture de remplacement, en lot
// -----------------------------------------------------------------------------

/// **Une liste d'affectations, appliquées en UNE transaction** (FR-043).
///
/// C'est elle que le formulaire d'édition appelle pour ses trois déclinaisons —
/// l'obligation que B3 avait laissée. Chaque rôle nommé est vidé puis regarni
/// dans l'ordre où ses affectations apparaissent ; un rôle **absent** de la
/// liste n'est pas touché, et une valeur nulle vide le sien **sans toucher aux
/// autres**.
///
/// La transaction unique n'est pas un confort : trois images enregistrées à
/// moitié laisseraient une édition avec un bandeau neuf et une vignette
/// ancienne, sans que rien ne le signale.
pub async fn remplacer(
    state: &MediaState,
    ctx: &RequestContext,
    acteur: Uuid,
    lot: &AttachmentBatch,
) -> Result<Vec<AttachedMedia>> {
    let porteuse = Porteuse {
        owner_schema: &lot.owner_schema,
        owner_table: &lot.owner_table,
        owner_id: lot.owner_id,
    };
    authz::exiger_le_droit(state.pool(), acteur, porteuse).await?;

    // **Tout est vérifié avant d'ouvrir la transaction.** Un refus au troisième
    // rôle après deux écritures serait rendu par le rollback, mais aurait coûté
    // deux poses pour rien — et surtout, la vérification préalable est ce qui
    // permet de NOMMER le refus.
    let mut verifies = Vec::with_capacity(lot.assignments.len());
    for affectation in &lot.assignments {
        let regle = exiger_la_regle(state, porteuse, &affectation.role).await?;
        match affectation.asset_id {
            Some(asset_id) => {
                let objet = exiger_lobjet(state, asset_id).await?;
                verifier_la_forme(&regle, &objet, &affectation.role)?;
                verifies.push((affectation, Some(asset_id)));
            }
            None => verifies.push((affectation, None)),
        }
    }
    refuser_le_trop_plein(&lot.assignments, state, porteuse).await?;

    let mut tx = state.db().write(ctx).await?;

    for role in roles_nommes(&lot.assignments) {
        attachments::vider_role(
            &mut tx,
            porteuse.owner_schema,
            porteuse.owner_table,
            porteuse.owner_id,
            &role,
        )
        .await?;
    }

    let mut rangs: Vec<(String, i16)> = Vec::new();
    for (affectation, asset_id) in verifies {
        let Some(asset_id) = asset_id else { continue };

        let rang = rangs.iter_mut().find(|(r, _)| *r == affectation.role);
        let ordre = match rang {
            Some((_, suivant)) => {
                *suivant += 1;
                *suivant
            }
            None => {
                rangs.push((affectation.role.clone(), 0));
                0
            }
        };

        if let Err(erreur) =
            poser_dans(&mut tx, porteuse, affectation, asset_id, ordre, acteur).await
        {
            tx.rollback().await?;
            return Err(traduire(erreur, &affectation.role));
        }
    }

    tx.commit().await?;

    attachments::par_entite(
        state.pool(),
        porteuse.owner_schema,
        porteuse.owner_table,
        porteuse.owner_id,
        None,
    )
    .await
}

async fn poser_dans(
    conn: &mut PgConnection,
    porteuse: Porteuse<'_>,
    affectation: &AttachmentAssignment,
    asset_id: Uuid,
    sort_order: i16,
    acteur: Uuid,
) -> std::result::Result<Uuid, sqlx::Error> {
    attachments::poser(
        conn,
        &attachments::NouveauRattachement {
            owner_schema: porteuse.owner_schema,
            owner_table: porteuse.owner_table,
            owner_id: porteuse.owner_id,
            asset_id,
            role: &affectation.role,
            sort_order,
            alt_text_override: affectation.alt_text_override.clone(),
            created_by: acteur,
        },
    )
    .await
}

/// Deux objets pour un rôle exclusif dans le **même** lot : le refus vient
/// d'ici, et non du déclencheur. Le laisser passer ferait sortir un
/// `unique_violation` au milieu de la transaction, sur un message qui ne dirait
/// pas que la faute est dans la demande elle-même.
async fn refuser_le_trop_plein(
    affectations: &[AttachmentAssignment],
    state: &MediaState,
    porteuse: Porteuse<'_>,
) -> Result<()> {
    for role in roles_nommes(affectations) {
        let poses = affectations
            .iter()
            .filter(|a| a.role == role && a.asset_id.is_some())
            .count();
        if poses <= 1 {
            continue;
        }
        let regle = exiger_la_regle(state, porteuse, &role).await?;
        if !regle.is_multiple {
            return Err(ApiError::new(ErrorCode::MediaRoleExclusive)
                .field("role")
                .detail(format!(
                    "{poses} objets demandés pour le rôle « {role} », qui n'en accepte qu'un"
                )));
        }
    }
    Ok(())
}

/// Les rôles nommés par le lot, sans doublon et **dans l'ordre d'apparition**.
fn roles_nommes(affectations: &[AttachmentAssignment]) -> Vec<String> {
    let mut roles: Vec<String> = Vec::new();
    for affectation in affectations {
        if !roles.contains(&affectation.role) {
            roles.push(affectation.role.clone());
        }
    }
    roles
}

// -----------------------------------------------------------------------------
// Retirer
// -----------------------------------------------------------------------------

/// Détache. **L'objet stocké demeure** — il peut servir ailleurs, et la
/// déduplication fait qu'il sert souvent ailleurs (FR-041, écart n° 128).
pub async fn detacher(
    state: &MediaState,
    ctx: &RequestContext,
    acteur: Uuid,
    attachment_id: Uuid,
) -> Result<()> {
    let Some(vise) = attachments::par_id(state.pool(), attachment_id).await? else {
        return Err(ApiError::not_found());
    };

    authz::exiger_le_droit(
        state.pool(),
        acteur,
        Porteuse {
            owner_schema: &vise.owner_schema,
            owner_table: &vise.owner_table,
            owner_id: vise.owner_id,
        },
    )
    .await?;

    let mut tx = state.db().write(ctx).await?;
    let retire = attachments::retirer(&mut tx, attachment_id).await?;
    tx.commit().await?;

    if retire {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}

// -----------------------------------------------------------------------------
// Les contrôles, et les refus qu'ils nomment
// -----------------------------------------------------------------------------

async fn exiger_la_regle(
    state: &MediaState,
    porteuse: Porteuse<'_>,
    role: &str,
) -> Result<attachments::Regle> {
    attachments::regle(
        state.pool(),
        porteuse.owner_schema,
        porteuse.owner_table,
        role,
    )
    .await?
    .ok_or_else(|| {
        // **Le refus NOMME l'entité et le rôle**, et sort en 422 : un 500 dirait
        // « panne » là où la demande est simplement hors de la table blanche.
        ApiError::new(ErrorCode::MediaRoleNotDeclared)
            .field("role")
            .detail(format!(
                "le rôle « {role} » n'est pas déclaré pour {}.{}",
                porteuse.owner_schema, porteuse.owner_table
            ))
    })
}

/// L'objet visé. **Un objet supprimé se refuse comme un inexistant** ; un objet
/// en quarantaine, lui, existe — et le dire est utile : c'est une décision, pas
/// une absence.
async fn exiger_lobjet(state: &MediaState, asset_id: Uuid) -> Result<ObjetARattacher> {
    let objet = assets::pour_rattachement(state.pool(), asset_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    if objet.status == "quarantined" {
        return Err(ApiError::new(ErrorCode::MediaAssetNotServable).field("asset_id"));
    }
    Ok(objet)
}

/// Type, poids et cadrage, contre la ligne de la table blanche.
fn verifier_la_forme(
    regle: &attachments::Regle,
    objet: &ObjetARattacher,
    role: &str,
) -> Result<()> {
    if !rules::type_accepte(&objet.mime_type, &regle.allowed_mime_prefixes) {
        return Err(ApiError::new(ErrorCode::MediaMimeNotAllowed)
            .field("file")
            .detail(format!(
                "type « {} » pour le rôle « {role} » ; accepté : {}",
                objet.mime_type,
                regle.allowed_mime_prefixes.join(", ")
            )));
    }

    if !rules::poids_accepte(objet.byte_size, regle.max_byte_size) {
        return Err(ApiError::new(ErrorCode::MediaTooLarge)
            .field("file")
            .detail(format!(
                "{} octets pour le rôle « {role} » ; limite : {}",
                objet.byte_size,
                regle.max_byte_size.unwrap_or_default()
            )));
    }

    if !rules::forme_acceptee(
        objet.width,
        objet.height,
        regle.expected_aspect_ratio,
        regle.aspect_ratio_tolerance,
    ) {
        return Err(refus_de_forme(regle, objet, role));
    }

    Ok(())
}

/// **Le refus de forme cite ses quatre nombres** (FR-037).
///
/// « Les dimensions ne correspondent pas » n'apprend rien à qui doit recadrer :
/// il faut savoir ce qu'on a envoyé, ce qui était attendu, et de combien on est
/// à côté. Le déclencheur les cite lui aussi ; ils sont recalculés ici parce
/// que le refus tombe **avant** l'écriture, et que son message est en français.
fn refus_de_forme(regle: &attachments::Regle, objet: &ObjetARattacher, role: &str) -> ApiError {
    let (Some(largeur), Some(hauteur), Some(attendu)) =
        (objet.width, objet.height, regle.expected_aspect_ratio)
    else {
        return ApiError::new(ErrorCode::MediaAspectRatio).field("file");
    };
    let obtenu = f64::from(largeur) / f64::from(hauteur);

    let message = format!(
        "Les dimensions de cette image ne correspondent pas à la forme attendue \
         du rôle « {role} » : {largeur} × {hauteur} donne un rapport de {obtenu:.4}, \
         quand {attendu:.4} est attendu à {tolerance:.1} % près.",
        tolerance = regle.aspect_ratio_tolerance * 100.0
    );

    ApiError {
        message,
        ..ApiError::new(ErrorCode::MediaAspectRatio)
            .field("file")
            .detail(format!(
                "width={largeur} height={hauteur} ratio={obtenu:.4} expected={attendu:.4} \
                 tolerance={:.3}",
                regle.aspect_ratio_tolerance
            ))
    }
}

/// Ce que le déclencheur refuse malgré les contrôles amont.
///
/// Il n'en reste que **deux** cas : le rôle exclusif déjà pourvu, que seul
/// `ux_attachments_exclusive_role` peut trancher sous concurrence ; et tout le
/// reste, qui sort sur le seul code restant. Reconnaître les autres au texte du
/// message serait la dépendance la plus fragile possible.
fn traduire(erreur: sqlx::Error, role: &str) -> ApiError {
    match kernel::pg_error::sqlstate(&erreur).as_deref() {
        Some("23505") => ApiError::new(ErrorCode::MediaRoleExclusive)
            .field("role")
            .detail(format!("le rôle « {role} » n'accepte qu'un seul objet")),
        Some("23000") | Some("23503") | Some("P0001") => {
            ApiError::new(ErrorCode::MediaRoleNotDeclared)
                .field("role")
                .detail(format!("refus du modèle sur le rôle « {role} » : {erreur}"))
        }
        _ => ApiError::from(erreur),
    }
}
