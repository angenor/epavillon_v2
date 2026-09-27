//! Les termes proposés. Les lectures de la file ne lisent ni `person_id` ni
//! `handled_by` pour les rendre (R9) ; seule `auteurs_a_prevenir` lit
//! l'adresse des auteurs, pour la charge du courriel.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::savoir::KnowledgeStatus;
use crate::domain::savoir_propositions::{AdminProposalNearby, ProposalReceipt};

/// Sérialise les envois d'une même personne : le plafond et le rejeu se lisent
/// sans course.
pub async fn verrouiller(conn: &mut PgConnection, personne: Uuid) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended('glossary_proposals:' || $1, 0))",
        personne.to_string()
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn recu(
    conn: &mut PgConnection,
    personne: Uuid,
    client_ref: Uuid,
) -> Result<Option<ProposalReceipt>> {
    Ok(sqlx::query_as!(
        ProposalReceipt,
        "SELECT p.id, a.client_ref, p.term, a.created_at
           FROM negotiation.glossary_proposal_authors a
           JOIN negotiation.glossary_proposals p ON p.id = a.proposal_id
          WHERE a.person_id = $1 AND a.client_ref = $2",
        personne,
        client_ref
    )
    .fetch_optional(conn)
    .await?)
}

pub struct Forme {
    /// `None` : le terme n'a ni lettre ni chiffre.
    pub normalisee: Option<String>,
    /// Le slug de l'entrée publiée ou « À revoir » que le terme désigne.
    pub au_lexique: Option<String>,
}

pub async fn forme(conn: &mut PgConnection, term: &str) -> Result<Forme> {
    Ok(sqlx::query_as!(
        Forme,
        r#"SELECT platform.normalize_label($1) AS normalisee,
                  (SELECT slug FROM negotiation.glossary_entries
                    WHERE id = negotiation.glossary_resolve($1)) AS au_lexique"#,
        term
    )
    .fetch_one(conn)
    .await?)
}

pub async fn envoyees_aujourdhui(conn: &mut PgConnection, personne: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM negotiation.glossary_proposal_authors
            WHERE person_id = $1
              AND created_at >= (date_trunc('day', now() AT TIME ZONE 'Europe/Paris')
                                 AT TIME ZONE 'Europe/Paris')"#,
        personne
    )
    .fetch_one(conn)
    .await?)
}

pub struct Cible {
    pub id: Uuid,
    pub term: String,
}

/// La proposition qui recueille un terme : celle qui attend ; à défaut, une
/// acceptée dont l'entrée n'a jamais été publiée — son auteur recevra ainsi le
/// courriel de la publication.
pub async fn cible(conn: &mut PgConnection, term: &str) -> Result<Option<Cible>> {
    Ok(sqlx::query_as!(
        Cible,
        "SELECT p.id, p.term
           FROM negotiation.glossary_proposals p
           LEFT JOIN negotiation.glossary_entries g ON g.id = p.glossary_entry_id
          WHERE p.term_norm = platform.normalize_label($1)
            AND (p.status = 'pending'
                 OR (p.status = 'accepted' AND g.id IS NOT NULL AND g.first_published_at IS NULL))
          ORDER BY p.status <> 'pending', p.handled_at DESC NULLS LAST, p.id
          LIMIT 1",
        term
    )
    .fetch_optional(conn)
    .await?)
}

/// `None` : un jumeau du même terme normalisé est passé entre-temps
/// (`ux_glossary_proposals_pending`).
pub async fn inserer(conn: &mut PgConnection, term: &str) -> Result<Option<Cible>> {
    Ok(sqlx::query_as!(
        Cible,
        "INSERT INTO negotiation.glossary_proposals (term) VALUES ($1)
         ON CONFLICT (term_norm) WHERE status = 'pending' DO NOTHING
         RETURNING id, term",
        term
    )
    .fetch_optional(conn)
    .await?)
}

/// `None` : la personne est déjà auteur de cette proposition.
pub async fn ajouter_l_auteur(
    conn: &mut PgConnection,
    proposition: Uuid,
    personne: Uuid,
    client_ref: Uuid,
    context: Option<&str>,
) -> Result<Option<OffsetDateTime>> {
    Ok(sqlx::query_scalar!(
        "INSERT INTO negotiation.glossary_proposal_authors
             (proposal_id, person_id, client_ref, context)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT DO NOTHING
         RETURNING created_at",
        proposition,
        personne,
        client_ref,
        context
    )
    .fetch_optional(conn)
    .await?)
}

pub async fn auteur_depuis(
    conn: &mut PgConnection,
    proposition: Uuid,
    personne: Uuid,
) -> Result<Option<OffsetDateTime>> {
    Ok(sqlx::query_scalar!(
        "SELECT created_at FROM negotiation.glossary_proposal_authors
          WHERE proposal_id = $1 AND person_id = $2",
        proposition,
        personne
    )
    .fetch_optional(conn)
    .await?)
}

pub struct Ligne {
    pub id: Uuid,
    pub term: String,
    pub status: String,
    pub authors_count: i64,
    pub glossary_entry_id: Option<Uuid>,
    pub glossary_entry_slug: Option<String>,
    pub rejection_reason: Option<String>,
    pub created_at: OffsetDateTime,
    pub handled_at: Option<OffsetDateTime>,
}

/// Sans `id`, celles qui attendent, la plus ancienne d'abord ; avec, celle-là
/// quel que soit son statut.
pub async fn lignes(conn: &mut PgConnection, id: Option<Uuid>) -> Result<Vec<Ligne>> {
    Ok(sqlx::query_as!(
        Ligne,
        r#"SELECT p.id, p.term, p.status::text AS "status!",
                  (SELECT count(*) FROM negotiation.glossary_proposal_authors a
                    WHERE a.proposal_id = p.id) AS "authors_count!",
                  p.glossary_entry_id, g.slug AS "glossary_entry_slug?", p.rejection_reason,
                  p.created_at, p.handled_at
             FROM negotiation.glossary_proposals p
             LEFT JOIN negotiation.glossary_entries g ON g.id = p.glossary_entry_id
            WHERE CASE WHEN $1::uuid IS NULL THEN p.status = 'pending' ELSE p.id = $1 END
            ORDER BY p.created_at, p.id"#,
        id
    )
    .fetch_all(conn)
    .await?)
}

pub struct Contexte {
    pub proposal_id: Uuid,
    pub context: Option<String>,
    pub created_at: OffsetDateTime,
}

pub async fn contextes(conn: &mut PgConnection, ids: &[Uuid]) -> Result<Vec<Contexte>> {
    Ok(sqlx::query_as!(
        Contexte,
        "SELECT proposal_id, context, created_at
           FROM negotiation.glossary_proposal_authors
          WHERE proposal_id = ANY($1)
          ORDER BY created_at, client_ref",
        ids
    )
    .fetch_all(conn)
    .await?)
}

pub struct Proche {
    pub proposal_id: Uuid,
    pub entree: AdminProposalNearby,
}

/// Les entrées du lexique proches de chaque terme proposé, tous statuts, cinq
/// au plus (R5).
pub async fn proches(conn: &mut PgConnection, ids: &[Uuid]) -> Result<Vec<Proche>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id AS proposal_id, g.id AS "id!", g.slug AS "slug!", g.term AS "term!",
                  g.status AS "status!", g.similarite AS "similarity!"
             FROM negotiation.glossary_proposals p
             CROSS JOIN LATERAL (
                   SELECT e.id, e.slug, e.term, e.status::text AS status,
                          similarity(e.term_norm, p.term_norm) AS similarite
                     FROM negotiation.glossary_entries e
                    WHERE e.term_norm % p.term_norm
                      AND similarity(e.term_norm, p.term_norm) >= 0.4
                    ORDER BY similarite DESC, e.id
                    LIMIT 5) g
            WHERE p.id = ANY($1)
            ORDER BY p.id, g.similarite DESC, g.id"#,
        ids
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| Proche {
            proposal_id: l.proposal_id,
            entree: AdminProposalNearby {
                id: l.id,
                slug: l.slug,
                term: l.term,
                status: KnowledgeStatus::depuis(&l.status),
                similarity: l.similarity,
            },
        })
        .collect())
}

pub struct ATrancher {
    pub status: String,
    pub term: String,
}

pub async fn a_trancher(conn: &mut PgConnection, id: Uuid) -> Result<Option<ATrancher>> {
    Ok(sqlx::query_as!(
        ATrancher,
        r#"SELECT status::text AS "status!", term
             FROM negotiation.glossary_proposals WHERE id = $1 FOR UPDATE"#,
        id
    )
    .fetch_optional(conn)
    .await?)
}

pub async fn accepter(conn: &mut PgConnection, id: Uuid, entree: Uuid, expert: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.glossary_proposals
            SET status = 'accepted', glossary_entry_id = $2, handled_by = $3, handled_at = now()
          WHERE id = $1",
        id,
        entree,
        expert
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn rejeter(conn: &mut PgConnection, id: Uuid, motif: &str, expert: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.glossary_proposals
            SET status = 'rejected', rejection_reason = $2, handled_by = $3, handled_at = now()
          WHERE id = $1",
        id,
        motif,
        expert
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub struct APrevenir {
    pub proposal_id: Uuid,
    pub person_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub locale: String,
    pub term: String,
    pub slug: String,
}

/// Les auteurs des propositions acceptées d'où l'entrée est née.
pub async fn auteurs_a_prevenir(conn: &mut PgConnection, entree: Uuid) -> Result<Vec<APrevenir>> {
    Ok(sqlx::query_as!(
        APrevenir,
        r#"SELECT p.id AS proposal_id, a.person_id, pe.primary_email::text AS "email!",
                  pe.first_name, pe.preferred_locale AS "locale!", g.term, g.slug
             FROM negotiation.glossary_proposals p
             JOIN negotiation.glossary_entries g ON g.id = p.glossary_entry_id
             JOIN negotiation.glossary_proposal_authors a ON a.proposal_id = p.id
             JOIN identity.people pe ON pe.id = a.person_id
            WHERE p.glossary_entry_id = $1 AND p.status = 'accepted'
            ORDER BY p.id, a.created_at"#,
        entree
    )
    .fetch_all(conn)
    .await?)
}
