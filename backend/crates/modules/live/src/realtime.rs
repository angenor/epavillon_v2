//! Temps réel du module : le sujet du poste de direct. Les signaux sont posés
//! par la base (`080_live.sql` § 3 bis et § 6) : poste de direct, messages
//! d'incident ; le travail `live.incident_window` rejoue celui d'un message aux
//! bornes de sa fenêtre d'affichage.
//!
//! | Sujet | Écran | Garde |
//! |---|---|---|
//! | `edition:<id>:desk` | poste de direct (`/admin/incidents`) | périmètre, puis `live.meeting.manage` sur l'édition (les réunions du poste, `list::ecran`) |
//! | `edition:<id>:public:incidents` | bandeaux publics | aucune, sujet public du socle |

use std::sync::Arc;

use async_trait::async_trait;
use kernel::auth::{administered_events, has_permission, Scope};
use kernel::error::Result;
use kernel::realtime::{Audience, Topic, TopicAccess, TopicAuthorizer, Viewer};
use sqlx::PgPool;

use crate::service::meeting::MEETING_MANAGE;

pub fn topic_authorizers() -> Vec<Arc<dyn TopicAuthorizer>> {
    vec![Arc::new(SujetDuPoste)]
}

struct SujetDuPoste;

#[async_trait]
impl TopicAuthorizer for SujetDuPoste {
    fn handles(&self, topic: &Topic) -> bool {
        topic.kind() == "edition" && topic.rest() == ["desk"]
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
        if !administered_events(pool, personne).await?.allows(id)
            || !has_permission(pool, personne, MEETING_MANAGE, Scope::Event(id)).await?
        {
            return Ok(TopicAccess::Denied);
        }
        Ok(TopicAccess::Granted(Audience::All))
    }
}
