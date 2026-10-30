//! Temps réel du module : les sujets dont il répond, et les signaux des
//! échanges. L'état d'un dossier et la grille du planificateur sont signalés
//! par la base (`tg_proposals_signal`, `tg_planner_signal`).
//!
//! | Sujet | Écran | Garde, celle de la route relue |
//! |---|---|---|
//! | `proposal:<id>`, `proposal:<id>:status` | fiche, des deux côtés | `perimeter::acces_au_dossier` ; le côté organisation n'admet pas l'audience `committee` |
//! | `organization:<id>` | espace organisation | adhésion ouverte (`workspace::espace`) |
//! | `event:<id>:proposals` | liste du comité, autres dossiers | périmètre d'administration (`list::ecran`) |
//! | `event:<id>:planner` | planificateur | périmètre, puis `programme.session.schedule` sur l'édition (`routes::planner::garder`) |

use std::sync::Arc;

use async_trait::async_trait;
use kernel::error::Result;
use kernel::realtime::{self, Audience, Topic, TopicAccess, TopicAuthorizer, Viewer};
use serde_json::json;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::ids::{EventId, ProposalId};
use crate::domain::permissions::SESSION_SCHEDULE;
use crate::repo::cross;
use crate::service::perimeter::{self, Acces};
use crate::service::workspace;

/// Audience d'un signal que le déposant ne doit pas recevoir.
pub const AUDIENCE_COMITE: &str = "committee";
const AUDIENCE_DEPOSANT: &str = "submitter";

pub fn topic_authorizers() -> Vec<Arc<dyn TopicAuthorizer>> {
    vec![
        Arc::new(SujetsDuDossier),
        Arc::new(SujetsDeLOrganisation),
        Arc::new(SujetsDeLEdition),
    ]
}

/// Un refus de la garde (404, 403) refuse le sujet ; une panne reste une panne.
fn refus_ou_panne(resultat: Result<TopicAccess>) -> Result<TopicAccess> {
    match resultat {
        Err(e) if e.code.status().is_client_error() => Ok(TopicAccess::Denied),
        autre => autre,
    }
}

struct SujetsDuDossier;

#[async_trait]
impl TopicAuthorizer for SujetsDuDossier {
    fn handles(&self, topic: &Topic) -> bool {
        topic.kind() == "proposal" && matches!(topic.rest().as_slice(), [] | ["status"])
    }

    async fn authorize(
        &self,
        pool: &PgPool,
        viewer: &Viewer,
        topic: &Topic,
    ) -> Result<TopicAccess> {
        let (Some(personne), Some(id)) = (viewer.person_id, topic.id()) else {
            return Ok(TopicAccess::Denied);
        };
        refus_ou_panne(
            perimeter::acces_au_dossier(pool, personne, ProposalId(id))
                .await
                .map(|(_, acces)| match acces {
                    Acces::BackOffice => TopicAccess::Granted(Audience::All),
                    Acces::Organisation => {
                        TopicAccess::Granted(Audience::Only(vec![AUDIENCE_DEPOSANT.to_owned()]))
                    }
                }),
        )
    }
}

struct SujetsDeLOrganisation;

#[async_trait]
impl TopicAuthorizer for SujetsDeLOrganisation {
    fn handles(&self, topic: &Topic) -> bool {
        topic.kind() == "organization" && topic.rest().is_empty()
    }

    async fn authorize(
        &self,
        pool: &PgPool,
        viewer: &Viewer,
        topic: &Topic,
    ) -> Result<TopicAccess> {
        let (Some(personne), Some(id)) = (viewer.person_id, topic.id()) else {
            return Ok(TopicAccess::Denied);
        };
        Ok(
            match workspace::adhesion_ouverte_dans(pool, personne, id).await? {
                Some(_) => TopicAccess::Granted(Audience::Only(vec![AUDIENCE_DEPOSANT.to_owned()])),
                None => TopicAccess::Denied,
            },
        )
    }
}

struct SujetsDeLEdition;

#[async_trait]
impl TopicAuthorizer for SujetsDeLEdition {
    fn handles(&self, topic: &Topic) -> bool {
        topic.kind() == "event" && matches!(topic.rest().as_slice(), ["proposals"] | ["planner"])
    }

    async fn authorize(
        &self,
        pool: &PgPool,
        viewer: &Viewer,
        topic: &Topic,
    ) -> Result<TopicAccess> {
        let (Some(personne), Some(id)) = (viewer.person_id, topic.id()) else {
            return Ok(TopicAccess::Denied);
        };
        let perimetre = kernel::auth::administered_events(pool, personne).await?;
        if !perimetre.allows(id) || !cross::event_exists(pool, EventId(id)).await? {
            return Ok(TopicAccess::Denied);
        }
        if topic.rest() == ["planner"]
            && !kernel::auth::has_permission(
                pool,
                personne,
                SESSION_SCHEDULE,
                kernel::auth::Scope::Event(id),
            )
            .await?
        {
            return Ok(TopicAccess::Denied);
        }
        Ok(TopicAccess::Granted(Audience::All))
    }
}

/// Un échange a changé sur un dossier : nouveau message, lu, non lu, résolu,
/// destinataires. `comite` : seul le comité le voit, le déposant n'en saura
/// rien. Sinon l'espace de l'organisation suit aussi.
pub async fn signaler_echange(
    conn: &mut PgConnection,
    dossier: ProposalId,
    edition: EventId,
    organisation: Uuid,
    comite: bool,
) -> Result<()> {
    let charge = if comite {
        json!({ "kind": "comments", "audience": AUDIENCE_COMITE })
    } else {
        json!({ "kind": "comments" })
    };
    realtime::signal(&mut *conn, &format!("proposal:{dossier}"), Some(charge)).await?;
    // Les non-lus du panneau des autres dossiers : sujet du comité seul.
    realtime::signal(
        &mut *conn,
        &format!("event:{edition}:proposals"),
        Some(json!({ "kind": "comments" })),
    )
    .await?;
    if !comite {
        realtime::signal(
            &mut *conn,
            &format!("organization:{organisation}"),
            Some(json!({ "kind": "comments" })),
        )
        .await?;
    }
    Ok(())
}
