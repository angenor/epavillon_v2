//! Lecture du schéma `org` — **en lecture seule**.
//!
//! La cinquième famille d'alerte : les paires d'organisations présumées
//! identiques, **non arbitrées**.
//!
//! # POURQUOI PAS `v_platform_overview`
//!
//! Elle porte le compte des doublons à arbitrer, et c'est le seul chiffre qu'on
//! lui aurait pris. Mais elle **compte la plateforme entière** (écart n° 44), et
//! elle ne porte **aucun exemple nommé** : « 4 doublons à arbitrer » ne dit pas
//! par où commencer, « IFDD / Institut de la Francophonie… » le dit. La lire
//! aurait coûté une vue de plus pour une information de moins.
//!
//! **Cette famille n'est pas filtrée par édition** — les doublons ne se
//! rattachent à aucune —, et elle **ne révèle l'existence d'aucune autre
//! édition** : elle ne nomme que des organisations.

use kernel::error::Result;
use sqlx::postgres::PgConnection;

/// Une paire présumée identique, avec ses deux dénominations.
///
/// **C'est le défaut n° 1 de la v1** : chercher « IFDD » ou « Institut de la
/// Francophonie pour le développement durable » devait ramener la même fiche, et
/// ne le faisait pas.
pub struct DoublonPresume {
    pub gauche: String,
    pub droite: String,
    pub score: f64,
}

/// Les paires **non arbitrées**, par score décroissant. Une paire déjà tranchée
/// n'appelle plus rien : la laisser remonter ferait de la liste un journal.
pub async fn doublons_a_arbitrer(conn: &mut PgConnection) -> Result<Vec<DoublonPresume>> {
    let lignes = sqlx::query!(
        r#"SELECT g.legal_name AS "gauche!", d.legal_name AS "droite!",
                  c.score::float8 AS "score!"
             FROM org.duplicate_candidates c
             JOIN org.organizations g ON g.id = c.left_id
             JOIN org.organizations d ON d.id = c.right_id
            WHERE c.reviewed_at IS NULL
            ORDER BY c.score DESC, g.legal_name"#
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| DoublonPresume {
            gauche: l.gauche,
            droite: l.droite,
            score: l.score,
        })
        .collect())
}
