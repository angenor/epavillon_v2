//! Les visites de l'espace d'une organisation, et ce qui les éclaire.
//!
//! Lectures qui franchissent la frontière du schéma (`identity`, `programme`,
//! `platform`) : la question porte sur une entité d'`org`, ses membres.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

/// Note l'ouverture d'une page de l'espace. Seul un membre ACTIF est compté :
/// un administrateur de l'IFDD qui ouvre un dossier n'est pas l'organisation.
/// Pour un dossier, chaque organisation du dossier dont la personne est membre.
pub async fn noter(
    conn: &mut PgConnection,
    personne: Uuid,
    organisation: Option<Uuid>,
    dossier: Option<Uuid>,
) -> Result<u64> {
    let n = sqlx::query!(
        r#"INSERT INTO org.space_visits (organization_id, person_id, proposal_id)
           SELECT m.organization_id, m.person_id, $3
             FROM org.memberships m
            WHERE m.person_id = $1 AND m.status = 'active'
              AND m.organization_id IN (
                    SELECT $2::uuid WHERE $3::uuid IS NULL
                    UNION
                    SELECT p.organization_id FROM programme.proposals p
                     WHERE p.id = $3 AND p.deleted_at IS NULL
                    UNION
                    SELECT po.organization_id FROM programme.proposal_organizations po
                      JOIN programme.proposals p ON p.id = po.proposal_id AND p.deleted_at IS NULL
                     WHERE po.proposal_id = $3)
           ON CONFLICT ON CONSTRAINT ux_space_visits DO UPDATE
              SET last_at = now(), open_count = org.space_visits.open_count + 1"#,
        personne,
        organisation,
        dossier
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(n)
}

/// La première visite enregistrée, toutes organisations confondues.
pub async fn depuis<'e>(executor: impl PgExecutor<'e>) -> Result<Option<OffsetDateTime>> {
    let debut = sqlx::query_scalar!(r#"SELECT min(first_at) AS "debut" FROM org.space_visits"#)
        .fetch_one(executor)
        .await?;
    Ok(debut)
}

pub struct Membre {
    pub person_id: Uuid,
    pub display_name: String,
    pub role: String,
    pub active: bool,
    pub last_connection_at: Option<OffsetDateTime>,
    pub last_visit_at: Option<OffsetDateTime>,
    pub last_visit_reference: Option<String>,
    pub visit_days: i64,
    pub changes: i64,
    pub last_change_at: Option<OffsetDateTime>,
}

/// Les membres actifs, plus les anciens venus dans la fenêtre.
///
/// Les modifications comptées sont celles des schémas que l'espace modifie
/// (`programme`, `org`, `media`), une par requête : un enregistrement touche
/// plusieurs tables et reste un seul geste.
pub async fn membres(
    conn: &mut PgConnection,
    organisation: Uuid,
    fenetre: Option<OffsetDateTime>,
) -> Result<Vec<Membre>> {
    let lignes = sqlx::query!(
        r#"WITH gens AS (
               SELECT m.person_id, m.role::text AS role, true AS active
                 FROM org.memberships m
                WHERE m.organization_id = $1 AND m.status = 'active'
               UNION
               SELECT v.person_id, 'former', false
                 FROM org.space_visits v
                WHERE v.organization_id = $1 AND ($2::timestamptz IS NULL OR v.last_at >= $2)
                  AND NOT EXISTS (SELECT 1 FROM org.memberships m
                                   WHERE m.organization_id = $1 AND m.person_id = v.person_id
                                     AND m.status = 'active')
           )
           SELECT g.person_id AS "person_id!", p.display_name AS "display_name!",
                  g.role AS "role!", g.active AS "active!",
                  GREATEST(
                      (SELECT max(a.last_login_at) FROM identity.accounts a WHERE a.person_id = g.person_id),
                      (SELECT max(s.issued_at) FROM identity.sessions s
                        WHERE s.person_id = g.person_id AND s.client_kind = 'web')
                  ) AS "last_connection_at?",
                  dv.last_at AS "last_visit_at?", dv.reference_code AS "last_visit_reference?",
                  (SELECT count(DISTINCT v.visit_on) FROM org.space_visits v
                    WHERE v.organization_id = $1 AND v.person_id = g.person_id
                      AND ($2::timestamptz IS NULL OR v.last_at >= $2)) AS "visit_days!",
                  ch.n AS "changes!", ch.derniere AS "last_change_at?"
             FROM gens g
             JOIN identity.people p ON p.id = g.person_id
             LEFT JOIN LATERAL (
                 SELECT v.last_at, pr.reference_code
                   FROM org.space_visits v
                   LEFT JOIN programme.proposals pr ON pr.id = v.proposal_id
                  WHERE v.organization_id = $1 AND v.person_id = g.person_id
                  ORDER BY v.last_at DESC
                  LIMIT 1
             ) dv ON true
             CROSS JOIN LATERAL (
                 SELECT count(DISTINCT COALESCE(a.request_id, a.id::text)) AS n,
                        max(a.occurred_at) AS derniere
                   FROM platform.audit_log a
                  WHERE a.actor_id = g.person_id
                    AND a.entity_schema IN ('programme', 'org', 'media')
                    AND ($2::timestamptz IS NULL OR a.occurred_at >= $2)
             ) ch
            ORDER BY g.active DESC, p.display_name"#,
        organisation,
        fenetre
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Membre {
            person_id: l.person_id,
            display_name: l.display_name,
            role: l.role,
            active: l.active,
            last_connection_at: l.last_connection_at,
            last_visit_at: l.last_visit_at,
            last_visit_reference: l.last_visit_reference,
            visit_days: l.visit_days,
            changes: l.changes,
            last_change_at: l.last_change_at,
        })
        .collect())
}

pub struct Connexion {
    pub person_id: Uuid,
    pub debut: OffsetDateTime,
    pub fin: OffsetDateTime,
    pub agent: Option<String>,
}

/// Les moments passés sur le site. Une session tourne à chaque renouvellement
/// du jeton d'accès (15 min) tant qu'une page reste ouverte : des rotations
/// espacées de moins de 45 min forment un même moment.
pub async fn connexions(
    conn: &mut PgConnection,
    personnes: &[Uuid],
    fenetre: Option<OffsetDateTime>,
) -> Result<Vec<Connexion>> {
    let lignes = sqlx::query!(
        r#"WITH s AS (
               SELECT s.person_id, s.issued_at, s.user_agent,
                      CASE WHEN lag(s.issued_at) OVER w IS NULL
                             OR s.issued_at - lag(s.issued_at) OVER w > interval '45 minutes'
                           THEN 1 ELSE 0 END AS rupture
                 FROM identity.sessions s
                WHERE s.person_id = ANY($1) AND s.client_kind = 'web'
                  AND ($2::timestamptz IS NULL OR s.issued_at >= $2)
               WINDOW w AS (PARTITION BY s.person_id ORDER BY s.issued_at)
           ), ilots AS (
               SELECT s.*, sum(s.rupture) OVER (PARTITION BY s.person_id ORDER BY s.issued_at) AS ilot
                 FROM s
           )
           SELECT person_id AS "person_id!", min(issued_at) AS "debut!", max(issued_at) AS "fin!",
                  (array_agg(user_agent ORDER BY issued_at DESC))[1] AS "agent?"
             FROM ilots
            GROUP BY person_id, ilot
            ORDER BY 2 DESC
            LIMIT 500"#,
        personnes,
        fenetre
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Connexion {
            person_id: l.person_id,
            debut: l.debut,
            fin: l.fin,
            agent: l.agent,
        })
        .collect())
}

pub struct Visite {
    pub person_id: Uuid,
    pub first_at: OffsetDateTime,
    pub last_at: OffsetDateTime,
    pub open_count: i32,
    pub proposal_id: Option<Uuid>,
    pub reference: Option<String>,
    pub title: Option<serde_json::Value>,
}

pub async fn visites(
    conn: &mut PgConnection,
    organisation: Uuid,
    fenetre: Option<OffsetDateTime>,
) -> Result<Vec<Visite>> {
    let lignes = sqlx::query!(
        r#"SELECT v.person_id, v.first_at, v.last_at, v.open_count, v.proposal_id,
                  pr.reference_code AS "reference?", pr.title::jsonb AS "title?"
             FROM org.space_visits v
             LEFT JOIN programme.proposals pr ON pr.id = v.proposal_id
            WHERE v.organization_id = $1 AND ($2::timestamptz IS NULL OR v.last_at >= $2)
            ORDER BY v.first_at DESC
            LIMIT 1000"#,
        organisation,
        fenetre
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Visite {
            person_id: l.person_id,
            first_at: l.first_at,
            last_at: l.last_at,
            open_count: l.open_count,
            proposal_id: l.proposal_id,
            reference: l.reference,
            title: l.title,
        })
        .collect())
}

pub struct Modification {
    pub person_id: Uuid,
    pub at: OffsetDateTime,
    pub request_id: Option<String>,
    pub tables: Vec<String>,
    pub proposal_id: Option<Uuid>,
    pub reference: Option<String>,
    pub title: Option<serde_json::Value>,
}

/// Les modifications, une par requête. Le dossier se lit sur la ligne du
/// dossier lui-même ou sur la colonne `proposal_id` d'une ligne fille.
pub async fn modifications(
    conn: &mut PgConnection,
    personnes: &[Uuid],
    fenetre: Option<OffsetDateTime>,
) -> Result<Vec<Modification>> {
    let lignes = sqlx::query!(
        r#"WITH gestes AS (
               SELECT a.actor_id, min(a.occurred_at) AS at, min(a.request_id) AS request_id,
                      array_agg(DISTINCT a.entity_table) AS tables,
                      (array_agg(x.proposal_id) FILTER (WHERE x.proposal_id IS NOT NULL))[1] AS proposal_id
                 FROM platform.audit_log a
                CROSS JOIN LATERAL (
                    SELECT CASE
                        WHEN a.entity_schema = 'programme' AND a.entity_table = 'proposals' THEN a.entity_id
                        WHEN a.entity_schema = 'programme' THEN
                            COALESCE(a.new_data->>'proposal_id', a.old_data->>'proposal_id')::uuid
                    END AS proposal_id
                ) x
                WHERE a.actor_id = ANY($1)
                  AND a.entity_schema IN ('programme', 'org', 'media')
                  AND ($2::timestamptz IS NULL OR a.occurred_at >= $2)
                GROUP BY a.actor_id, COALESCE(a.request_id, a.id::text)
           )
           SELECT g.actor_id AS "person_id!", g.at AS "at!", g.request_id,
                  g.tables AS "tables!", g.proposal_id AS "proposal_id?",
                  pr.reference_code AS "reference?", pr.title::jsonb AS "title?"
             FROM gestes g
             LEFT JOIN programme.proposals pr ON pr.id = g.proposal_id
            ORDER BY g.at DESC
            LIMIT 500"#,
        personnes,
        fenetre
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Modification {
            person_id: l.person_id,
            at: l.at,
            request_id: l.request_id,
            tables: l.tables,
            proposal_id: l.proposal_id,
            reference: l.reference,
            title: l.title,
        })
        .collect())
}

pub struct Refus {
    pub person_id: Uuid,
    pub at: OffsetDateTime,
    pub route: String,
    pub status: i16,
    pub error_code: Option<String>,
    pub request_id: Option<String>,
}

/// Les refus et pannes vus par ces personnes, depuis le journal d'exploitation.
pub async fn refus(
    conn: &mut PgConnection,
    personnes: &[Uuid],
    fenetre: Option<OffsetDateTime>,
) -> Result<Vec<Refus>> {
    let lignes = sqlx::query!(
        r#"SELECT o.actor_id AS "person_id!", o.occurred_at AS "at!", o.route AS "route!",
                  o.status AS "status!", o.error_code, o.request_id
             FROM platform.ops_log o
            WHERE o.actor_id = ANY($1)
              AND o.kind IN ('http_refused', 'http_error')
              AND ($2::timestamptz IS NULL OR o.occurred_at >= $2)
            ORDER BY o.occurred_at DESC
            LIMIT 300"#,
        personnes,
        fenetre
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Refus {
            person_id: l.person_id,
            at: l.at,
            route: l.route,
            status: l.status,
            error_code: l.error_code,
            request_id: l.request_id,
        })
        .collect())
}
