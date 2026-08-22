//! Le modèle de message servi, et le type qu'il sert.
//!
//! # Un type sans révision publiée part quand même
//!
//! **Rien ne sème de modèle** (écart n° 131). Échouer laisserait tous les
//! rappels à terre sur une base neuve ; envoyer sans le dire empêcherait de
//! découvrir qu'un modèle manque. La lecture rend donc `None`, et l'appelant
//! compose un texte de secours dont la trace porte `template_id` nul — ce qui
//! **dit** qu'aucun modèle n'a servi (R27).
//!
//! # Trois chemins pour trouver le modèle, dans cet ordre
//!
//! Celui que la règle désigne ; sinon le modèle par défaut du type ; sinon un
//! modèle actif qui sert ce type. Les trois existent dans le modèle de données,
//! et n'en suivre qu'un ferait ignorer une désignation qu'un administrateur a
//! pourtant faite.

use kernel::error::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// La révision réellement servie.
#[derive(Debug, Clone)]
pub struct RevisionServie {
    pub template_id: Uuid,
    pub version: i16,
    pub subject: serde_json::Value,
    pub body_html: serde_json::Value,
    pub body_text: Option<serde_json::Value>,
}

pub async fn revision_servie(
    pool: &PgPool,
    template_id: Option<Uuid>,
    type_code: &str,
) -> Result<Option<RevisionServie>> {
    let ligne = sqlx::query!(
        r#"WITH cible AS (
               SELECT t.id, t.current_version
                 FROM engagement.message_templates t
                WHERE t.is_active
                  AND t.current_version IS NOT NULL
                  AND t.id = COALESCE(
                        $1,
                        (SELECT nt.default_template_id
                           FROM engagement.notification_types nt
                          WHERE nt.code = $2),
                        (SELECT t2.id
                           FROM engagement.message_templates t2
                          WHERE t2.type_code = $2
                            AND t2.is_active
                            AND t2.current_version IS NOT NULL
                          ORDER BY t2.updated_at DESC
                          LIMIT 1))
           )
           SELECT c.id AS "template_id!", v.version AS "version!",
                  v.subject AS "subject!", v.body_html AS "body_html!", v.body_text
             FROM cible c
             JOIN engagement.template_versions v
               ON v.template_id = c.id
              AND v.version = c.current_version
              AND v.published_at IS NOT NULL"#,
        template_id,
        type_code
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| RevisionServie {
        template_id: l.template_id,
        version: l.version,
        subject: l.subject,
        body_html: l.body_html,
        body_text: l.body_text,
    }))
}

/// Le type de notification, résolu dans la langue du destinataire.
#[derive(Debug, Clone)]
pub struct TypeDAvis {
    pub label: String,
    pub module_code: String,
    pub criticality: String,
}

pub async fn type_davis(pool: &PgPool, code: &str, locale: &str) -> Result<Option<TypeDAvis>> {
    let ligne = sqlx::query!(
        r#"SELECT platform.t(nt.label, $2) AS "label!",
                  nt.module_code,
                  nt.criticality::text AS "criticality!"
             FROM engagement.notification_types nt
            WHERE nt.code = $1 AND nt.is_active"#,
        code,
        locale
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| TypeDAvis {
        label: l.label,
        module_code: l.module_code,
        criticality: l.criticality,
    }))
}

// -----------------------------------------------------------------------------
// Ce que le back-office lit et écrit
// -----------------------------------------------------------------------------

use crate::domain::template::{MessageTemplateRow, TemplateVersion};
use sqlx::postgres::PgConnection;

/// Les modèles, avec le nombre de révisions de chacun.
///
/// Le compte vient d'ici et non d'un second appel : deux requêtes donneraient
/// deux instants, et un écran affichant « 3 révisions » sur une liste qui en
/// montre quatre.
pub async fn lister(pool: &PgPool) -> Result<Vec<MessageTemplateRow>> {
    let lignes = sqlx::query!(
        r#"SELECT t.id, t.key::text AS "key!", t.label, t.type_code,
                  t.current_version, t.is_active, t.updated_at,
                  (SELECT count(*) FROM engagement.template_versions v
                    WHERE v.template_id = t.id) AS "version_count!"
             FROM engagement.message_templates t
            ORDER BY t.key"#
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| MessageTemplateRow {
            id: l.id,
            key: l.key,
            label: l.label,
            type_code: l.type_code,
            current_version: l.current_version,
            is_active: l.is_active,
            version_count: l.version_count,
            updated_at: l.updated_at,
        })
        .collect())
}

pub async fn par_id(pool: &PgPool, template_id: Uuid) -> Result<Option<MessageTemplateRow>> {
    let ligne = sqlx::query!(
        r#"SELECT t.id, t.key::text AS "key!", t.label, t.type_code,
                  t.current_version, t.is_active, t.updated_at,
                  (SELECT count(*) FROM engagement.template_versions v
                    WHERE v.template_id = t.id) AS "version_count!"
             FROM engagement.message_templates t
            WHERE t.id = $1"#,
        template_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| MessageTemplateRow {
        id: l.id,
        key: l.key,
        label: l.label,
        type_code: l.type_code,
        current_version: l.current_version,
        is_active: l.is_active,
        version_count: l.version_count,
        updated_at: l.updated_at,
    }))
}

/// Les révisions d'un modèle, **de la plus récente à la plus ancienne** — un
/// retour arrière se choisit dans une liste où la précédente est juste dessous.
pub async fn revisions(pool: &PgPool, template_id: Uuid) -> Result<Vec<TemplateVersion>> {
    let lignes = sqlx::query!(
        r#"SELECT id, template_id, version, subject, body_html, body_text,
                  variables, published_at, created_by, created_at
             FROM engagement.template_versions
            WHERE template_id = $1
            ORDER BY version DESC"#,
        template_id
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| TemplateVersion {
            id: l.id,
            template_id: l.template_id,
            version: l.version,
            subject: l.subject,
            body_html: l.body_html,
            body_text: l.body_text,
            variables: l.variables,
            published_at: l.published_at,
            created_by: l.created_by,
            created_at: l.created_at,
        })
        .collect())
}

/// Le numéro de la prochaine révision.
///
/// **Posé par le service, jamais reçu.** Deux administrateurs qui enregistrent
/// en même temps ne doivent pas se disputer un numéro : `ux_template_versions`
/// refuse le second, qui recommence.
pub async fn prochain_numero(conn: &mut PgConnection, template_id: Uuid) -> Result<i16> {
    let numero = sqlx::query_scalar!(
        r#"SELECT (COALESCE(max(version), 0) + 1)::smallint AS "suivant!"
             FROM engagement.template_versions WHERE template_id = $1"#,
        template_id
    )
    .fetch_one(conn)
    .await?;

    Ok(numero)
}

/// Les valeurs d'une révision, **déjà assainies** par le service.
pub struct ValeursDeRevision {
    pub subject: serde_json::Value,
    pub body_html: serde_json::Value,
    pub body_text: Option<serde_json::Value>,
    pub variables: Vec<String>,
    pub created_by: Uuid,
}

/// Écrit une révision — **jamais publiée à l'écriture**. Publier est un second
/// geste, et c'est ce qui permet d'enregistrer un brouillon sans l'envoyer à
/// deux mille personnes.
pub async fn ecrire_revision(
    conn: &mut PgConnection,
    template_id: Uuid,
    version: i16,
    v: &ValeursDeRevision,
) -> std::result::Result<TemplateVersion, sqlx::Error> {
    let ligne = sqlx::query!(
        r#"INSERT INTO engagement.template_versions
               (template_id, version, subject, body_html, body_text, variables, created_by)
           VALUES ($1, $2, $3::jsonb::platform.i18n_text, $4::jsonb::platform.i18n_text,
                   $5::jsonb::platform.i18n_text, $6, $7)
        RETURNING id, template_id, version, subject, body_html, body_text,
                  variables, published_at, created_by, created_at"#,
        template_id,
        version,
        v.subject,
        v.body_html,
        v.body_text,
        &v.variables,
        v.created_by
    )
    .fetch_one(conn)
    .await?;

    Ok(TemplateVersion {
        id: ligne.id,
        template_id: ligne.template_id,
        version: ligne.version,
        subject: ligne.subject,
        body_html: ligne.body_html,
        body_text: ligne.body_text,
        variables: ligne.variables,
        published_at: ligne.published_at,
        created_by: ligne.created_by,
        created_at: ligne.created_at,
    })
}

/// **Publier, c'est un seul geste et il est réversible.**
///
/// La révision reçoit son instant de publication — une seule fois, le premier —
/// et le modèle pointe dessus. Republier une révision antérieure est **le
/// retour arrière** : rien n'est effacé, le pointeur recule.
pub async fn publier(conn: &mut PgConnection, template_id: Uuid, version: i16) -> Result<bool> {
    let marquee = sqlx::query!(
        "UPDATE engagement.template_versions
            SET published_at = COALESCE(published_at, now())
          WHERE template_id = $1 AND version = $2",
        template_id,
        version
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    if marquee == 0 {
        return Ok(false);
    }

    sqlx::query!(
        "UPDATE engagement.message_templates SET current_version = $2 WHERE id = $1",
        template_id,
        version
    )
    .execute(conn)
    .await?;

    Ok(true)
}

/// **Les variables que le TYPE s'engage à fournir** — jamais celles du modèle.
///
/// C'est contre cette liste qu'une publication est refusée : un gabarit qui
/// cite `{{prenom_du_president}}` partirait avec un trou, et le trou ne se
/// verrait qu'à la réception. `None` : le modèle ne sert aucun type — une
/// campagne d'infolettre —, et rien ne peut alors être promis.
pub async fn variables_promises(pool: &PgPool, type_code: &str) -> Result<Option<Vec<String>>> {
    let promises = sqlx::query_scalar!(
        r#"SELECT expected_variables AS "promises!"
             FROM engagement.notification_types WHERE code = $1"#,
        type_code
    )
    .fetch_optional(pool)
    .await?;

    Ok(promises)
}

/// **La trace d'expédition porte le modèle ET le numéro de révision réellement
/// servis** (FR-089).
///
/// Elle est annotée **après coup**, sur le travail qui l'a ouverte : la garde
/// d'envoi écrit la trace pour **tous** les courriels de la plateforme, y
/// compris ceux des modules livrés qui n'ont pas de modèle — et enrichir le
/// message qu'elle reçoit casserait leurs six sites de construction, ce que le
/// décorateur existe précisément pour éviter (R24).
///
/// `template_id` nul est une information et non une absence : il **dit** que le
/// texte de secours a servi, donc qu'un modèle manque (R27).
pub async fn annoter_la_trace(
    conn: &mut PgConnection,
    job_id: Uuid,
    type_code: &str,
    template: Option<(Uuid, i16)>,
    reminder_id: Uuid,
    person_id: Uuid,
) -> Result<()> {
    sqlx::query!(
        "UPDATE engagement.email_messages
            SET type_code = $2,
                template_id = $3,
                template_version = $4,
                reminder_id = $5,
                to_person_id = $6
          WHERE job_id = $1",
        job_id,
        type_code,
        template.map(|t| t.0),
        template.map(|t| t.1),
        reminder_id,
        person_id
    )
    .execute(conn)
    .await?;
    Ok(())
}
