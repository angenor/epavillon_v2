//! Temps réel des envois de courriels (31/10).
//!
//! Trois sujets : `mailing:<id>` (la fiche), `event:<id>:mailings` (la liste
//! de l'édition), `person:<auteur>` (la notification à qui a lancé l'envoi,
//! à la fin ou en cas de problème seulement). Le signal part dans la
//! transaction du changement, donc au commit et jamais pour une annulation.
//!
//! L'abonnement reprend la garde des routes que les pages relisent :
//! `engagement.mailing.send` sur l'édition et le périmètre d'administration
//! (règle 8). `person:<soi>` est servi par le socle.

use async_trait::async_trait;
use kernel::error::Result;
use kernel::realtime::{self, Audience, Topic, TopicAccess, TopicAuthorizer, Viewer};
use sqlx::postgres::{PgConnection, PgPool};
use std::sync::Arc;
use uuid::Uuid;

use crate::repo::cross;
use crate::service::mailings::droit_sur_l_edition;

/// Prévenir aussi l'auteur : à la fin de l'envoi, ou quand il échoue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auteur {
    Prevenu,
    Tenu,
}

impl Auteur {
    pub fn si(prevenir: bool) -> Self {
        if prevenir {
            Self::Prevenu
        } else {
            Self::Tenu
        }
    }
}

/// L'édition et l'auteur d'un envoi, lus dans la transaction appelante.
async fn reperes(conn: &mut PgConnection, id: Uuid) -> Result<Option<(Uuid, Option<Uuid>)>> {
    let ligne = sqlx::query!(
        "SELECT event_id, queued_by FROM engagement.mailings WHERE id = $1",
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| (l.event_id, l.queued_by)))
}

/// Annonce qu'un envoi a changé, à sa fiche et à la liste de son édition.
/// Avant une suppression : l'appeler avant le `DELETE`, la ligne disparaît.
pub async fn signaler(conn: &mut PgConnection, id: Uuid, auteur: Auteur) -> Result<()> {
    let Some((event_id, queued_by)) = reperes(&mut *conn, id).await? else {
        return Ok(());
    };
    realtime::signal(&mut *conn, &format!("mailing:{id}"), None).await?;
    realtime::signal(&mut *conn, &format!("event:{event_id}:mailings"), None).await?;
    if let (Auteur::Prevenu, Some(personne)) = (auteur, queued_by) {
        realtime::signal(
            &mut *conn,
            &format!("person:{personne}"),
            Some(serde_json::json!({ "kind": "mailing" })),
        )
        .await?;
    }
    Ok(())
}

/// `mailing:<id>` et `event:<id>:mailings`.
pub struct MailingTopics;

impl MailingTopics {
    fn edition_de_la_liste(topic: &Topic) -> Option<Uuid> {
        (topic.kind() == "event" && topic.rest() == ["mailings"])
            .then(|| topic.id())
            .flatten()
    }

    fn envoi(topic: &Topic) -> Option<Uuid> {
        (topic.kind() == "mailing" && topic.rest().is_empty())
            .then(|| topic.id())
            .flatten()
    }
}

#[async_trait]
impl TopicAuthorizer for MailingTopics {
    fn handles(&self, topic: &Topic) -> bool {
        topic.kind() == "mailing" || Self::edition_de_la_liste(topic).is_some()
    }

    async fn authorize(
        &self,
        pool: &PgPool,
        viewer: &Viewer,
        topic: &Topic,
    ) -> Result<TopicAccess> {
        let Some(personne) = viewer.person_id else {
            return Ok(TopicAccess::Denied);
        };
        let edition = if let Some(id) = Self::envoi(topic) {
            sqlx::query_scalar!("SELECT event_id FROM engagement.mailings WHERE id = $1", id)
                .fetch_optional(pool)
                .await?
        } else if let Some(event_id) = Self::edition_de_la_liste(topic) {
            cross::edition_existe(pool, event_id)
                .await?
                .then_some(event_id)
        } else {
            None
        };
        let Some(event_id) = edition else {
            return Ok(TopicAccess::Denied);
        };
        Ok(if droit_sur_l_edition(pool, personne, event_id).await? {
            TopicAccess::Granted(Audience::All)
        } else {
            TopicAccess::Denied
        })
    }
}

/// Enregistrées par l'API (`api::realtime::autorisations`).
pub fn topic_authorizers() -> Vec<Arc<dyn TopicAuthorizer>> {
    vec![Arc::new(MailingTopics)]
}
