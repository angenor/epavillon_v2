//! Les règles de rappel : lecture, écriture, coupure.
//!
//! # Les décalages traversent EN MINUTES, dans les deux sens
//!
//! La conversion vit ici, en SQL — `extract(epoch FROM o) / 60` en lecture,
//! `make_interval(mins => m)` en écriture. La **règle**, ses bornes et ses tests
//! vivent dans [`crate::domain::offsets`] : `'1 day'` et `'24 hours'` sont le
//! même intervalle pour la base et deux chaînes différentes pour un écran, ce
//! qui suffirait à afficher deux fois le même rappel (R19).
//!
//! Ils sortent **rangés du plus lointain au plus proche**, l'ordre dans lequel
//! le modèle écrit son défaut et celui dans lequel l'écran les lit.
//!
//! # L'unicité est une MODIFICATION, jamais une erreur
//!
//! `ux_reminder_rules_event` et `ux_reminder_rules_session` interdisent deux
//! règles pour la même portée. Rendre un conflit ferait dire à l'écran
//! « impossible » là où l'administrateur voulait simplement changer ses
//! décalages (FR-073). L'écriture est donc un `ON CONFLICT … DO UPDATE`.
//!
//! **Deux fonctions et non une**, parce que les deux index sont **partiels** :
//! une clause `ON CONFLICT` ne peut nommer qu'un prédicat, et le choix dépend de
//! la portée visée. Les fondre demanderait de composer le SQL, ce que ce dépôt
//! ne fait nulle part.
//!
//! # Ce fichier ne lit rien hors de `engagement`
//!
//! Les séances d'une édition viennent de [`crate::repo::cross`], qui porte la
//! liste exhaustive de ce que le module lit ailleurs.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::reminder::{ApplicableReminderRule, ReminderRule};

/// Ce que le service écrit d'une règle. La portée est portée par **l'une** des
/// deux fonctions d'écriture, jamais par un champ de cette structure : le
/// modèle exige exactement une des deux colonnes, et la structure le reflète.
#[derive(Debug, Clone)]
pub struct ValeursDeRegle {
    /// En minutes, déjà validées et rangées par le domaine.
    pub offsets: Vec<i32>,
    pub channels: Vec<String>,
    pub type_code: String,
    pub template_id: Option<Uuid>,
    pub is_active: bool,
    pub created_by: Uuid,
}

/// La règle d'une édition — écrite, ou **modifiée** si elle existait.
///
/// **Rend le refus de la base tel quel** : `ck_reminder_rules_offsets` et
/// `ck_reminder_rules_scope` doivent arriver au service avec leur nom de
/// contrainte, sans lequel il ne saurait pas sur quel champ poser le refus.
pub async fn ecrire_pour_edition(
    conn: &mut PgConnection,
    event_id: Uuid,
    v: &ValeursDeRegle,
) -> std::result::Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO engagement.reminder_rules
               (event_id, offsets, channels, type_code, template_id, is_active, created_by)
           VALUES ($1,
                   (SELECT array_agg(make_interval(mins => m) ORDER BY m DESC)
                      FROM unnest($2::int[]) AS m),
                   $3::text[]::engagement.notification_channel[],
                   $4, $5, $6, $7)
           ON CONFLICT (event_id) WHERE event_id IS NOT NULL
           DO UPDATE SET offsets     = EXCLUDED.offsets,
                         channels    = EXCLUDED.channels,
                         type_code   = EXCLUDED.type_code,
                         template_id = EXCLUDED.template_id,
                         is_active   = EXCLUDED.is_active
        RETURNING id"#,
        event_id,
        &v.offsets,
        &v.channels,
        v.type_code,
        v.template_id,
        v.is_active,
        v.created_by
    )
    .fetch_one(conn)
    .await
}

/// La règle d'une séance — elle **remplace** celle de son édition, sans cumul.
pub async fn ecrire_pour_seance(
    conn: &mut PgConnection,
    session_id: Uuid,
    v: &ValeursDeRegle,
) -> std::result::Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO engagement.reminder_rules
               (session_id, offsets, channels, type_code, template_id, is_active, created_by)
           VALUES ($1,
                   (SELECT array_agg(make_interval(mins => m) ORDER BY m DESC)
                      FROM unnest($2::int[]) AS m),
                   $3::text[]::engagement.notification_channel[],
                   $4, $5, $6, $7)
           ON CONFLICT (session_id) WHERE session_id IS NOT NULL
           DO UPDATE SET offsets     = EXCLUDED.offsets,
                         channels    = EXCLUDED.channels,
                         type_code   = EXCLUDED.type_code,
                         template_id = EXCLUDED.template_id,
                         is_active   = EXCLUDED.is_active
        RETURNING id"#,
        session_id,
        &v.offsets,
        &v.channels,
        v.type_code,
        v.template_id,
        v.is_active,
        v.created_by
    )
    .fetch_one(conn)
    .await
}

/// Une règle par son identifiant.
pub async fn par_id(pool: &PgPool, rule_id: Uuid) -> Result<Option<ReminderRule>> {
    let ligne = sqlx::query!(
        r#"SELECT r.id, r.event_id, r.session_id,
                  (SELECT array_agg(round(extract(epoch FROM o) / 60)::int ORDER BY o DESC)
                     FROM unnest(r.offsets) AS o) AS "offsets!",
                  r.channels::text[] AS "channels!",
                  r.type_code, r.template_id, r.is_active,
                  r.created_by, r.created_at, r.updated_at
             FROM engagement.reminder_rules r
            WHERE r.id = $1"#,
        rule_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| ReminderRule {
        id: l.id,
        event_id: l.event_id,
        session_id: l.session_id,
        offsets: l.offsets,
        channels: l.channels,
        type_code: l.type_code,
        template_id: l.template_id,
        is_active: l.is_active,
        created_by: l.created_by,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }))
}

/// Les règles d'une édition : la sienne, et celles de ses séances.
///
/// Les identifiants de séance sont **fournis par l'appelant**, qui les tient de
/// [`crate::repo::cross`] : ce fichier ne lit rien hors de son schéma.
pub async fn par_edition(
    pool: &PgPool,
    event_id: Uuid,
    seances: &[Uuid],
) -> Result<Vec<ReminderRule>> {
    let lignes = sqlx::query!(
        r#"SELECT r.id, r.event_id, r.session_id,
                  (SELECT array_agg(round(extract(epoch FROM o) / 60)::int ORDER BY o DESC)
                     FROM unnest(r.offsets) AS o) AS "offsets!",
                  r.channels::text[] AS "channels!",
                  r.type_code, r.template_id, r.is_active,
                  r.created_by, r.created_at, r.updated_at
             FROM engagement.reminder_rules r
            WHERE r.event_id = $1 OR r.session_id = ANY($2)
            ORDER BY (r.session_id IS NOT NULL), r.created_at"#,
        event_id,
        seances
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ReminderRule {
            id: l.id,
            event_id: l.event_id,
            session_id: l.session_id,
            offsets: l.offsets,
            channels: l.channels,
            type_code: l.type_code,
            template_id: l.template_id,
            is_active: l.is_active,
            created_by: l.created_by,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}

/// **La règle APPLICABLE à une séance** : la sienne si elle existe, sinon celle
/// de son édition — **sans cumul**.
///
/// L'`ORDER BY` est **celui de `engagement.schedule_session_reminders()`**, mot
/// pour mot. Une fusion des deux règles, ou un tri différent, ferait partir des
/// rappels que cette lecture n'annonce pas — et l'administrateur ne saurait plus
/// ce qui va être envoyé (FR-075).
///
/// `origin` rend la non-cumulation **vérifiable de l'extérieur** : sans elle,
/// une règle de séance à deux décalages ne se distingue pas d'une règle
/// d'édition qu'on aurait tronquée (FR-074).
pub async fn applicable(
    pool: &PgPool,
    session_id: Uuid,
    event_id: Uuid,
) -> Result<Option<ApplicableReminderRule>> {
    let ligne = sqlx::query!(
        r#"SELECT r.id, r.event_id, r.session_id,
                  (SELECT array_agg(round(extract(epoch FROM o) / 60)::int ORDER BY o DESC)
                     FROM unnest(r.offsets) AS o) AS "offsets!",
                  r.channels::text[] AS "channels!",
                  r.type_code, r.template_id, r.is_active
             FROM engagement.reminder_rules r
            WHERE r.is_active AND (r.session_id = $1 OR r.event_id = $2)
            ORDER BY (r.session_id IS NOT NULL) DESC
            LIMIT 1"#,
        session_id,
        event_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| {
        let de_la_seance = l.session_id.is_some();
        ApplicableReminderRule {
            rule_id: l.id,
            origin: if de_la_seance { "session" } else { "event" }.to_owned(),
            origin_id: l.session_id.or(l.event_id).unwrap_or(session_id),
            offsets: l.offsets,
            channels: l.channels,
            type_code: l.type_code,
            template_id: l.template_id,
            is_active: l.is_active,
        }
    }))
}

/// **Annule les rappels encore à traiter que cette règle gouvernait**, et rend
/// leur nombre.
///
/// À appeler **avant** la suppression de la règle : `scheduled_reminders.rule_id`
/// est `ON DELETE SET NULL`, et une règle supprimée d'abord laisserait ses
/// rappels orphelins, donc introuvables.
///
/// Les travaux déjà en file, eux, ne sont pas décommandés : l'envoi relit l'état
/// de la ligne avant d'écrire un courriel, et un travail qui trouve un rappel
/// annulé n'envoie rien. Décommander en plus ferait un second dispositif à tenir
/// d'accord avec le premier.
pub async fn annuler_les_rappels(
    conn: &mut PgConnection,
    rule_id: Uuid,
    motif: &str,
) -> Result<i64> {
    let annules = sqlx::query!(
        "UPDATE engagement.scheduled_reminders
            SET status = 'cancelled', skip_reason = $2
          WHERE rule_id = $1 AND status IN ('pending', 'queued')",
        rule_id,
        motif
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(annules as i64)
}

/// Supprime la règle. Rend `false` si elle n'existait déjà plus.
pub async fn supprimer(conn: &mut PgConnection, rule_id: Uuid) -> Result<bool> {
    let supprimees = sqlx::query!(
        "DELETE FROM engagement.reminder_rules WHERE id = $1",
        rule_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(supprimees > 0)
}
