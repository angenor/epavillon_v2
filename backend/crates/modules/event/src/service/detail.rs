//! **Tout l'écran de détail en une réponse** — l'édition, ses six onglets et les
//! listes de référence dont ses formulaires ont besoin (FR-022, FR-023).
//!
//! ## Une connexion, une transaction en lecture seule
//!
//! Les douze lectures sont **séquentielles sur une seule connexion**, dans une
//! transaction `REPEATABLE READ READ ONLY` (research.md § R3).
//!
//! **Pas un éventail concurrent** : douze requêtes lancées de front prennent
//! douze connexions du pool pour un seul écran d'administration. B2 a payé
//! exactement cette monnaie — cent créations concurrentes sortaient en « service
//! indisponible » parce qu'une transaction perdante retenait deux connexions
//! avant d'être rendue. Un back-office ouvert à la main n'a aucun besoin de
//! gagner cinquante millisecondes au prix d'un pool saturé.
//!
//! **Et pas douze lectures libres** : la réponse mêle des décomptes venus de
//! trois schémas. Sans instantané commun, l'onglet des journées pourrait
//! annoncer trois séances quand celui des salles en compte quatre, pour la même
//! édition et au même instant. `REPEATABLE READ` rend les lectures cohérentes
//! entre elles, et `READ ONLY` le dit à PostgreSQL.
//!
//! ## Ce que ce fichier ne décide pas
//!
//! Le périmètre est vérifié **avant** d'arriver ici, par le garde d'ascendance
//! (`service::edition_dans_le_perimetre`) : une édition introuvable et une
//! édition hors périmètre y reçoivent déjà le même refus. Ici, l'absence rend
//! simplement `None`.

use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::domain::detail::{
    CommitteeCandidate, EditionCommitteeMember, EditionDetail, EditionPeriod, ThemeBadge,
};
use crate::domain::ids::{CallId, EventId};
use crate::domain::permissions::{EVENT_MANAGE, SESSION_SCHEDULE};
use crate::repo::cross::REVIEW_WRITE;
use crate::repo::{calls, channels, committee, criteria, cross, days, editions, tracks, venues};

use super::edition_read;

/// La composition entière. `None` quand l'édition n'existe pas.
pub async fn composer(pool: &PgPool, event_id: EventId) -> Result<Option<EditionDetail>> {
    let mut tx = pool.begin().await?;

    // `SET TRANSACTION` doit précéder toute lecture : posée après la première
    // requête, PostgreSQL la refuse. C'est donc la première chose que fait cette
    // transaction, et la seule qui ne soit pas une lecture.
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;

    let detail = lire(&mut tx, event_id).await?;

    // Une transaction en lecture seule n'a rien à valider ; la rendre au pool
    // sans attendre la fin de la portée est plus clair qu'un `drop` implicite.
    tx.rollback().await?;

    Ok(detail)
}

async fn lire(conn: &mut PgConnection, event_id: EventId) -> Result<Option<EditionDetail>> {
    let Some(base) = editions::ligne(&mut *conn, event_id).await? else {
        return Ok(None);
    };
    let decomptes = cross::decomptes(&mut *conn, event_id).await?;
    let edition = edition_read::composer(base, decomptes);

    let Some((description, highlights)) = editions::textes(&mut *conn, event_id).await? else {
        return Ok(None);
    };

    let period = periode(&mut *conn, event_id).await?;
    let images = cross::images_de_l_edition(&mut *conn, event_id).await?;

    let days = journees(&mut *conn, event_id, &period).await?;
    let tracks = fils(&mut *conn, event_id).await?;
    let venues = lieux(&mut *conn, event_id).await?;
    let channels = canaux(&mut *conn, event_id).await?;

    let call = appel(&mut *conn, event_id).await?;
    let committee = match &call {
        Some(c) => comite(&mut *conn, CallId::from(c.id), event_id).await?,
        None => Vec::new(),
    };

    let personne_exclue = HashSet::new();
    let curators = assignables(
        &mut *conn,
        event_id,
        &[EVENT_MANAGE, SESSION_SCHEDULE],
        &personne_exclue,
    )
    .await?;
    // **Les membres actuels sont écartés** : proposer d'ajouter quelqu'un qui
    // siège déjà ferait un doublon que le service devrait ensuite rattraper.
    let deja_assis: HashSet<Uuid> = committee.iter().map(|m| m.person_id).collect();
    let committee_candidates =
        assignables(&mut *conn, event_id, &[REVIEW_WRITE], &deja_assis).await?;

    let available_themes = pastilles(cross::themes_disponibles(&mut *conn).await?)?;

    Ok(Some(EditionDetail {
        edition,
        description,
        highlights,
        period,
        images,
        days,
        tracks,
        venues,
        channels,
        call,
        committee,
        curators,
        committee_candidates,
        available_themes,
    }))
}

// -----------------------------------------------------------------------------
// La période, et les six onglets
// -----------------------------------------------------------------------------

/// La période **en dates civiles, dans le fuseau de l'édition** (§ R5).
///
/// `ck_events_period` garantit une fin postérieure au début : la série de dates
/// porte donc toujours au moins un jour. Une série vide serait une incohérence
/// de la base, et elle doit se voir plutôt que se replier en silence sur la date
/// du serveur — c'est précisément le décalage d'un jour qu'on cherche à éviter.
async fn periode(conn: &mut PgConnection, event_id: EventId) -> Result<EditionPeriod> {
    let jours = editions::periode_civile(&mut *conn, event_id).await?;
    let (Some(premier), Some(dernier)) = (jours.first(), jours.last()) else {
        return Err(ApiError::internal(
            "période civile vide pour une édition dont la base garantit la période",
        ));
    };

    Ok(EditionPeriod {
        first_day: *premier,
        last_day: *dernier,
    })
}

/// Les journées, avec leurs séances et le marquage **hors période**.
///
/// Une journée hors période n'est pas une erreur : une soirée d'ouverture la
/// veille est un cas légitime. On la signale ; on ne la supprime pas d'office
/// (FR-035).
async fn journees(
    conn: &mut PgConnection,
    event_id: EventId,
    period: &EditionPeriod,
) -> Result<Vec<crate::domain::detail::EditionDay>> {
    let mut journees = days::de_l_edition(&mut *conn, event_id).await?;
    let seances = cross::seances_par_journee(&mut *conn, event_id).await?;

    for j in &mut journees {
        j.session_count = seances.get(&j.id).copied().unwrap_or(0);
        j.is_outside_period = j.day_date < period.first_day || j.day_date > period.last_day;
    }

    Ok(journees)
}

/// Les fils, avec leurs séances rattachées, leurs thématiques et le **nom** de
/// leur responsable.
async fn fils(
    conn: &mut PgConnection,
    event_id: EventId,
) -> Result<Vec<crate::domain::detail::EditionTrack>> {
    let mut fils = tracks::de_l_edition(&mut *conn, event_id).await?;
    if fils.is_empty() {
        return Ok(fils);
    }

    let identifiants: Vec<Uuid> = fils.iter().map(|f| f.id).collect();
    let seances = cross::seances_par_fil(&mut *conn, event_id).await?;
    let themes = cross::themes_des_fils(&mut *conn, &identifiants).await?;

    let responsables: Vec<Uuid> = fils.iter().filter_map(|f| f.curated_by).collect();
    let noms = cross::noms_de_personnes(&mut *conn, &responsables).await?;

    for f in &mut fils {
        f.session_count = seances.get(&f.id).copied().unwrap_or(0);
        f.curator_name = f.curated_by.and_then(|id| noms.get(&id).cloned());
        if let Some(badges) = themes.get(&f.id) {
            f.themes = pastilles(badges.clone())?;
        }
    }

    Ok(fils)
}

/// Les lieux et leurs salles, avec ce qu'un retrait déplacerait.
async fn lieux(
    conn: &mut PgConnection,
    event_id: EventId,
) -> Result<Vec<crate::domain::detail::EditionVenue>> {
    let mut lieux = venues::de_l_edition(&mut *conn, event_id).await?;
    let seances = cross::seances_par_salle(&mut *conn, event_id).await?;

    for lieu in &mut lieux {
        for salle in &mut lieu.rooms {
            salle.session_count = seances.get(&salle.id).copied().unwrap_or(0);
        }
    }

    Ok(lieux)
}

/// Les canaux — ceux de l'édition **et** les généraux de la plateforme.
async fn canaux(
    conn: &mut PgConnection,
    event_id: EventId,
) -> Result<Vec<crate::domain::detail::EditionChannel>> {
    let mut canaux = channels::de_l_edition(&mut *conn, event_id).await?;
    let identifiants: Vec<Uuid> = canaux.iter().map(|c| c.id).collect();
    let seances = cross::seances_par_canal(&mut *conn, &identifiants).await?;

    for c in &mut canaux {
        c.session_count = seances.get(&c.id).copied().unwrap_or(0);
    }

    Ok(canaux)
}

/// L'appel et sa grille. **Zéro ou un**, jamais un tableau.
async fn appel(
    conn: &mut PgConnection,
    event_id: EventId,
) -> Result<Option<crate::domain::detail::EditionCall>> {
    let Some(mut appel) = calls::de_l_edition(&mut *conn, event_id).await? else {
        return Ok(None);
    };

    let call_id = CallId::from(appel.id);
    appel.proposal_count = cross::dossiers_de_l_appel(&mut *conn, call_id).await?;

    let mut grille = criteria::de_l_appel(&mut *conn, call_id).await?;
    let notes = cross::notes_par_critere(&mut *conn, call_id).await?;
    for critere in &mut grille {
        if let Some(id) = critere.id {
            critere.score_count = notes.get(&id).copied().unwrap_or(0);
        }
    }
    appel.criteria = grille;

    Ok(Some(appel))
}

/// Le comité, **résolu**. Un siège dont la personne a disparu de `identity` ne
/// s'invente pas : la clé étrangère est `ON DELETE CASCADE`, le cas ne se
/// produit pas, et on l'écarte plutôt que de rendre une ligne sans nom.
async fn comite(
    conn: &mut PgConnection,
    call_id: CallId,
    event_id: EventId,
) -> Result<Vec<EditionCommitteeMember>> {
    let sieges = committee::de_l_appel(&mut *conn, call_id).await?;
    if sieges.is_empty() {
        return Ok(Vec::new());
    }

    let personnes = cross::comite_resolu(&mut *conn, call_id, event_id).await?;

    Ok(sieges
        .into_iter()
        .filter_map(|s| {
            let p = personnes.get(&s.person_id)?;
            Some(EditionCommitteeMember {
                person_id: s.person_id,
                full_name: p.full_name.clone(),
                email: p.email.clone(),
                organization_name: p.organization_name.clone(),
                is_lead: s.is_lead,
                workload_cap: s.workload_cap,
                added_at: s.added_at,
                assigned_count: p.assigned_count,
                submitted_count: p.submitted_count,
                has_review_permission: p.has_review_permission,
            })
        })
        .collect())
}

/// Les personnes désignables, **par permission et jamais par nom de rôle**.
async fn assignables(
    conn: &mut PgConnection,
    event_id: EventId,
    permissions: &[&str],
    exclues: &HashSet<Uuid>,
) -> Result<Vec<CommitteeCandidate>> {
    let candidats = cross::personnes_assignables(&mut *conn, event_id, permissions).await?;

    Ok(candidats
        .into_iter()
        .filter(|c| !exclues.contains(&c.person_id))
        .map(|c| CommitteeCandidate {
            person_id: c.person_id,
            full_name: c.full_name,
            email: c.email,
            organization_name: c.organization_name,
            has_review_permission: c.has_review_permission,
        })
        .collect())
}

/// Les pastilles thématiques, telles que `reference.term_badges()` les rend.
///
/// La fonction du modèle garantit un tableau — jamais `NULL` — : une forme
/// inattendue est une incohérence de la base, pas une donnée manquante.
fn pastilles(badges: serde_json::Value) -> Result<Vec<ThemeBadge>> {
    serde_json::from_value(badges)
        .map_err(|_| ApiError::internal("pastilles thématiques de forme inattendue"))
}

/// **L'appel tel qu'il vient d'être écrit** — sa grille, ses décomptes et les
/// trois fonctions du modèle, hors de la composition du détail.
///
/// La lecture par édition écarte l'annulé ; celle-ci non. Après un
/// enregistrement, c'est l'appel écrit qu'il faut rendre, fût-il annulé — sans
/// quoi annuler un appel rendrait `null`, et l'écran croirait l'avoir perdu.
pub async fn appel_par_id(
    pool: &PgPool,
    call_id: CallId,
) -> Result<Option<crate::domain::detail::EditionCall>> {
    let Some(mut appel) = calls::par_id(pool, call_id).await? else {
        return Ok(None);
    };

    appel.proposal_count = cross::dossiers_de_l_appel(pool, call_id).await?;

    let mut grille = criteria::de_l_appel(pool, call_id).await?;
    let notes = cross::notes_par_critere(pool, call_id).await?;
    for critere in &mut grille {
        if let Some(id) = critere.id {
            critere.score_count = notes.get(&id).copied().unwrap_or(0);
        }
    }
    appel.criteria = grille;

    Ok(Some(appel))
}

/// Le comité **résolu**, hors de la composition du détail — ce que rend
/// l'enregistrement d'une composition.
pub async fn comite_resolu(
    pool: &PgPool,
    call_id: CallId,
    event_id: EventId,
) -> Result<Vec<EditionCommitteeMember>> {
    let mut conn = pool.acquire().await?;
    comite(&mut conn, call_id, event_id).await
}
