//! Le fil des échanges — **filtré par visibilité à la source, jamais après
//! coup**.
//!
//! # Trois visibilités, et la plus dangereuse est la deuxième
//!
//! `committee` reste entre membres du comité ; `submitter` **part chez le
//! déposant** ; `private` n'est lue que de son auteur. Se tromper est
//! irrattrapable — un message lu ne se retire pas.
//!
//! Le filtrage vit dans la clause `WHERE`, et non dans une boucle qui écarte
//! après lecture : ce qui n'est pas envoyé ne peut pas fuiter, tandis qu'un
//! filtre applicatif laisse la donnée à portée d'un champ oublié dans un type
//! de sortie, d'une trace de débogage, d'un message d'erreur enrichi. C'est le
//! même parti que le voile de l'aveugle, et pour la même raison.
//!
//! # Une demande de correction est forcée en visibilité partagée
//!
//! Les deux colonnes sont indépendantes en base : rien n'empêche une demande
//! de correction en visibilité « comité ». Elle bloquerait alors le dossier
//! **sans que le déposant sache pourquoi** (écart n° 99). Le service l'impose ;
//! ce fichier écrit ce qu'on lui donne.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::ProposalId;

/// Un message — exactement `ProposalComment`.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct Message {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub author_id: Uuid,
    pub visibility: String,
    pub body: String,
    pub is_change_request: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub resolved_at: Option<OffsetDateTime>,
    pub resolved_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub edited_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

macro_rules! message_depuis {
    ($l:expr) => {
        Message {
            id: $l.id,
            proposal_id: $l.proposal_id,
            parent_id: $l.parent_id,
            author_id: $l.author_id,
            visibility: $l.visibility,
            body: $l.body,
            is_change_request: $l.is_change_request,
            resolved_at: $l.resolved_at,
            resolved_by: $l.resolved_by,
            edited_at: $l.edited_at,
            created_at: $l.created_at,
        }
    };
}

/// De quel côté le lecteur se trouve. **Ce n'est pas un niveau de droit mais
/// une place** : le comité voit le fil interne, l'organisation ne voit que ce
/// qu'on lui a adressé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cote {
    Comite,
    Organisation,
}

/// Le fil d'un dossier, **tel que ce lecteur a le droit de le voir**.
///
/// Côté comité : ce que `programme.comment_in_committee_circle()` lui ouvre,
/// la règle tenue en base (fil adressé : son cercle seul, administrateurs
/// compris). Côté organisation : les messages qui lui sont adressés, et rien
/// d'autre, ni note personnelle, ni délibération.
pub async fn fil<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    lecteur: Uuid,
    cote: Cote,
) -> Result<Vec<Message>> {
    let comite = cote == Cote::Comite;

    let lignes = sqlx::query!(
        r#"SELECT id, proposal_id, parent_id, author_id,
                  visibility::text AS "visibility!", body, is_change_request,
                  resolved_at, resolved_by, edited_at, created_at
             FROM programme.proposal_comments
            WHERE proposal_id = $1
              AND deleted_at IS NULL
              AND CASE WHEN $2 THEN programme.comment_in_committee_circle(id, $3)
                       ELSE visibility = 'submitter' END
            ORDER BY created_at, id"#,
        dossier.as_uuid(),
        comite,
        lecteur
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| message_depuis!(l)).collect())
}

/// Ce qu'une écriture pose.
pub struct NouveauMessage<'a> {
    pub parent_id: Option<Uuid>,
    pub visibility: &'a str,
    pub body: &'a str,
    pub is_change_request: bool,
}

/// Écrire un message. **Le corps est refusé vide par la base** — `length(btrim(
/// body)) > 0` —, et le service traduit plutôt que de redoubler la règle.
pub async fn ecrire(
    conn: &mut PgConnection,
    dossier: ProposalId,
    auteur: Uuid,
    nouveau: &NouveauMessage<'_>,
) -> Result<Message> {
    let ligne = sqlx::query!(
        r#"INSERT INTO programme.proposal_comments
               (proposal_id, parent_id, author_id, visibility, body, is_change_request)
           VALUES ($1, $2, $3, $4::text::programme.comment_visibility, $5, $6)
        RETURNING id, proposal_id, parent_id, author_id,
                  visibility::text AS "visibility!", body, is_change_request,
                  resolved_at, resolved_by, edited_at, created_at"#,
        dossier.as_uuid(),
        nouveau.parent_id,
        auteur,
        nouveau.visibility,
        nouveau.body,
        nouveau.is_change_request
    )
    .fetch_one(conn)
    .await?;

    Ok(message_depuis!(ligne))
}

/// Un message par son identifiant, **effacé exclu** — ce qu'il faut connaître
/// avant de répondre ou de résoudre.
pub async fn par_id<'e>(
    executor: impl PgExecutor<'e>,
    message: crate::domain::ids::CommentId,
) -> Result<Option<Message>> {
    let ligne = sqlx::query!(
        r#"SELECT id, proposal_id, parent_id, author_id,
                  visibility::text AS "visibility!", body, is_change_request,
                  resolved_at, resolved_by, edited_at, created_at
             FROM programme.proposal_comments
            WHERE id = $1 AND deleted_at IS NULL"#,
        message.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| message_depuis!(l)))
}

// -----------------------------------------------------------------------------
// Destinataires et lectures (27/10)
// -----------------------------------------------------------------------------

/// `CommentPerson`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ToSchema)]
pub struct CommentPerson {
    pub person_id: Uuid,
    pub display_name: String,
}

/// `CommentReader`. `side` vaut `committee` ou `organisation`.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct CommentReader {
    pub person_id: Uuid,
    pub display_name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub read_at: OffsetDateTime,
    pub side: &'static str,
}

/// Ce lecteur peut-il lire ce message, du côté où il se tient ?
pub async fn lisible<'e>(
    executor: impl PgExecutor<'e>,
    message: Uuid,
    lecteur: Uuid,
    cote: Cote,
) -> Result<bool> {
    let lisible = sqlx::query_scalar!(
        r#"SELECT CASE WHEN $3 THEN programme.comment_in_committee_circle(c.id, $2)
                       ELSE c.visibility = 'submitter' END AS "lisible!"
             FROM programme.proposal_comments c
            WHERE c.id = $1 AND c.deleted_at IS NULL"#,
        message,
        lecteur,
        cote == Cote::Comite
    )
    .fetch_optional(executor)
    .await?;

    Ok(lisible.unwrap_or(false))
}

/// Le message qui ouvre le fil de celui-ci (lui-même s'il n'a pas de parent).
pub async fn racine<'e>(executor: impl PgExecutor<'e>, message: Uuid) -> Result<Option<Uuid>> {
    let racine = sqlx::query_scalar!(
        r#"WITH RECURSIVE fil AS (
               SELECT id, parent_id FROM programme.proposal_comments WHERE id = $1
               UNION ALL
               SELECT c.id, c.parent_id FROM programme.proposal_comments c JOIN fil f ON c.id = f.parent_id
           )
           SELECT id AS "id!" FROM fil WHERE parent_id IS NULL"#,
        message
    )
    .fetch_optional(executor)
    .await?;

    Ok(racine)
}

/// Les destinataires nommés d'un fil.
pub async fn destinataires<'e>(executor: impl PgExecutor<'e>, racine: Uuid) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT person_id FROM programme.proposal_comment_recipients
          WHERE comment_id = $1 ORDER BY added_at, person_id",
        racine
    )
    .fetch_all(executor)
    .await?;

    Ok(ids)
}

/// Poser des destinataires ; rend ceux qui ne l'étaient pas encore.
pub async fn ajouter_destinataires(
    conn: &mut PgConnection,
    racine: Uuid,
    personnes: &[Uuid],
    par: Uuid,
) -> Result<Vec<Uuid>> {
    let ajoutes = sqlx::query_scalar!(
        "INSERT INTO programme.proposal_comment_recipients (comment_id, person_id, added_by)
         SELECT $1, unnest($2::uuid[]), $3
         ON CONFLICT DO NOTHING
         RETURNING person_id",
        racine,
        personnes,
        par
    )
    .fetch_all(conn)
    .await?;

    Ok(ajoutes)
}

/// Les messages de ce dossier que ce lecteur a lus.
pub async fn lus_par<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    lecteur: Uuid,
) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT r.comment_id FROM programme.proposal_comment_reads r
           JOIN programme.proposal_comments c ON c.id = r.comment_id
          WHERE c.proposal_id = $1 AND r.person_id = $2",
        dossier.as_uuid(),
        lecteur
    )
    .fetch_all(executor)
    .await?;

    Ok(ids)
}

/// Marquer lus. Ses propres messages et ceux qu'il ne peut pas lire sont
/// écartés par la requête elle-même : le filtre vit dans le `WHERE`, comme
/// celui du fil.
pub async fn marquer_lus(
    conn: &mut PgConnection,
    dossier: ProposalId,
    lecteur: Uuid,
    messages: &[Uuid],
    cote: Cote,
) -> Result<u64> {
    let poses = sqlx::query!(
        "INSERT INTO programme.proposal_comment_reads (comment_id, person_id)
         SELECT c.id, $2 FROM programme.proposal_comments c
          WHERE c.id = ANY($3) AND c.proposal_id = $1
            AND c.deleted_at IS NULL AND c.author_id <> $2
            AND CASE WHEN $4 THEN programme.comment_in_committee_circle(c.id, $2)
                     ELSE c.visibility = 'submitter' END
         ON CONFLICT DO NOTHING",
        dossier.as_uuid(),
        lecteur,
        messages,
        cote == Cote::Comite
    )
    .execute(conn)
    .await?;

    Ok(poses.rows_affected())
}

pub async fn marquer_non_lu(conn: &mut PgConnection, message: Uuid, lecteur: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM programme.proposal_comment_reads WHERE comment_id = $1 AND person_id = $2",
        message,
        lecteur
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Pour chaque message, combien de personnes de `audience` le lisent, son
/// auteur exclu : le « sur M » d'un accusé.
pub async fn tailles_daudience<'e>(
    executor: impl PgExecutor<'e>,
    messages: &[Uuid],
    audience: &[Uuid],
) -> Result<Vec<(Uuid, i64)>> {
    let lignes = sqlx::query!(
        r#"SELECT c.id, count(a.pid) AS "n!"
             FROM programme.proposal_comments c
             LEFT JOIN LATERAL unnest($2::uuid[]) AS a(pid)
                    ON a.pid <> c.author_id
                   AND programme.comment_in_committee_circle(c.id, a.pid)
            WHERE c.id = ANY($1)
            GROUP BY c.id"#,
        messages,
        audience
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.id, l.n)).collect())
}

/// `UnreadCommentCount`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ToSchema)]
pub struct UnreadCommentCount {
    pub proposal_id: Uuid,
    pub count: i64,
}

/// Par dossier de l'édition, les messages que cette personne lit côté comité,
/// qu'elle n'a pas écrits et qu'elle n'a pas encore lus.
pub async fn non_lus_par_dossier<'e>(
    executor: impl PgExecutor<'e>,
    lecteur: Uuid,
    edition: crate::domain::ids::EventId,
) -> Result<Vec<UnreadCommentCount>> {
    let lignes = sqlx::query!(
        r#"SELECT c.proposal_id, count(*) AS "n!"
             FROM programme.proposal_comments c
             JOIN programme.proposals d ON d.id = c.proposal_id
            WHERE d.event_id = $2 AND d.deleted_at IS NULL
              AND c.deleted_at IS NULL AND c.author_id <> $1
              AND NOT EXISTS (SELECT 1 FROM programme.proposal_comment_reads r
                               WHERE r.comment_id = c.id AND r.person_id = $1)
              AND programme.comment_in_committee_circle(c.id, $1)
            GROUP BY c.proposal_id
            ORDER BY c.proposal_id"#,
        lecteur,
        edition.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| UnreadCommentCount {
            proposal_id: l.proposal_id,
            count: l.n,
        })
        .collect())
}
