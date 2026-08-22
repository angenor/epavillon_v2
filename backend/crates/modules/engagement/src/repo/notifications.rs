//! Les notifications in-app : écriture groupée, lecture, marquage.
//!
//! # Le regroupement est décrit par le modèle et n'est fait par personne
//!
//! *« Le worker incrémente `group_count` sur la notification non lue portant la
//! même clé plutôt que d'en créer une autre »* — le commentaire du modèle le
//! dit, et aucune fonction ne le fait. Sans lui, trois réponses à un même
//! commentaire donnent trois lignes ; l'écran affiche une pile qui se répète, et
//! le badge annonce trois avis pour un seul fait (FR-092).
//!
//! `ux_notifications_group` est **partiel** — il ne porte que sur les lignes non
//! lues et à clé non nulle : une notification déjà lue ne se regroupe plus, ce
//! qui est le comportement voulu. La clause `ON CONFLICT` doit donc reprendre le
//! prédicat mot pour mot, sans quoi PostgreSQL ne sait pas quel index viser.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::notification::Notification;

/// Ce qu'une notification porte à l'écriture.
pub struct NouvelleNotification<'a> {
    pub person_id: Uuid,
    pub type_code: &'a str,
    /// Texte figé au moment de l'événement, multilingue. L'autre mode
    /// d'alimentation du modèle — les variables rendues à l'affichage — vit dans
    /// `variables`, et les deux se complètent.
    pub title: serde_json::Value,
    pub body: Option<serde_json::Value>,
    pub variables: serde_json::Value,
    /// **Chemin relatif, jamais une adresse absolue** : les domaines de
    /// préproduction ne doivent pas fuiter dans les données (FR-091). La base le
    /// vérifie elle-même.
    pub link_path: Option<String>,
    pub subject_schema: Option<&'a str>,
    pub subject_table: Option<&'a str>,
    pub subject_id: Option<Uuid>,
    pub group_key: Option<String>,
}

/// Écrit une notification, **ou incrémente celle qui porte la même clé** si elle
/// n'est pas encore lue.
///
/// Rend `true` quand une ligne a été créée, `false` quand une existante a été
/// incrémentée — ce qui permet de compter les destinataires réellement touchés
/// sans relire la table.
pub async fn ecrire(conn: &mut PgConnection, n: &NouvelleNotification<'_>) -> Result<bool> {
    let cree = sqlx::query_scalar!(
        r#"INSERT INTO engagement.notifications
               (person_id, type_code, title, body, variables, link_path,
                subject_schema, subject_table, subject_id, group_key)
           VALUES ($1, $2, $3::jsonb::platform.i18n_text, $4::jsonb::platform.i18n_text,
                   $5, $6, $7, $8, $9, $10)
           ON CONFLICT (person_id, group_key)
               WHERE group_key IS NOT NULL AND read_at IS NULL
               DO UPDATE SET group_count = engagement.notifications.group_count + 1,
                             created_at  = now()
        RETURNING (xmax = 0) AS "cree!""#,
        n.person_id,
        n.type_code,
        n.title,
        n.body,
        n.variables,
        n.link_path,
        n.subject_schema,
        n.subject_table,
        n.subject_id,
        n.group_key
    )
    .fetch_one(conn)
    .await?;

    Ok(cree)
}

/// La liste **et** le compte de non lues, dans la même réponse.
///
/// Deux appels donneraient deux chiffres mesurés à deux instants, et un badge
/// qui contredit la liste qu'il coiffe.
pub struct Fil {
    pub items: Vec<Notification>,
    pub unread_count: i64,
}

pub async fn fil(
    pool: &PgPool,
    person_id: Uuid,
    non_lues_seulement: bool,
    limite: i64,
    avant: Option<time::OffsetDateTime>,
) -> Result<Fil> {
    let lignes = sqlx::query!(
        r#"SELECT id, type_code, title, body, variables, link_path,
                  subject_schema, subject_table, subject_id, group_count,
                  read_at, created_at
             FROM engagement.notifications
            WHERE person_id = $1
              AND archived_at IS NULL
              AND ($2 = false OR read_at IS NULL)
              AND ($3::timestamptz IS NULL OR created_at < $3)
            ORDER BY created_at DESC
            LIMIT $4"#,
        person_id,
        non_lues_seulement,
        avant,
        limite
    )
    .fetch_all(pool)
    .await?;

    // Le compte porte sur **toutes** les non lues, pas sur la page : un badge
    // qui ne compterait que la page afficherait « 20 » pour toujours.
    let unread_count = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM engagement.notifications
            WHERE person_id = $1 AND read_at IS NULL AND archived_at IS NULL"#,
        person_id
    )
    .fetch_one(pool)
    .await?;

    Ok(Fil {
        items: lignes
            .into_iter()
            .map(|l| Notification {
                id: l.id,
                type_code: l.type_code,
                title: l.title,
                body: l.body,
                variables: l.variables,
                link_path: l.link_path,
                subject_schema: l.subject_schema,
                subject_table: l.subject_table,
                subject_id: l.subject_id,
                group_count: l.group_count,
                read_at: l.read_at,
                created_at: l.created_at,
            })
            .collect(),
        unread_count,
    })
}

/// Marque des notifications lues — **les siennes, et uniquement**. Sans `ids` :
/// toutes.
pub async fn marquer_lues(
    conn: &mut PgConnection,
    person_id: Uuid,
    ids: Option<&[Uuid]>,
) -> Result<u64> {
    let marquees = sqlx::query!(
        "UPDATE engagement.notifications
            SET read_at = now()
          WHERE person_id = $1 AND read_at IS NULL
            AND ($2::uuid[] IS NULL OR id = ANY($2))",
        person_id,
        ids
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(marquees)
}

pub async fn archiver(conn: &mut PgConnection, person_id: Uuid, ids: &[Uuid]) -> Result<u64> {
    let archivees = sqlx::query!(
        "UPDATE engagement.notifications
            SET archived_at = now(), read_at = COALESCE(read_at, now())
          WHERE person_id = $1 AND archived_at IS NULL AND id = ANY($2)",
        person_id,
        ids
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(archivees)
}

/// Le type de notification, **tel quel** — son libellé multilingue, non résolu.
///
/// La résolution attend l'affichage : une notification écrite aujourd'hui doit
/// rester traduisible demain, et figer le texte dans la langue du destinataire
/// au moment de l'événement rendrait la traduction a posteriori impossible.
/// C'est le second mode d'alimentation que le modèle décrit.
#[derive(Debug, Clone)]
pub struct TypeActif {
    pub label: serde_json::Value,
    pub criticality: String,
    pub module_code: String,
}

/// **La correspondance entre un événement et un avis est une DONNÉE.**
///
/// `notification_types.code` suit « la même grammaire que
/// `outbox_events.event_type` » — le modèle le dit dans son propre commentaire.
/// Chercher le type par le code de l'événement fait donc d'un avis nouveau un
/// simple INSERT, comme le modèle le promet. Un filtre écrit en dur exigerait un
/// cache chargé au démarrage, qu'un type ajouté ensuite rendrait faux.
pub async fn type_actif(pool: &PgPool, code: &str) -> Result<Option<TypeActif>> {
    let ligne = sqlx::query!(
        r#"SELECT nt.label, nt.criticality::text AS "criticality!", nt.module_code
             FROM engagement.notification_types nt
            WHERE nt.code = $1 AND nt.is_active"#,
        code
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| TypeActif {
        label: l.label,
        criticality: l.criticality,
        module_code: l.module_code,
    }))
}
