//! L'audit de toute la plateforme (`platform.audit_log`), valeurs sensibles
//! retirées par `platform.audit_is_sensitive` : le nom du champ reste, jamais
//! sa valeur.

use actix_web::{web, HttpResponse};
use kernel::auth::Requires;
use kernel::db::Db;
use kernel::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    debut_de_retention, invalide, motif, pagination, recherche, JournalPerson, JournalRead, Page,
    Period,
};

const ACTIONS: [&str; 3] = ["insert", "update", "delete"];

#[derive(Debug, Serialize)]
pub struct JournalChange {
    pub id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    pub actor: Option<JournalPerson>,
    pub request_id: Option<String>,
    pub entity_schema: String,
    pub entity_table: String,
    pub entity_id: Option<Uuid>,
    pub action: String,
    /// `JournalChangeField[]`, composé en base.
    pub fields: Value,
}

#[derive(Debug, Deserialize)]
pub struct ChangeQuery {
    pub period: Option<Period>,
    /// `schema.table`.
    pub entity: Option<String>,
    pub action: Option<String>,
    pub q: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

struct Filtre {
    depuis: OffsetDateTime,
    schema: Option<String>,
    table: Option<String>,
    action: Option<String>,
    q: Option<String>,
    request_id: Option<String>,
    limite: i64,
    decalage: i64,
}

async fn lire(pool: &PgPool, f: &Filtre) -> Result<(Vec<JournalChange>, i64)> {
    let q_motif = f.q.as_deref().map(motif);
    let lignes = sqlx::query!(
        r#"WITH page AS (
               SELECT a.id, a.occurred_at, a.actor_id, a.actor_label, a.request_id,
                      a.entity_schema, a.entity_table, a.entity_id, a.action,
                      a.changed_fields, a.old_data, a.new_data,
                      count(*) OVER () AS total
                 FROM platform.audit_log a
                WHERE a.occurred_at >= $1
                  AND ($2::text IS NULL OR (a.entity_schema = $2 AND a.entity_table = $3))
                  AND ($4::text IS NULL OR a.action = $4)
                  AND ($5::text IS NULL
                       OR a.actor_label ILIKE '%' || $6 || '%'
                       OR a.entity_id::text = $5)
                  AND ($7::text IS NULL OR a.request_id = $7)
                ORDER BY a.occurred_at DESC, a.id DESC
                LIMIT $8 OFFSET $9
           )
           SELECT page.id AS "id!", page.occurred_at AS "occurred_at!",
                  page.actor_id AS "actor_id?", page.actor_label AS "actor_label?",
                  g.nom AS "organization_name?", page.request_id AS "request_id?",
                  page.entity_schema AS "entity_schema!", page.entity_table AS "entity_table!",
                  page.entity_id AS "entity_id?", page.action AS "action!",
                  COALESCE((
                      SELECT jsonb_agg(jsonb_build_object(
                                 'name', f.champ,
                                 'masked', s.masque,
                                 'before', CASE WHEN s.masque THEN NULL ELSE
                                     CASE WHEN jsonb_typeof(page.old_data -> f.champ) = 'string'
                                               AND length(page.old_data ->> f.champ) > 300
                                          THEN to_jsonb(left(page.old_data ->> f.champ, 300) || '…')
                                          ELSE page.old_data -> f.champ END END,
                                 'after', CASE WHEN s.masque THEN NULL ELSE
                                     CASE WHEN jsonb_typeof(page.new_data -> f.champ) = 'string'
                                               AND length(page.new_data ->> f.champ) > 300
                                          THEN to_jsonb(left(page.new_data ->> f.champ, 300) || '…')
                                          ELSE page.new_data -> f.champ END END
                             ) ORDER BY f.champ)
                        FROM unnest(page.changed_fields) AS f(champ)
                        CROSS JOIN LATERAL (
                             SELECT platform.audit_is_sensitive(page.entity_schema, page.entity_table, f.champ) AS masque
                        ) s
                       WHERE f.champ NOT IN ('updated_at', 'search_vector')
                  ), '[]'::jsonb) AS "fields!",
                  page.total AS "total!"
             FROM page
             LEFT JOIN LATERAL (
                  SELECT COALESCE(o.acronym, o.legal_name) AS nom
                    FROM org.memberships m
                    JOIN org.organizations o ON o.id = m.organization_id
                   WHERE m.person_id = page.actor_id AND m.status = 'active'
                   ORDER BY m.is_primary DESC, m.approved_at NULLS LAST
                   LIMIT 1
             ) g ON true
            ORDER BY page.occurred_at DESC, page.id DESC"#,
        f.depuis,
        f.schema,
        f.table,
        f.action,
        f.q,
        q_motif,
        f.request_id,
        f.limite,
        f.decalage
    )
    .fetch_all(pool)
    .await?;

    let total = lignes.first().map_or(0, |l| l.total);
    let modifs = lignes
        .into_iter()
        .map(|l| JournalChange {
            actor: l.actor_id.map(|id| JournalPerson {
                id,
                display_name: l.actor_label.unwrap_or_default(),
                organization_name: l.organization_name,
            }),
            id: l.id,
            occurred_at: l.occurred_at,
            request_id: l.request_id,
            entity_schema: l.entity_schema,
            entity_table: l.entity_table,
            entity_id: l.entity_id,
            action: l.action,
            fields: l.fields,
        })
        .collect();
    Ok((modifs, total))
}

/// Les modifications d'une demande, les plus récentes d'abord.
pub(super) async fn de_la_demande(
    pool: &PgPool,
    request_id: &str,
    limite: i64,
) -> Result<Vec<JournalChange>> {
    let filtre = Filtre {
        depuis: debut_de_retention(),
        schema: None,
        table: None,
        action: None,
        q: None,
        request_id: Some(request_id.to_owned()),
        limite,
        decalage: 0,
    };
    Ok(lire(pool, &filtre).await?.0)
}

/// `schema.table` : deux identifiants en minuscules.
fn objet(entity: &str) -> Option<(String, String)> {
    let (schema, table) = entity.split_once('.')?;
    let valide = |s: &str| {
        !s.is_empty()
            && s.len() <= 63
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    (valide(schema) && valide(table)).then(|| (schema.to_owned(), table.to_owned()))
}

#[utoipa::path(
    get,
    description = "`JournalChangePage` : l'audit de toute la plateforme, le plus récent d'abord. Les valeurs des champs sensibles (identité personnelle, secrets, jetons, empreintes) ne sortent jamais : le champ est nommé avec `masked: true`. Filtres : `period`, `entity` (`schema.table`), `action` (insert, update, delete), `q` (auteur ou identifiant de l'objet).",
    path = "/admin/journal/changes",
    tag = "Journal",
    operation_id = "admin_journal_modifications",
    params(
        ("period" = Option<String>, Query, description = "24h, 7d, 30d ou 90d"),
        ("entity" = Option<String>, Query, description = "schema.table"),
        ("action" = Option<String>, Query, description = "insert, update ou delete"),
        ("q" = Option<String>, Query, description = "Auteur ou identifiant"),
        ("page" = Option<i64>, Query, description = "Dès 1"),
        ("per_page" = Option<i64>, Query, description = "200 au plus"),
    ),
    responses(
        (status = 200, description = "JournalChangePage", body = Object),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`identity.platform.journal` absente en portée globale", body = crate::openapi::ApiErrorBody),
        (status = 422, description = "Filtre inconnu", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn modifications(
    _droit: Requires<JournalRead>,
    db: web::Data<Db>,
    requete: web::Query<ChangeQuery>,
) -> Result<HttpResponse> {
    let (schema, table) = match requete.entity.as_deref().filter(|e| !e.is_empty()) {
        None => (None, None),
        Some(e) => {
            let (s, t) = objet(e).ok_or_else(|| invalide("entity"))?;
            (Some(s), Some(t))
        }
    };
    let action = requete.action.clone().filter(|a| !a.is_empty());
    if action.as_deref().is_some_and(|a| !ACTIONS.contains(&a)) {
        return Err(invalide("action"));
    }
    let (page, per_page, decalage) = pagination(requete.page, requete.per_page);
    let filtre = Filtre {
        depuis: requete.period.unwrap_or_default().debut(),
        schema,
        table,
        action,
        q: recherche(requete.q.as_deref()),
        request_id: None,
        limite: per_page,
        decalage,
    };
    let (items, total) = lire(db.pool(), &filtre).await?;
    Ok(HttpResponse::Ok().json(Page {
        items,
        total,
        page,
        per_page,
    }))
}

#[cfg(test)]
mod tests {
    use super::objet;

    #[test]
    fn objet_schema_point_table() {
        assert_eq!(
            objet("programme.proposals"),
            Some(("programme".into(), "proposals".into()))
        );
        assert_eq!(objet("programme"), None);
        assert_eq!(objet("x.y; drop"), None);
    }
}
