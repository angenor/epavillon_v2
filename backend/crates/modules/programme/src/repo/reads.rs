//! Les accusés de lecture — **une relation, pas une propriété du dossier**.
//!
//! « Ce dossier, moi, l'ai-je ouvert ? » n'est pas une colonne : la même ligne
//! est lue par l'un et pas par l'autre. La vue de pilotage porte donc un
//! compteur **collectif** (`read_count`), et la réponse personnelle vient de
//! `programme.unread_proposals_for()`, qui prend le lecteur en paramètre.
//!
//! Le modèle explique aussi pourquoi la fonction ne lit pas
//! `current_setting('app.actor_id')` : son résultat deviendrait invisible à la
//! relecture — deux requêtes identiques, deux réponses.
//!
//! **C'est la seule exception à R16** : les sept facettes se comptent sur les
//! lignes déjà lues, celle-ci non, parce qu'elle ne se déduit pas des lignes.

use kernel::error::Result;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::ids::EventId;

/// Les dossiers d'une édition que cette personne n'a **jamais** ouverts.
///
/// La fonction du modèle exclut déjà les dossiers effacés et ordonne par
/// numéro de dossier : on ne retrie pas.
pub async fn non_lus<'e>(
    executor: impl PgExecutor<'e>,
    personne: Uuid,
    edition: EventId,
) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        r#"SELECT programme.unread_proposals_for($1, $2) AS "ids!""#,
        personne,
        edition.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(ids)
}

/// Ce dossier a-t-il **déjà** été ouvert par cette personne ?
///
/// **À lire avant de poser l'accusé**, sans quoi la réponse dirait toujours
/// « déjà vu » : la fonction du modèle insère ou incrémente, elle ne distingue
/// pas. L'écran, lui, a besoin de l'état **d'avant** la visite — c'est ce qui
/// lui permet de signaler un dossier qu'on découvre.
pub async fn deja_lu<'e>(
    executor: impl PgExecutor<'e>,
    dossier: crate::domain::ids::ProposalId,
    personne: Uuid,
) -> Result<bool> {
    let lu = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM programme.proposal_reads
                WHERE proposal_id = $1 AND person_id = $2
           ) AS "lu!""#,
        dossier.as_uuid(),
        personne
    )
    .fetch_one(executor)
    .await?;

    Ok(lu)
}

/// Poser l'accusé de lecture, **par la fonction du modèle**.
///
/// C'est une **lecture qui écrit**, assumée par le modèle lui-même : la
/// composition de la fiche passe donc par la porte d'écriture du noyau, qui
/// pose l'acteur et l'identifiant de requête (principe VII, R3). La déguiser en
/// deux appels dont l'un serait hors contexte reviendrait à écrire sans acteur.
pub async fn poser_accuse(
    conn: &mut sqlx::postgres::PgConnection,
    dossier: crate::domain::ids::ProposalId,
    personne: Uuid,
) -> Result<()> {
    sqlx::query!(
        "SELECT programme.record_proposal_read($1, $2)",
        dossier.as_uuid(),
        personne
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Combien de membres du comité ont ouvert ce dossier. **Collectif** — c'est le
/// « lu par 3 membres du comité » de l'en-tête.
pub async fn compter_lecteurs<'e>(
    executor: impl PgExecutor<'e>,
    dossier: crate::domain::ids::ProposalId,
) -> Result<i64> {
    let compte = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.proposal_reads WHERE proposal_id = $1"#,
        dossier.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(compte)
}
