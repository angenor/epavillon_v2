//! `engagement.mail_templates` (29/10). Les images se lisent par
//! `media.attached_image()`, la fonction du module media, jamais par sa jointure.

use kernel::error::Result;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct LigneGabarit {
    pub id: Uuid,
    pub name: String,
    pub layout: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub gradient: bool,
    pub event_id: Option<Uuid>,
    pub preselect_for_submitters: bool,
    pub created_by_name: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub background: Option<serde_json::Value>,
    pub logo: Option<serde_json::Value>,
}

/// Les gabarits utilisables pour une édition : les siens et les communs.
pub async fn lister(pool: &PgPool, event_id: Uuid) -> Result<Vec<LigneGabarit>> {
    let lignes = sqlx::query!(
        r#"SELECT t.id, t.name, t.layout, t.title, t.subtitle, t.gradient, t.event_id,
                  t.preselect_for_submitters,
                  (SELECT p.display_name FROM identity.people p WHERE p.id = t.created_by) AS created_by_name,
                  t.created_at, t.updated_at,
                  media.attached_image('engagement', 'mail_templates', t.id, 'background') AS background,
                  media.attached_image('engagement', 'mail_templates', t.id, 'logo') AS logo
             FROM engagement.mail_templates t
            WHERE t.event_id IS NULL OR t.event_id = $1
            ORDER BY t.event_id NULLS LAST, lower(t.name)"#,
        event_id
    )
    .fetch_all(pool)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| LigneGabarit {
            id: l.id,
            name: l.name,
            layout: l.layout,
            title: l.title,
            subtitle: l.subtitle,
            gradient: l.gradient,
            event_id: l.event_id,
            preselect_for_submitters: l.preselect_for_submitters,
            created_by_name: l.created_by_name,
            created_at: l.created_at,
            updated_at: l.updated_at,
            background: l.background,
            logo: l.logo,
        })
        .collect())
}

pub async fn lire(pool: &PgPool, id: Uuid) -> Result<Option<LigneGabarit>> {
    let l = sqlx::query!(
        r#"SELECT t.id, t.name, t.layout, t.title, t.subtitle, t.gradient, t.event_id,
                  t.preselect_for_submitters,
                  (SELECT p.display_name FROM identity.people p WHERE p.id = t.created_by) AS created_by_name,
                  t.created_at, t.updated_at,
                  media.attached_image('engagement', 'mail_templates', t.id, 'background') AS background,
                  media.attached_image('engagement', 'mail_templates', t.id, 'logo') AS logo
             FROM engagement.mail_templates t
            WHERE t.id = $1"#,
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(l.map(|l| LigneGabarit {
        id: l.id,
        name: l.name,
        layout: l.layout,
        title: l.title,
        subtitle: l.subtitle,
        gradient: l.gradient,
        event_id: l.event_id,
        preselect_for_submitters: l.preselect_for_submitters,
        created_by_name: l.created_by_name,
        created_at: l.created_at,
        updated_at: l.updated_at,
        background: l.background,
        logo: l.logo,
    }))
}

pub struct ValeursGabarit<'a> {
    pub name: &'a str,
    pub layout: &'a str,
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub gradient: bool,
    pub event_id: Option<Uuid>,
    pub preselect_for_submitters: bool,
}

/// Un autre gabarit de l'édition est-il déjà présélectionné ?
pub async fn autre_preselectionne(
    pool: &PgPool,
    event_id: Uuid,
    sauf: Option<Uuid>,
) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM engagement.mail_templates
                           WHERE event_id = $1 AND preselect_for_submitters
                             AND ($2::uuid IS NULL OR id <> $2)) AS "existe!""#,
        event_id,
        sauf
    )
    .fetch_one(pool)
    .await?;
    Ok(existe)
}

pub async fn creer(conn: &mut PgConnection, acteur: Uuid, v: &ValeursGabarit<'_>) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO engagement.mail_templates
               (name, layout, title, subtitle, gradient, event_id, preselect_for_submitters, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id"#,
        v.name,
        v.layout,
        v.title,
        v.subtitle,
        v.gradient,
        v.event_id,
        v.preselect_for_submitters,
        acteur
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

pub async fn modifier(conn: &mut PgConnection, id: Uuid, v: &ValeursGabarit<'_>) -> Result<()> {
    sqlx::query!(
        r#"UPDATE engagement.mail_templates
              SET name = $2, layout = $3, title = $4, subtitle = $5, gradient = $6,
                  event_id = $7, preselect_for_submitters = $8
            WHERE id = $1"#,
        id,
        v.name,
        v.layout,
        v.title,
        v.subtitle,
        v.gradient,
        v.event_id,
        v.preselect_for_submitters
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Les envois qui le citent retombent sur leur mise en page (`ON DELETE SET
/// NULL`). Ses rattachements media restent, comme ceux d'un envoi supprimé : le
/// module n'écrit pas hors de son schéma, la purge des orphelins s'en charge.
pub async fn supprimer(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!("DELETE FROM engagement.mail_templates WHERE id = $1", id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Le gabarit à présélectionner pour un envoi, par la fonction du modèle.
pub async fn suggere(pool: &PgPool, mailing_id: Uuid) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT engagement.mailing_suggested_template($1)",
        mailing_id
    )
    .fetch_one(pool)
    .await?;
    Ok(id)
}
