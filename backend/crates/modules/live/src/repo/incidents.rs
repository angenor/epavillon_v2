//! `live.incidents` — la seule table écrite du jalon, et la fonction qui la
//! lit.
//!
//! # LA LECTURE PASSE PAR LA FONCTION, JAMAIS PAR UN `WHERE event_id`
//!
//! `live.incidents` n'a **aucune colonne d'édition** pour les portées `session`,
//! `event_day` et `organization` : le rattachement est un **calcul**, et c'est
//! `live.event_incidents()` qui le fait — en descendant l'édition vers ses
//! journées, ses activités, les organisations qui y animent, plus les messages
//! globaux. Un filtre écrit à la main laisserait fuir **trois portées sur cinq**,
//! et l'écran paraîtrait juste : il montrerait simplement moins que ce qui
//! existe.
//!
//! C'est aussi ce qui rend le **contrôle de périmètre et la lecture
//! indissociables** : retrouver un message « par la fonction, sur les éditions
//! du périmètre » est à la fois la requête et la garde.
//!
//! # LES COLONNES D'UNE FONCTION S'ANNOTENT UNE À UNE
//!
//! Une fonction qui rend une table ne porte **aucune contrainte de nullité** :
//! SQLx les déclare toutes `Option`. Chaque colonne est donc annotée d'après ce
//! que le corps de la fonction garantit réellement. C'est la leçon de B3, payée
//! deux fois.
//!
//! # CE QUI N'EST JAMAIS ÉCRIT DIRECTEMENT
//!
//! `published_at`, `published_by`, `unpublished_at`, `unpublished_by` et
//! `unpublish_reason`. Les deux fonctions du modèle les posent — et **émettent**
//! au passage. Un `UPDATE` direct ferait de l'historique un effet de bord.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::incident::ManagedIncident;
use crate::domain::payload::IncidentPayload;

/// Toutes les lignes de gestion d'une édition, **dans l'ordre où la fonction les
/// rend** : actifs, programmés, brouillons, historique ; gravité décroissante à
/// état égal. L'API ne réordonne pas — c'est l'ordre dans lequel l'équipe agit.
///
/// **`unpublished_by_name` ne vient pas de la fonction**, qui rend l'instant et
/// le motif mais pas le nom. La jointure le complète ; sans elle, l'historique
/// afficherait « retiré par — » alors que la colonne porte l'identifiant.
pub async fn de_ledition(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<ManagedIncident>> {
    let lignes = sqlx::query!(
        r#"SELECT x.incident_id     AS "incident_id!",
                  x.scope::text     AS "scope!",
                  x.severity::text  AS "severity!",
                  x.kind_code       AS "kind_code!",
                  x.title           AS "title?: Value",
                  x.message         AS "message!: Value",
                  x.action_url::text AS "action_url?",
                  x.is_dismissible  AS "is_dismissible!",
                  x.display_from    AS "display_from!",
                  x.display_until,
                  x.target_id,
                  x.target_label,
                  x.state           AS "state!",
                  x.published_at,
                  x.published_by,
                  x.published_by_name,
                  x.unpublished_at,
                  x.unpublish_reason,
                  x.created_at      AS "created_at!",
                  x.updated_at      AS "updated_at!",
                  unpub.display_name AS "unpublished_by_name?"
             FROM live.event_incidents($1, now()) x
             JOIN live.incidents i        ON i.id = x.incident_id
             LEFT JOIN identity.people unpub ON unpub.id = i.unpublished_by"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ManagedIncident {
            incident_id: l.incident_id,
            scope: l.scope,
            severity: l.severity,
            kind_code: l.kind_code,
            title: l.title,
            message: l.message,
            action_url: l.action_url,
            is_dismissible: l.is_dismissible,
            display_from: l.display_from,
            display_until: l.display_until,
            target_id: l.target_id,
            target_label: l.target_label,
            state: l.state,
            published_at: l.published_at,
            published_by: l.published_by,
            published_by_name: l.published_by_name,
            unpublished_at: l.unpublished_at,
            unpublished_by_name: l.unpublished_by_name,
            unpublish_reason: l.unpublish_reason,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}

/// Un message, retrouvé **par la fonction** sur les éditions du périmètre.
///
/// La portée globale est visible de toute édition administrée : c'est voulu, une
/// équipe qui pilote un pavillon doit savoir qu'un bandeau d'entretien le
/// couvre. Un compte de portée globale, lui, passe par `n_importe_ou`.
pub async fn dans_le_perimetre(
    conn: &mut PgConnection,
    incident_id: Uuid,
    event_ids: &[Uuid],
) -> Result<Option<ManagedIncident>> {
    let mut trouve = None;
    for event_id in event_ids {
        if let Some(ligne) = de_ledition(&mut *conn, *event_id)
            .await?
            .into_iter()
            .find(|l| l.incident_id == incident_id)
        {
            trouve = Some(ligne);
            break;
        }
    }
    Ok(trouve)
}

/// Un message, sans borne d'édition — pour un compte de **portée globale**.
///
/// Il ne contourne pas le contrôle : il l'exprime. Une personne qui administre
/// la plateforme entière n'a pas de liste d'éditions à parcourir.
pub async fn n_importe_ou(
    conn: &mut PgConnection,
    incident_id: Uuid,
) -> Result<Option<ManagedIncident>> {
    let edition = edition_du_message(&mut *conn, incident_id).await?;

    match edition {
        Some(event_id) => Ok(de_ledition(conn, event_id)
            .await?
            .into_iter()
            .find(|l| l.incident_id == incident_id)),
        // Un message de portée globale n'est rattaché à aucune édition : il se
        // relit par n'importe laquelle, la fonction le rendant pour toutes.
        None => {
            let une_edition = sqlx::query_scalar!(
                r#"SELECT e.id FROM event.events e ORDER BY e.starts_at DESC LIMIT 1"#
            )
            .fetch_optional(&mut *conn)
            .await?;
            match une_edition {
                Some(event_id) => Ok(de_ledition(conn, event_id)
                    .await?
                    .into_iter()
                    .find(|l| l.incident_id == incident_id)),
                None => Ok(None),
            }
        }
    }
}

/// **L'édition à laquelle un message se rattache — un CALCUL, jamais une
/// colonne.**
///
/// Pour les portées `session`, `event_day` et `organization`, `live.incidents`
/// n'en porte aucune : le rattachement se déduit de la cible. `None` pour un
/// message de portée `global`, qui n'appartient à aucune édition et s'affiche
/// sur toutes.
///
/// Pour la portée `organization`, l'édition retenue est celle de sa **première
/// activité dans le temps** : une organisation qui anime sur deux COP à la fois
/// est un cas que le modèle admet, et il faut bien en désigner une pour
/// vérifier la permission. Le périmètre reste, lui, vérifié édition par édition
/// par `live.event_incidents()`.
pub async fn edition_du_message(
    conn: &mut PgConnection,
    incident_id: Uuid,
) -> Result<Option<Uuid>> {
    let edition = sqlx::query_scalar!(
        r#"SELECT COALESCE(
                      i.event_id,
                      d.event_id,
                      s.event_id,
                      (SELECT ss.event_id FROM programme.sessions ss
                        WHERE ss.organization_id = i.organization_id
                        ORDER BY ss.starts_at LIMIT 1)
                  ) AS "event_id?"
             FROM live.incidents i
             LEFT JOIN event.event_days   d ON d.id = i.event_day_id
             LEFT JOIN programme.sessions s ON s.id = i.session_id
            WHERE i.id = $1"#,
        incident_id
    )
    .fetch_optional(conn)
    .await?
    .flatten();

    Ok(edition)
}

/// L'édition à laquelle une cible se rattache, pour l'autorisation et le
/// contrôle d'appartenance.
///
/// `None` : la cible n'existe pas, ou — pour une organisation — n'anime aucune
/// activité de l'édition depuis laquelle on agit. La distinction est faite par
/// l'appelant, qui seul sait quelle édition il compare.
pub async fn edition_de_la_cible(
    conn: &mut PgConnection,
    scope: &str,
    cible: Option<Uuid>,
    depuis: Uuid,
) -> Result<Option<Uuid>> {
    let Some(cible) = cible else {
        return Ok(None);
    };

    let edition = match scope {
        "event" => {
            sqlx::query_scalar!(r#"SELECT id FROM event.events WHERE id = $1"#, cible)
                .fetch_optional(&mut *conn)
                .await?
        }
        "event_day" => {
            sqlx::query_scalar!(
                r#"SELECT event_id FROM event.event_days WHERE id = $1"#,
                cible
            )
            .fetch_optional(&mut *conn)
            .await?
        }
        "session" => {
            sqlx::query_scalar!(
                r#"SELECT event_id FROM programme.sessions WHERE id = $1"#,
                cible
            )
            .fetch_optional(&mut *conn)
            .await?
        }
        // **Même critère que la portée `organization` du modèle** : une
        // organisation n'appartient à une édition que si elle y ANIME au moins
        // une activité. Une ONG en panne sur une autre COP n'a rien à y faire.
        "organization" => {
            sqlx::query_scalar!(
                r#"SELECT ss.event_id
                 FROM programme.sessions ss
                WHERE ss.organization_id = $1 AND ss.event_id = $2
                LIMIT 1"#,
                cible,
                depuis
            )
            .fetch_optional(&mut *conn)
            .await?
        }
        _ => None,
    };

    Ok(edition)
}

/// Crée un message. **Aucune colonne de publication n'est touchée** : elles
/// appartiennent aux deux fonctions du modèle.
pub async fn creer(
    conn: &mut PgConnection,
    valeurs: &IncidentPayload,
    titre: Option<&Value>,
    auteur: Uuid,
) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO live.incidents (
               scope, event_id, event_day_id, session_id, organization_id,
               incident_kind_code, severity, title, message, action_url,
               is_dismissible, display_from, display_until, created_by)
           VALUES ($1::text::live.incident_scope, $2, $3, $4, $5,
                   $6, $7::text::live.incident_severity, $8, $9, $10::text::platform.url,
                   $11, $12, $13, $14)
           RETURNING id"#,
        valeurs.scope,
        valeurs.event_id,
        valeurs.event_day_id,
        valeurs.session_id,
        valeurs.organization_id,
        valeurs.incident_kind_code,
        valeurs.severity,
        titre,
        valeurs.message,
        valeurs.action_url,
        valeurs.is_dismissible,
        valeurs.display_from,
        valeurs.display_until,
        auteur
    )
    .fetch_one(conn)
    .await?;

    Ok(id)
}

/// Corrige un message. **Aucune colonne de publication n'est touchée** :
/// republier passe par `publier()`, qui efface le retrait comme le modèle le
/// veut.
pub async fn modifier(
    conn: &mut PgConnection,
    incident_id: Uuid,
    valeurs: &IncidentPayload,
    titre: Option<&Value>,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE live.incidents
              SET scope              = $2::text::live.incident_scope,
                  event_id           = $3,
                  event_day_id       = $4,
                  session_id         = $5,
                  organization_id    = $6,
                  incident_kind_code = $7,
                  severity           = $8::text::live.incident_severity,
                  title              = $9,
                  message            = $10,
                  action_url         = $11::text::platform.url,
                  is_dismissible     = $12,
                  display_from       = $13,
                  display_until      = $14
            WHERE id = $1"#,
        incident_id,
        valeurs.scope,
        valeurs.event_id,
        valeurs.event_day_id,
        valeurs.session_id,
        valeurs.organization_id,
        valeurs.incident_kind_code,
        valeurs.severity,
        titre,
        valeurs.message,
        valeurs.action_url,
        valeurs.is_dismissible,
        valeurs.display_from,
        valeurs.display_until
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// `live.publish_incident()` — **jamais un `UPDATE` direct**.
///
/// Elle horodate, attribue depuis `platform.current_actor_id()`, **efface la
/// dépublication** et **émet** `live.incident.published`. Le service n'émet
/// rien : un `emit_event` ajouté ici doublerait chaque ligne d'outbox.
pub async fn publier(conn: &mut PgConnection, incident_id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"SELECT (live.publish_incident($1)).id AS "id!""#,
        incident_id
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

/// `live.unpublish_incident()` — **jamais un `UPDATE` direct**.
///
/// Elle lève `no_data_found` sur un message jamais publié. **La condition n'est
/// pas rejouée en amont** : le service traduit la levée en issue
/// `not_published`, et la règle vit à un seul endroit.
pub async fn depublier(
    conn: &mut PgConnection,
    incident_id: Uuid,
    motif: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"SELECT (live.unpublish_incident($1, $2)).id AS "id!""#,
        incident_id,
        motif
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

/// Le nombre de messages **actifs de portée `session`** visant chaque activité
/// d'une édition, à l'instant de la transaction.
pub async fn actifs_par_activite(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Vec<(Uuid, i64)>> {
    let lignes = sqlx::query!(
        r#"SELECT i.session_id AS "session_id!", count(*) AS "n!"
             FROM live.incidents i
             JOIN programme.sessions s ON s.id = i.session_id
            WHERE s.event_id = $1
              AND i.scope = 'session'
              AND i.published_at IS NOT NULL
              AND i.unpublished_at IS NULL
              AND i.display_from <= now()
              AND (i.display_until IS NULL OR i.display_until > now())
            GROUP BY i.session_id"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.session_id, l.n)).collect())
}

/// Instant de lecture de la transaction courante. Sert aux compositions qui ont
/// besoin de dater leur propre réponse **du même instant** que leurs lectures.
pub async fn maintenant(conn: &mut PgConnection) -> Result<OffsetDateTime> {
    let instant = sqlx::query_scalar!(r#"SELECT now() AS "maintenant!""#)
        .fetch_one(conn)
        .await?;
    Ok(instant)
}
