//! L'assistance (02/11) : qui y a droit, sur quels dossiers, et ses sujets.
//! Lecture n° 22 du tableau : `org.memberships` (l'accès d'un membre à un
//! dossier, la même règle que `acces_au_dossier`), `org.organizations` (le nom
//! court de l'organisation d'un sujet), `identity.people` (`display_name`
//! seul), `reference.taxonomy_terms` (le code de la nature).
//!
//! Ce que chacun lit est décidé par `programme.support_visible_comments()`,
//! en base : la liste, son compteur et la lecture ne peuvent pas diverger.

use kernel::error::Result;
use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

/// Où se tient la personne dans l'assistance, et ce que cela lui ouvre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Equipe { globale: bool, editions: Vec<Uuid> },
    Organisation,
}

impl Place {
    fn parametres(&self) -> (bool, bool, Vec<Uuid>) {
        match self {
            Place::Equipe { globale, editions } => (true, *globale, editions.clone()),
            Place::Organisation => (false, false, Vec::new()),
        }
    }
}

/// Un dossier sur lequel la personne peut ouvrir un sujet.
pub struct DossierSuivi {
    pub id: Uuid,
    pub reference_code: String,
    pub title: Value,
    pub organization_id: Uuid,
}

/// Les dossiers que cette personne suit pour son organisation, dans ces états.
pub async fn dossiers_suivis<'e>(
    executor: impl PgExecutor<'e>,
    personne: Uuid,
    etats: &[&str],
) -> Result<Vec<DossierSuivi>> {
    let etats: Vec<String> = etats.iter().map(|e| (*e).to_owned()).collect();
    let lignes = sqlx::query!(
        r#"SELECT d.id, d.reference_code AS "reference_code!", d.title AS "title!: Value",
                  d.organization_id
             FROM programme.proposals d
            WHERE d.deleted_at IS NULL AND d.status::text = ANY($2)
              AND EXISTS (SELECT 1 FROM org.memberships m
                           WHERE m.organization_id = d.organization_id AND m.person_id = $1
                             AND (m.status = 'active'
                                  OR (d.submitted_by = $1 AND m.status = 'pending'
                                      AND m.invited_at IS NULL)))
            ORDER BY d.submitted_at DESC NULLS LAST, d.reference_code"#,
        personne,
        &etats
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| DossierSuivi {
            id: l.id,
            reference_code: l.reference_code,
            title: l.title,
            organization_id: l.organization_id,
        })
        .collect())
}

/// Un message de l'assistance, avec ce qui le situe.
pub struct LigneDAssistance {
    pub id: Uuid,
    pub proposal_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub author_id: Uuid,
    pub author_name: String,
    pub from_team: bool,
    pub body: String,
    pub is_change_request: bool,
    pub resolved_at: Option<OffsetDateTime>,
    pub edited_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub topic: Option<String>,
    pub topic_other: Option<String>,
    pub subject: Option<String>,
    pub closed_at: Option<OffsetDateTime>,
    pub closed_by: Option<Uuid>,
    pub is_read: bool,
    pub reference_code: Option<String>,
    pub proposal_title: Option<Value>,
    pub organization_name: Option<String>,
}

/// Tous les messages de l'assistance que cette personne lit, du plus ancien au
/// plus récent. L'organisation d'un sujet est celle du dossier, ou, sans
/// dossier, l'adhésion active principale de son auteur.
pub async fn messages<'e>(
    executor: impl PgExecutor<'e>,
    personne: Uuid,
    place: &Place,
) -> Result<Vec<LigneDAssistance>> {
    let (equipe, globale, editions) = place.parametres();
    let lignes = sqlx::query!(
        r#"SELECT c.id, c.proposal_id, c.parent_id, c.author_id,
                  a.display_name AS "author_name!",
                  CASE WHEN v.platform_author IS NOT NULL THEN c.author_id <> v.platform_author
                       ELSE NOT EXISTS (SELECT 1 FROM org.memberships m
                                         WHERE m.organization_id = d.organization_id
                                           AND m.person_id = c.author_id
                                           AND m.status IN ('active', 'pending'))
                  END AS "from_team!",
                  c.body, c.is_change_request, c.resolved_at, c.edited_at, c.created_at,
                  (SELECT t.code FROM reference.taxonomy_terms t WHERE t.id = c.topic_term_id) AS topic,
                  c.topic_other, c.subject, c.closed_at, c.closed_by,
                  (c.author_id = $1 OR EXISTS (SELECT 1 FROM programme.proposal_comment_reads r
                                                WHERE r.comment_id = c.id AND r.person_id = $1)) AS "is_read!",
                  d.reference_code AS "reference_code?",
                  d.title AS "proposal_title?: Value",
                  CASE WHEN c.parent_id IS NULL THEN
                       COALESCE((SELECT COALESCE(o.acronym, o.legal_name) FROM org.organizations o
                                  WHERE o.id = d.organization_id),
                                (SELECT COALESCE(o.acronym, o.legal_name)
                                   FROM org.memberships m JOIN org.organizations o ON o.id = m.organization_id
                                  WHERE m.person_id = c.author_id AND m.status = 'active'
                                  ORDER BY m.is_primary DESC, m.created_at LIMIT 1))
                  END AS organization_name
             FROM programme.support_visible_comments($1, $2, $3, $4) v
             JOIN programme.proposal_comments c ON c.id = v.comment_id
             JOIN identity.people a ON a.id = c.author_id
             LEFT JOIN programme.proposals d ON d.id = c.proposal_id
            ORDER BY c.created_at, c.id"#,
        personne,
        equipe,
        globale,
        &editions
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| LigneDAssistance {
            id: l.id,
            proposal_id: l.proposal_id,
            parent_id: l.parent_id,
            author_id: l.author_id,
            author_name: l.author_name,
            from_team: l.from_team,
            body: l.body,
            is_change_request: l.is_change_request,
            resolved_at: l.resolved_at,
            edited_at: l.edited_at,
            created_at: l.created_at,
            topic: l.topic,
            topic_other: l.topic_other,
            subject: l.subject,
            closed_at: l.closed_at,
            closed_by: l.closed_by,
            is_read: l.is_read,
            reference_code: l.reference_code,
            proposal_title: l.proposal_title,
            organization_name: l.organization_name,
        })
        .collect())
}

/// Les messages de l'assistance que cette personne n'a ni écrits ni lus.
pub async fn non_lus<'e>(
    executor: impl PgExecutor<'e>,
    personne: Uuid,
    place: &Place,
) -> Result<i64> {
    let (equipe, globale, editions) = place.parametres();
    Ok(sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!"
             FROM programme.support_visible_comments($1, $2, $3, $4) v
             JOIN programme.proposal_comments c ON c.id = v.comment_id
            WHERE c.author_id <> $1
              AND NOT EXISTS (SELECT 1 FROM programme.proposal_comment_reads r
                               WHERE r.comment_id = c.id AND r.person_id = $1)"#,
        personne,
        equipe,
        globale,
        &editions
    )
    .fetch_one(executor)
    .await?)
}

/// Ce message est-il dans ce que cette personne lit de l'assistance ?
pub async fn lisible<'e>(
    executor: impl PgExecutor<'e>,
    personne: Uuid,
    place: &Place,
    message: Uuid,
) -> Result<bool> {
    let (equipe, globale, editions) = place.parametres();
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM programme.support_visible_comments($1, $2, $3, $4) v
                           WHERE v.comment_id = $5) AS "lisible!""#,
        personne,
        equipe,
        globale,
        &editions,
        message
    )
    .fetch_one(executor)
    .await?)
}

/// Marquer lus ceux de ces messages que la personne lit et n'a pas écrits.
pub async fn marquer_lus(
    conn: &mut sqlx::PgConnection,
    personne: Uuid,
    place: &Place,
    messages: &[Uuid],
) -> Result<u64> {
    let (equipe, globale, editions) = place.parametres();
    let poses = sqlx::query!(
        "INSERT INTO programme.proposal_comment_reads (comment_id, person_id)
         SELECT c.id, $1
           FROM programme.support_visible_comments($1, $2, $3, $4) v
           JOIN programme.proposal_comments c ON c.id = v.comment_id
          WHERE c.id = ANY($5) AND c.author_id <> $1
         ON CONFLICT DO NOTHING",
        personne,
        equipe,
        globale,
        &editions,
        messages
    )
    .execute(conn)
    .await?;

    Ok(poses.rows_affected())
}
