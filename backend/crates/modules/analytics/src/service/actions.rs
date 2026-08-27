//! **Les cinq familles d'alerte, leur critère, leurs exemples, leur ordre.**
//!
//! Une ligne par famille, jamais une par élément. **Une famille sans élément
//! n'émet aucune ligne** — une édition où tout va bien rend `actions: []`, et
//! l'écran doit rester lisible ainsi.
//!
//! **Trois exemples nommés au plus.** « 7 revues en retard » ne dit pas par où
//! commencer ; « 7 revues en retard — Lemoine (3), Ben Amor (2) » le dit.
//! Au-delà de trois, la ligne cesse d'être un résumé et il faut ouvrir l'écran
//! concerné : c'est précisément ce que le lien propose.
//!
//! **Rangement** : gravité, puis échéance la plus proche, puis décompte.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::action::{
    AdminAction, AdminActionExample, AdminActionKind, AdminActionSeverity,
};
use crate::repo::cross::{live, org, programme};
use crate::repo::settings;

/// Le nombre d'exemples nommés sous une ligne. Règle d'écran, portée par le
/// contrat du site.
const EXEMPLES: usize = 3;

pub async fn composer(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<AdminAction>> {
    let mut actions = Vec::new();

    if let Some(action) = dossiers_sans_evaluation(&mut *conn, event_id).await? {
        actions.push(action);
    }
    if let Some(action) = revues_en_retard(&mut *conn, event_id).await? {
        actions.push(action);
    }
    if let Some(action) = messages_actifs(&mut *conn, event_id).await? {
        actions.push(action);
    }
    if let Some(action) = chevauchements(&mut *conn, event_id).await? {
        actions.push(action);
    }
    if let Some(action) = doublons(&mut *conn).await? {
        actions.push(action);
    }

    // Gravité d'abord, puis l'échéance la plus proche — une ligne sans échéance
    // passe après celles qui en ont —, puis le décompte décroissant.
    actions.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| match (a.due_at, b.due_at) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| b.count.cmp(&a.count))
    });

    Ok(actions)
}

/// **Le seuil vient de la base**, jamais d'une constante : c'est une règle
/// d'exploitation que l'IFDD ajuste d'une COP à l'autre, sans redéploiement.
async fn dossiers_sans_evaluation(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Option<AdminAction>> {
    let jours = settings::jours_avant_alerte(&mut *conn).await?;
    let dossiers = programme::dossiers_sans_evaluation(&mut *conn, event_id, jours).await?;
    if dossiers.is_empty() {
        return Ok(None);
    }

    let due_at = dossiers.iter().filter_map(|d| d.echeance_applicable).min();
    let examples = dossiers
        .iter()
        .take(EXEMPLES)
        .map(|d| AdminActionExample {
            label: d
                .title_text
                .clone()
                .unwrap_or_else(|| d.reference_code.clone()),
            hint: Some(d.reference_code.clone()),
            target: Some(format!("/admin/propositions/{}", d.proposal_id)),
        })
        .collect();

    Ok(Some(AdminAction {
        kind: AdminActionKind::ProposalsUnreviewed,
        severity: AdminActionSeverity::High,
        count: dossiers.len() as i64,
        due_at,
        examples,
        target: "/admin/propositions?filtre=non-evaluees".to_owned(),
    }))
}

async fn revues_en_retard(conn: &mut PgConnection, event_id: Uuid) -> Result<Option<AdminAction>> {
    let charge = crate::repo::projections::charge_du_comite(&mut *conn, event_id).await?;
    let en_retard: Vec<_> = charge
        .into_iter()
        .filter(|c| c.revues_en_retard > 0)
        .collect();
    if en_retard.is_empty() {
        return Ok(None);
    }

    let count = en_retard.iter().map(|c| c.revues_en_retard).sum();
    let due_at = programme::prochaine_echeance_de_revue(&mut *conn, event_id).await?;
    let examples = en_retard
        .iter()
        .take(EXEMPLES)
        .map(|c| AdminActionExample {
            label: c
                .revisionniste
                .clone()
                .unwrap_or_else(|| "Révisionniste".to_owned()),
            hint: Some(c.revues_en_retard.to_string()),
            target: None,
        })
        .collect();

    Ok(Some(AdminAction {
        kind: AdminActionKind::ReviewsOverdue,
        severity: AdminActionSeverity::High,
        count,
        due_at,
        examples,
        target: "/admin/evaluations?filtre=en-retard".to_owned(),
    }))
}

/// **`high`, parce qu'un message actif est VU DU PUBLIC.** C'est la seule
/// famille dont la gravité ne vient pas d'une échéance : elle vient de ce que
/// quelqu'un, dehors, lit ce texte en ce moment.
async fn messages_actifs(conn: &mut PgConnection, event_id: Uuid) -> Result<Option<AdminAction>> {
    let actifs = live::incidents_actifs(&mut *conn, event_id).await?;
    if actifs.is_empty() {
        return Ok(None);
    }

    let examples = actifs
        .iter()
        .take(EXEMPLES)
        .map(|i| AdminActionExample {
            label: i
                .target_label
                .clone()
                .unwrap_or_else(|| "Toute la plateforme".to_owned()),
            hint: Some(i.severity.clone()),
            target: Some(format!("/admin/incidents/{}", i.incident_id)),
        })
        .collect();

    // La fin d'affichage la plus proche : c'est la seule échéance qu'un message
    // porte, et un message SANS fin est justement le vrai danger de la table —
    // il n'en porte donc aucune, et la ligne remonte sur sa seule gravité.
    let due_at = actifs.iter().filter_map(|i| i.display_until).min();

    Ok(Some(AdminAction {
        kind: AdminActionKind::ActiveIncidents,
        severity: AdminActionSeverity::High,
        count: actifs.len() as i64,
        due_at,
        examples,
        target: "/admin/incidents".to_owned(),
    }))
}

/// **Jamais bloquants** (règle métier n° 2) : on détecte, on affiche, on
/// n'empêche pas. La gravité vient de la fonction du modèle — un conflit
/// `blocking` fait passer la ligne en `high` —, jamais d'un jugement du code.
async fn chevauchements(conn: &mut PgConnection, event_id: Uuid) -> Result<Option<AdminAction>> {
    let conflits = programme::conflits(&mut *conn, event_id).await?;
    if conflits.is_empty() {
        return Ok(None);
    }

    let severity = if conflits.iter().any(|c| c.severity == "blocking") {
        AdminActionSeverity::High
    } else {
        AdminActionSeverity::Medium
    };

    let examples = conflits
        .iter()
        .take(EXEMPLES)
        .map(|c| AdminActionExample {
            label: match (&c.session_a_title, &c.session_b_title) {
                (Some(a), Some(b)) => format!("{a} / {b}"),
                (Some(a), None) => a.clone(),
                _ => c.conflict_kind.clone(),
            },
            hint: c.subject_label.clone(),
            target: None,
        })
        .collect();

    Ok(Some(AdminAction {
        kind: AdminActionKind::ScheduleConflicts,
        severity,
        count: conflits.len() as i64,
        due_at: None,
        examples,
        target: "/admin/programmation?filtre=conflits".to_owned(),
    }))
}

/// **Cette famille n'est pas filtrée par édition** — un doublon ne se rattache à
/// aucune — et **ne révèle l'existence d'aucune autre** : elle ne nomme que des
/// organisations.
async fn doublons(conn: &mut PgConnection) -> Result<Option<AdminAction>> {
    let paires = org::doublons_a_arbitrer(&mut *conn).await?;
    if paires.is_empty() {
        return Ok(None);
    }

    let examples = paires
        .iter()
        .take(EXEMPLES)
        .map(|p| AdminActionExample {
            label: format!("{} / {}", p.gauche, p.droite),
            hint: Some(format!("{:.0}", p.score)),
            target: None,
        })
        .collect();

    Ok(Some(AdminAction {
        kind: AdminActionKind::OrganizationDuplicates,
        severity: AdminActionSeverity::Medium,
        count: paires.len() as i64,
        due_at: None::<OffsetDateTime>,
        examples,
        target: "/admin/organisations/doublons".to_owned(),
    }))
}
