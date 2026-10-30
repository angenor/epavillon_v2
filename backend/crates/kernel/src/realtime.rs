//! Temps réel : sujets, émission d'un signal, autorisation d'un abonnement.
//!
//! Un signal dit « tel sujet a changé » ; la page relit sa route REST. Il ne
//! porte jamais la donnée, donc l'autorisation de la donnée reste celle de la
//! route. Le concentrateur, le flux et le pouls vivent dans le crate `api`.
//!
//! Brancher un écran :
//! 1. émettre `realtime::signal(&mut tx, "proposal:<id>", …)` dans la
//!    transaction du changement (livré au commit seulement) ;
//! 2. implémenter `TopicAuthorizer` pour ses sujets dans le module, et le
//!    rendre par une fonction `topic_authorizers()` que l'API enregistre
//!    (`api::realtime::autorisations`) ;
//! 3. côté site, `useRealtime(['proposal:<id>'], relire)`.
//!
//! Étude : `docs/progression/decisions/2026-10-31-temps-reel.md`.

use async_trait::async_trait;
use serde_json::Value;
use sqlx::postgres::{PgConnection, PgPool};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;

/// Canal unique de `platform.signal()` (010_platform.sql § 3 bis).
pub const SIGNAL_CHANNEL: &str = "platform_signal";

/// Même borne que la fonction SQL.
pub const TOPIC_MAX_LEN: usize = 200;

/// Annonce un changement, dans la transaction appelante.
///
/// `payload` reste petit : `{"kind": "comments", "audience": "committee"}`.
/// `audience` restreint la livraison aux abonnés que l'autorisation du sujet
/// admet pour cette audience (voir `Audience`).
pub async fn signal(conn: &mut PgConnection, topic: &str, payload: Option<Value>) -> Result<()> {
    // `FROM` : la fonction rend `void`, type qu'aucune colonne Rust ne décode.
    sqlx::query!(
        r#"SELECT 1 AS "un!" FROM platform.signal($1, $2)"#,
        topic,
        payload
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

/// Un sujet valide : `nom:segment[:segment…]`, minuscules, chiffres, `_` et
/// `-`. Même grammaire que la fonction SQL : un sujet que la base refuse ne
/// s'écoute pas non plus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Topic(String);

impl Topic {
    pub fn parse(brut: &str) -> Option<Self> {
        if brut.is_empty() || brut.len() > TOPIC_MAX_LEN {
            return None;
        }
        let mut segments = brut.split(':');
        let nom = segments.next()?;
        let nom_valide = nom.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
            && nom.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
        if !nom_valide {
            return None;
        }
        let mut compte = 0;
        for segment in segments {
            let valide = !segment.is_empty()
                && segment.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-'
                });
            if !valide {
                return None;
            }
            compte += 1;
        }
        (compte > 0).then(|| Self(brut.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Premier segment : `proposal`, `event`, `edition`…
    pub fn kind(&self) -> &str {
        self.0.split(':').next().unwrap_or_default()
    }

    /// Deuxième segment lu comme un identifiant, s'il en est un.
    pub fn id(&self) -> Option<Uuid> {
        self.0
            .split(':')
            .nth(1)
            .and_then(|s| Uuid::parse_str(s).ok())
    }

    /// Segments après l'identifiant : `["public", "live"]` pour
    /// `edition:<id>:public:live`.
    pub fn rest(&self) -> Vec<&str> {
        self.0.split(':').skip(2).collect()
    }
}

/// Chercher un sujet par sa chaîne dans une table indexée par `Topic`.
impl std::borrow::Borrow<str> for Topic {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Qui demande. Anonyme : personne et session nulles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Viewer {
    pub person_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
}

impl Viewer {
    pub const ANONYMOUS: Self = Self {
        person_id: None,
        session_id: None,
    };
}

/// Quels signaux d'un sujet autorisé parviennent à l'abonné.
///
/// Un signal sans audience parvient à tous. Un signal marqué
/// `audience: "committee"` ne parvient qu'à qui l'admet : sans ce filtre, une
/// organisation saurait qu'on parle de son dossier en interne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audience {
    All,
    Only(Vec<String>),
}

impl Audience {
    pub fn admits(&self, audience: Option<&str>) -> bool {
        match (self, audience) {
            (_, None) | (Self::All, _) => true,
            (Self::Only(admises), Some(a)) => admises.iter().any(|x| x == a),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopicAccess {
    Granted(Audience),
    Denied,
}

/// L'autorisation des sujets d'un module. Chaque module répond des siens
/// (`programme` pour `proposal:`, `engagement` pour `mailing:`…) : aucun crate
/// de module ne dépend d'un autre, l'API les assemble.
///
/// La réponse doit refléter **le droit de lire la route** que la page relira :
/// même garde, même portée (règle 8). Elle est rejouée toutes les 60 s.
#[async_trait]
pub trait TopicAuthorizer: Send + Sync {
    /// Sujets dont ce module répond. Deux modules peuvent partager un premier
    /// segment (`event:<id>:mailings`, `event:<id>:planner`) : d'où un test
    /// sur le sujet entier.
    fn handles(&self, topic: &Topic) -> bool;

    async fn authorize(&self, pool: &PgPool, viewer: &Viewer, topic: &Topic)
        -> Result<TopicAccess>;
}

/// Les autorisations enregistrées à l'assemblage de l'API. Un sujet dont
/// personne ne répond est refusé.
#[derive(Default, Clone)]
pub struct TopicRegistry {
    authorizers: Vec<Arc<dyn TopicAuthorizer>>,
}

impl TopicRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(mut self, authorizer: impl TopicAuthorizer + 'static) -> Self {
        self.authorizers.push(Arc::new(authorizer));
        self
    }

    pub fn register_all(
        mut self,
        authorizers: impl IntoIterator<Item = Arc<dyn TopicAuthorizer>>,
    ) -> Self {
        self.authorizers.extend(authorizers);
        self
    }

    pub async fn authorize(
        &self,
        pool: &PgPool,
        viewer: &Viewer,
        topic: &Topic,
    ) -> Result<TopicAccess> {
        match self.authorizers.iter().find(|a| a.handles(topic)) {
            Some(a) => a.authorize(pool, viewer, topic).await,
            None => Ok(TopicAccess::Denied),
        }
    }
}

/// `person:<id>` : soi seulement (fin d'un envoi qu'on a lancé, cloche).
pub struct PersonTopics;

#[async_trait]
impl TopicAuthorizer for PersonTopics {
    fn handles(&self, topic: &Topic) -> bool {
        topic.kind() == "person"
    }

    async fn authorize(&self, _: &PgPool, viewer: &Viewer, topic: &Topic) -> Result<TopicAccess> {
        let soi =
            viewer.person_id.is_some() && viewer.person_id == topic.id() && topic.rest().is_empty();
        Ok(if soi {
            TopicAccess::Granted(Audience::All)
        } else {
            TopicAccess::Denied
        })
    }
}

/// `edition:<id>:public` et ses sous-sujets (`:live`, `:incidents`,
/// `:schedule`) : ouverts à tous, sans session. N'y signaler que ce que les
/// pages publiques montrent déjà.
pub struct PublicEditionTopics;

#[async_trait]
impl TopicAuthorizer for PublicEditionTopics {
    fn handles(&self, topic: &Topic) -> bool {
        topic.kind() == "edition" && topic.rest().first() == Some(&"public")
    }

    async fn authorize(&self, _: &PgPool, _: &Viewer, topic: &Topic) -> Result<TopicAccess> {
        Ok(if topic.id().is_some() && topic.rest().len() <= 2 {
            TopicAccess::Granted(Audience::All)
        } else {
            TopicAccess::Denied
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammaire_du_sujet() {
        let id = "0198a1b2-0000-7000-8000-000000000001";
        assert!(Topic::parse(&format!("proposal:{id}")).is_some());
        assert!(Topic::parse(&format!("event:{id}:mailings")).is_some());
        assert!(Topic::parse(&format!("edition:{id}:public:live")).is_some());
        assert!(Topic::parse("proposal").is_none());
        assert!(Topic::parse("Proposal:x").is_none());
        assert!(Topic::parse("proposal:ABC").is_none());
        assert!(Topic::parse("proposal::x").is_none());
        assert!(Topic::parse("pro posal:x").is_none());
        assert!(Topic::parse(&format!("p:{}", "a".repeat(TOPIC_MAX_LEN))).is_none());
    }

    #[test]
    fn audience() {
        let deposant = Audience::Only(vec!["submitter".into()]);
        assert!(deposant.admits(None));
        assert!(deposant.admits(Some("submitter")));
        assert!(!deposant.admits(Some("committee")));
        assert!(Audience::All.admits(Some("committee")));
    }
}
