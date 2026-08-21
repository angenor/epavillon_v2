//! `programme.detect_conflicts()`, **telle quelle**.
//!
//! # Ni filtrée, ni requalifiée
//!
//! Les cinq branches de la fonction et leurs deux gravités sont un arbitrage du
//! modèle, corrigé le 18/08 après avoir noyé le planificateur sous des conflits
//! qui n'en étaient pas — un atelier en salle virtuelle n'occupe aucun mètre
//! carré du pavillon. Requalifier ici produirait deux vérités : celle du bandeau
//! et celle du contrôle avant publication, qui lit la même fonction.
//!
//! # Aucun de ces conflits ne refuse une écriture
//!
//! C'est la décision structurante n° 1 du fichier `075`, et la règle métier
//! n° 2 : l'équipe travaille par déplacements successifs, en passant par des
//! états incohérents. Le seul garde-fou dur est la publication du programme.

use kernel::error::Result;
use sqlx::PgExecutor;

use crate::domain::ids::EventId;
use crate::domain::sessions::ScheduleConflict;

/// Les chevauchements d'une édition.
///
/// **L'intervalle traverse en `text`** : le contrat du front déclare une chaîne,
/// et c'est la représentation que PostgreSQL rend déjà. La recomposer côté Rust
/// depuis un `PgRange` reviendrait à réécrire ce que la base sait faire (R26).
pub async fn conflits<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<ScheduleConflict>> {
    let lignes = sqlx::query!(
        r#"SELECT severity AS "severity!", conflict_kind AS "conflict_kind!",
                  subject_id, subject_label,
                  session_a AS "session_a!", session_a_title,
                  session_b AS "session_b!", session_b_title,
                  overlap::text AS "overlap?"
             FROM programme.detect_conflicts($1)
            ORDER BY severity, conflict_kind, session_a, session_b"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ScheduleConflict {
            severity: l.severity,
            conflict_kind: l.conflict_kind,
            subject_id: l.subject_id,
            subject_label: l.subject_label,
            session_a: l.session_a,
            session_a_title: l.session_a_title,
            session_b: l.session_b,
            session_b_title: l.session_b_title,
            overlap: l.overlap,
        })
        .collect())
}
