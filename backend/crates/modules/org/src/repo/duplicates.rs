//! La file des doublons présumés : l'enregistrement d'une paire, ses deux
//! sections, et les décisions.
//!
//! **`LEAST`/`GREATEST` tiennent l'ordre que la base impose**
//! (`ck_duplicate_candidates_ordered`) : sans eux, une paire sur deux serait
//! refusée par une vérification, et le message parlerait d'une contrainte au
//! lieu de dire quoi que ce soit d'utile.

use kernel::error::{ApiError, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::duplicates::{DuplicatePair, DuplicateSide};
use crate::domain::ids::{DuplicatePairId, OrganizationId, PersonId};

/// Enregistre une paire suspecte.
///
/// **Une seule ligne tient les deux moitiés de FR-059** : une paire déjà
/// arbitrée n'est pas ressuscitée — la clause `WHERE` de la mise à jour l'exclut
/// —, une paire en attente est mise à jour.
pub async fn consigner(
    conn: &mut PgConnection,
    une: OrganizationId,
    autre: OrganizationId,
    score: f64,
    motifs: &[String],
) -> Result<bool> {
    let ecrite = sqlx::query!(
        "INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
         VALUES (LEAST($1::uuid, $2::uuid), GREATEST($1::uuid, $2::uuid), $3::float8::numeric(5,1), $4)
         ON CONFLICT (left_id, right_id) DO UPDATE
            SET score = EXCLUDED.score,
                reasons = EXCLUDED.reasons,
                detected_at = now()
          WHERE org.duplicate_candidates.reviewed_at IS NULL",
        une.as_uuid(),
        autre.as_uuid(),
        score,
        motifs
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(ecrite == 1)
}

/// Les paires **non arbitrées**, triées par similarité décroissante.
pub async fn en_attente(pool: &PgPool) -> Result<Vec<DuplicatePair>> {
    paires(pool, true).await
}

/// Les paires **déjà tranchées** : fusionnées, écartées, reportées. Elles ne
/// disparaissent pas — « ce ne sont pas des doublons » se reprend.
pub async fn tranchees(pool: &PgPool) -> Result<Vec<DuplicatePair>> {
    paires(pool, false).await
}

/// Les paires ouvertes où une fiche apparaît : le lien de sa fiche vers l'écran
/// de fusion.
pub async fn ouvertes_pour(
    conn: &mut PgConnection,
    organisation: OrganizationId,
) -> Result<Vec<DuplicatePair>> {
    let entetes = sqlx::query_as!(
        Entete,
        r#"SELECT d.id, d.left_id, d.right_id, d.score::float8 AS "score!",
                  d.reasons AS "reasons!", d.detected_at, d.reviewed_at, d.reviewed_by,
                  p.display_name AS "reviewed_by_name?", d.decision
             FROM org.duplicate_candidates d
             LEFT JOIN identity.people p ON p.id = d.reviewed_by
            WHERE (d.left_id = $1 OR d.right_id = $1) AND d.reviewed_at IS NULL
            ORDER BY d.score DESC"#,
        organisation.as_uuid()
    )
    .fetch_all(&mut *conn)
    .await?;

    assembler(conn, entetes).await
}

pub async fn par_identifiant(
    conn: &mut PgConnection,
    id: DuplicatePairId,
) -> Result<Option<DuplicatePair>> {
    let entetes = sqlx::query_as!(
        Entete,
        r#"SELECT d.id, d.left_id, d.right_id, d.score::float8 AS "score!",
                  d.reasons AS "reasons!", d.detected_at, d.reviewed_at, d.reviewed_by,
                  p.display_name AS "reviewed_by_name?", d.decision
             FROM org.duplicate_candidates d
             LEFT JOIN identity.people p ON p.id = d.reviewed_by
            WHERE d.id = $1"#,
        id.as_uuid()
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(assembler(conn, entetes).await?.into_iter().next())
}

/// Pose une décision sur une paire. `merged` n'est **jamais** posé ici : c'est
/// `org.merge_organizations()` qui l'écrit, et elle seule.
pub async fn trancher(
    conn: &mut PgConnection,
    id: DuplicatePairId,
    decision: &str,
    qui: PersonId,
) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE org.duplicate_candidates
            SET decision = $2, reviewed_at = now(), reviewed_by = $3
          WHERE id = $1",
        id.as_uuid(),
        decision,
        qui.as_uuid()
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Remet une paire sortie de la file **en circulation**.
///
/// Le tri de l'écran se fait sur `reviewed_at`, pas sur `decision` : une paire
/// reste hors de la file tant que cette date est posée. La remise en
/// circulation efface donc **les trois colonnes**, sans quoi le bouton dirait
/// le contraire de ce qu'il fait — c'est le défaut relevé le 20/08 sur les
/// paires écartées, que seul un `deferred` savait alors reprendre.
///
/// `merged` en est exclue : la fusion a eu lieu, on ne la rouvre pas.
pub async fn remettre_en_circulation(conn: &mut PgConnection, id: DuplicatePairId) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE org.duplicate_candidates
            SET decision = NULL, reviewed_at = NULL, reviewed_by = NULL
          WHERE id = $1 AND reviewed_at IS NOT NULL AND decision IS DISTINCT FROM 'merged'",
        id.as_uuid()
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// Les fiches vivantes d'une tranche du balayage, curseur compris.
///
/// L'ordre est celui de l'identifiant : il est stable, et `uuid_v7` le rend
/// chronologique — une fiche créée pendant le balayage entre à la fin, jamais au
/// milieu d'une tranche déjà passée.
pub async fn tranche_a_balayer(
    pool: &PgPool,
    apres: Option<Uuid>,
    taille: i64,
) -> Result<Vec<FicheABalayer>> {
    let lignes = sqlx::query_as!(
        FicheABalayer,
        r#"SELECT o.id, o.legal_name, o.country_id,
                  o.contact_email::text AS "contact_email?",
                  o.website::text AS "website?"
             FROM org.organizations o
            WHERE o.status IN ('candidate', 'active')
              AND ($1::uuid IS NULL OR o.id > $1)
            ORDER BY o.id
            LIMIT $2"#,
        apres,
        taille
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes)
}

/// Ce dont le balayage a besoin pour interroger la recherche : les quatre
/// signaux que la fonction du modèle attend.
pub struct FicheABalayer {
    pub id: Uuid,
    pub legal_name: String,
    pub country_id: Option<Uuid>,
    pub contact_email: Option<String>,
    pub website: Option<String>,
}

// -----------------------------------------------------------------------------

async fn paires(pool: &PgPool, en_attente: bool) -> Result<Vec<DuplicatePair>> {
    let mut conn = pool.acquire().await?;

    let entetes = if en_attente {
        sqlx::query_as!(
            Entete,
            r#"SELECT d.id, d.left_id, d.right_id, d.score::float8 AS "score!",
                      d.reasons AS "reasons!", d.detected_at, d.reviewed_at, d.reviewed_by,
                      p.display_name AS "reviewed_by_name?", d.decision
                 FROM org.duplicate_candidates d
                 LEFT JOIN identity.people p ON p.id = d.reviewed_by
                WHERE d.reviewed_at IS NULL
                ORDER BY d.score DESC, d.detected_at DESC"#
        )
        .fetch_all(&mut *conn)
        .await?
    } else {
        sqlx::query_as!(
            Entete,
            r#"SELECT d.id, d.left_id, d.right_id, d.score::float8 AS "score!",
                      d.reasons AS "reasons!", d.detected_at, d.reviewed_at, d.reviewed_by,
                      p.display_name AS "reviewed_by_name?", d.decision
                 FROM org.duplicate_candidates d
                 LEFT JOIN identity.people p ON p.id = d.reviewed_by
                WHERE d.reviewed_at IS NOT NULL
                ORDER BY d.reviewed_at DESC"#
        )
        .fetch_all(&mut *conn)
        .await?
    };

    assembler(&mut conn, entetes).await
}

/// Les deux fiches d'une paire, réduites à ce qui permet de trancher.
///
/// Elles sont lues **en une fois pour toute la file** : une file de cinquante
/// paires ferait sinon cent lectures.
async fn assembler(conn: &mut PgConnection, entetes: Vec<Entete>) -> Result<Vec<DuplicatePair>> {
    if entetes.is_empty() {
        return Ok(Vec::new());
    }

    let mut ids: Vec<Uuid> = Vec::with_capacity(entetes.len() * 2);
    for e in &entetes {
        ids.push(e.left_id);
        ids.push(e.right_id);
    }

    let cotes = cotes_de(conn, &ids).await?;

    entetes
        .into_iter()
        .map(|e| {
            let gauche = cotes
                .iter()
                .find(|c| c.organization_id.as_uuid() == e.left_id)
                .cloned()
                .ok_or_else(|| ApiError::internal("fiche gauche d'une paire introuvable"))?;
            let droite = cotes
                .iter()
                .find(|c| c.organization_id.as_uuid() == e.right_id)
                .cloned()
                .ok_or_else(|| ApiError::internal("fiche droite d'une paire introuvable"))?;

            Ok(DuplicatePair {
                id: DuplicatePairId(e.id),
                score: e.score,
                reasons: e.reasons,
                detected_at: e.detected_at,
                reviewed_at: e.reviewed_at,
                reviewed_by: e.reviewed_by.map(PersonId),
                reviewed_by_name: e.reviewed_by_name,
                decision: e.decision,
                left: Box::new(gauche),
                right: Box::new(droite),
            })
        })
        .collect()
}

/// Les fiches réduites, pour la file comme pour l'aperçu de fusion.
pub async fn cotes_de(conn: &mut PgConnection, ids: &[Uuid]) -> Result<Vec<DuplicateSide>> {
    let lignes = sqlx::query!(
        r#"SELECT o.id AS "id!", o.legal_name AS "legal_name!", o.acronym,
                  o.slug::text AS "slug!",
                  o.status::text AS "statut!",
                  o.organization_type_code AS "organization_type_code!",
                  t.label::jsonb AS "type_label?",
                  o.country_id, c.name::jsonb AS "country_name?", o.city,
                  o.website::text AS "website?",
                  o.contact_email::text AS "contact_email?",
                  o.verified_at, o.trust_score AS "trust_score!",
                  o.created_at AS "created_at!",
                  cr.display_name AS "created_by_name?",
                  (SELECT count(*) FROM org.memberships m
                    WHERE m.organization_id = o.id AND m.status = 'active') AS "member_count!",
                  (SELECT count(*) FROM programme.proposals p
                    WHERE p.organization_id = o.id AND p.deleted_at IS NULL) AS "proposal_count!",
                  (SELECT count(*) FROM programme.sessions s
                    WHERE s.organization_id = o.id) AS "session_count!",
                  COALESCE(
                      (SELECT array_agg(d.domain ORDER BY d.domain)
                         FROM org.organization_domains d
                        WHERE d.organization_id = o.id),
                      '{}') AS "domains!"
             FROM org.organizations o
             LEFT JOIN reference.taxonomy_terms t
                    ON t.taxonomy_code = 'organization_type' AND t.code = o.organization_type_code
             LEFT JOIN reference.countries c ON c.id = o.country_id
             LEFT JOIN identity.people cr ON cr.id = o.created_by
            WHERE o.id = ANY($1)"#,
        ids
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| DuplicateSide {
            organization_id: OrganizationId(l.id),
            legal_name: l.legal_name,
            acronym: l.acronym,
            slug: l.slug,
            status: l.statut,
            organization_type_code: l.organization_type_code,
            organization_type_label: l.type_label,
            country_id: l.country_id,
            country_name: l.country_name,
            city: l.city,
            website: l.website,
            contact_email: l.contact_email,
            verified_at: l.verified_at,
            trust_score: l.trust_score,
            member_count: l.member_count,
            proposal_count: l.proposal_count,
            session_count: l.session_count,
            domains: l.domains,
            created_at: l.created_at,
            created_by_name: l.created_by_name,
        })
        .collect())
}

struct Entete {
    id: Uuid,
    left_id: Uuid,
    right_id: Uuid,
    score: f64,
    reasons: Vec<String>,
    detected_at: time::OffsetDateTime,
    reviewed_at: Option<time::OffsetDateTime>,
    reviewed_by: Option<Uuid>,
    reviewed_by_name: Option<String>,
    decision: Option<String>,
}

/// Le curseur du balayage se lit dans la charge utile du travail ; ce type ne
/// sert qu'à le typer à la frontière.
pub type Curseur = Option<Uuid>;

/// Les motifs d'une paire, tels que la recherche les a posés. Ils voyagent en
/// texte : le front en connaît quatre valeurs, et en figer un énuméré ferait
/// échouer la lecture le jour où la fonction du modèle en ajoute une.
pub fn motifs(valeur: &Value) -> Vec<String> {
    valeur
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(|m| m.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
