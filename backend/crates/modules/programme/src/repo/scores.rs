//! Les notes par critère — **et ce qu'une note absente signifie**.
//!
//! # Une note absente n'est pas une note à zéro
//!
//! Zéro sur un critère éliminatoire **disqualifie** le dossier ; ne pas avoir
//! encore noté ne disqualifie rien. Le modèle distingue les deux par l'absence
//! de ligne, et ce fichier ne comble jamais un trou par un zéro.
//!
//! Conséquence directe sur l'écriture : une note **retirée** de la charge
//! utile doit disparaître de la table. Se contenter d'un `ON CONFLICT DO
//! UPDATE` laisserait la ligne précédente en place, et un membre du comité qui
//! efface une note verrait l'ancienne revenir au rechargement — avec, pour un
//! critère éliminatoire, un dossier qui reste éliminé sans raison visible.
//!
//! # Le plafond dépend du critère, et c'est la base qui l'arbitre
//!
//! `tg_check_score_bounds()` refuse une note supérieure au `max_score` du
//! critère, valeur qui varie d'un appel à l'autre. Le service **ne le
//! redouble pas** : il traduit le refus en nommant le critère et sa borne.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use utoipa::ToSchema;
use uuid::Uuid;

/// Une note par critère — exactement `ReviewScore`.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct Note {
    pub review_id: Uuid,
    pub criterion_id: Uuid,
    pub score: f64,
    pub comment: Option<String>,
}

/// Ce que l'appelant demande d'écrire, critère par critère.
pub struct NoteAPoser {
    pub criterion_id: Uuid,
    pub score: f64,
    pub comment: Option<String>,
}

/// **Remplacer** les notes d'une revue par celles reçues.
///
/// Les tableaux voyagent en un seul aller : `unnest` pose autant de lignes
/// qu'il y a de critères, et la suppression préalable porte sur ce qui n'est
/// **pas** dans la liste. Écrire critère par critère coûterait six
/// allers-retours par frappe, dans un formulaire qui s'enregistre seul.
pub async fn remplacer(conn: &mut PgConnection, revue: Uuid, notes: &[NoteAPoser]) -> Result<()> {
    let criteres: Vec<Uuid> = notes.iter().map(|n| n.criterion_id).collect();
    let valeurs: Vec<f64> = notes.iter().map(|n| n.score).collect();
    let commentaires: Vec<Option<String>> = notes.iter().map(|n| n.comment.clone()).collect();

    sqlx::query!(
        "DELETE FROM programme.review_scores
          WHERE review_id = $1 AND criterion_id <> ALL($2::uuid[])",
        revue,
        &criteres
    )
    .execute(&mut *conn)
    .await?;

    if criteres.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO programme.review_scores (review_id, criterion_id, score, comment)
         SELECT $1, c.critere, v.valeur::numeric(5,2), t.texte
           FROM unnest($2::uuid[])  WITH ORDINALITY AS c(critere, i)
           JOIN unnest($3::float8[]) WITH ORDINALITY AS v(valeur, i) USING (i)
           JOIN unnest($4::text[])  WITH ORDINALITY AS t(texte, i)  USING (i)
         ON CONFLICT (review_id, criterion_id) DO UPDATE
            SET score = EXCLUDED.score, comment = EXCLUDED.comment",
        revue,
        &criteres,
        &valeurs,
        &commentaires as &[Option<String>]
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Les notes d'une revue.
pub async fn de_la_revue<'e>(executor: impl PgExecutor<'e>, revue: Uuid) -> Result<Vec<Note>> {
    des_revues(executor, &[revue]).await
}

/// Les notes de plusieurs revues, **en une requête** : les demander une par une
/// est le N+1 que la composition de la fiche existe pour éviter.
pub async fn des_revues<'e>(executor: impl PgExecutor<'e>, revues: &[Uuid]) -> Result<Vec<Note>> {
    let lignes = sqlx::query!(
        r#"SELECT rs.review_id, rs.criterion_id, rs.score::float8 AS "score!", rs.comment
             FROM programme.review_scores rs
             JOIN event.review_criteria c ON c.id = rs.criterion_id
            WHERE rs.review_id = ANY($1)
            ORDER BY c.sort_order, c.code"#,
        revues
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Note {
            review_id: l.review_id,
            criterion_id: l.criterion_id,
            score: l.score,
            comment: l.comment,
        })
        .collect())
}
