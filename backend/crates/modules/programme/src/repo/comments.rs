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
/// Côté comité : les messages du comité, ceux adressés au déposant, et **ses
/// propres** notes personnelles. Côté organisation : les messages qui lui sont
/// adressés, et rien d'autre — ni note personnelle, ni délibération.
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
              AND (
                    visibility = 'submitter'
                 OR ($2 AND visibility = 'committee')
                 OR ($2 AND visibility = 'private' AND author_id = $3)
              )
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
