//! Garde d'autorisation — **dans le noyau, pas dans `identity`**.
//!
//! L'autorisation se teste par `identity.has_permission()` et
//! `identity.administered_events()`, deux fonctions SQL. Tous les modules à
//! venir doivent les appeler, et aucun crate de module n'a le droit de dépendre
//! du crate `identity` (principe II). Placer le garde dans `identity` créerait
//! donc, dès B2, exactement l'arête interdite.
//!
//! Ce n'est pas une entorse : le noyau connaît le schéma `identity` comme il
//! connaît `platform`. Le modèle avait déjà tranché en posant l'autorisation
//! comme *fonction de base*, appelable par n'importe qui.

use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use uuid::Uuid;

use crate::context::RequestContext;
use crate::db::Db;
use crate::error::{ApiError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "identity.scope_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ScopeType {
    Global,
    Organization,
    Event,
    NegotiationSpace,
}

impl ScopeType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Organization => "organization",
            Self::Event => "event",
            Self::NegotiationSpace => "negotiation_space",
        }
    }

    /// L'énuméré est fermé en base : une valeur inconnue signale que le code et
    /// le modèle ont divergé.
    pub fn from_db(valeur: &str) -> Option<Self> {
        match valeur {
            "global" => Some(Self::Global),
            "organization" => Some(Self::Organization),
            "event" => Some(Self::Event),
            "negotiation_space" => Some(Self::NegotiationSpace),
            _ => None,
        }
    }
}

/// Une permission se teste toujours **sur une portée visée** : c'est la portée
/// qui distingue « administrer la plateforme » de « administrer la COP31 ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Organization(Uuid),
    Event(Uuid),
    NegotiationSpace(Uuid),
}

impl Scope {
    pub fn scope_type(self) -> ScopeType {
        match self {
            Self::Global => ScopeType::Global,
            Self::Organization(_) => ScopeType::Organization,
            Self::Event(_) => ScopeType::Event,
            Self::NegotiationSpace(_) => ScopeType::NegotiationSpace,
        }
    }

    pub fn scope_id(self) -> Option<Uuid> {
        match self {
            Self::Global => None,
            Self::Organization(id) | Self::Event(id) | Self::NegotiationSpace(id) => Some(id),
        }
    }
}

pub async fn has_permission(
    pool: &PgPool,
    person_id: Uuid,
    permission: &str,
    scope: Scope,
) -> Result<bool> {
    // Le double transtypage `::text::` est nécessaire : la macro ne sait pas
    // typer un paramètre de type personnalisé. Le `!` reprend l'invariant de la
    // fonction SQL — elle rend toujours un booléen.
    let autorise = sqlx::query_scalar!(
        r#"SELECT identity.has_permission($1, $2, $3::text::identity.scope_type, $4) AS "autorise!""#,
        person_id,
        permission,
        scope.scope_type().as_str(),
        scope.scope_id()
    )
    .fetch_one(pool)
    .await?;

    Ok(autorise)
}

pub async fn require_permission(
    pool: &PgPool,
    person_id: Uuid,
    permission: &str,
    scope: Scope,
) -> Result<()> {
    if has_permission(pool, person_id, permission, scope).await? {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}

/// La permission est-elle détenue **sur au moins une portée** ?
///
/// `has_permission` répond pour une portée précise, et c'est ce qu'il faut pour
/// autoriser une écriture. Mais une lecture d'identité s'ouvre à qui détient le
/// droit « quelle que soit la portée » : un référent d'organisation le détient
/// sur son organisation, un administrateur d'édition sur son édition, et tester
/// la portée globale les refuserait tous les deux.
pub async fn has_permission_anywhere(
    pool: &PgPool,
    person_id: Uuid,
    permission: &str,
) -> Result<bool> {
    let autorise = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM identity.effective_permissions($1)
                WHERE permission_code = $2
           ) AS "autorise!""#,
        person_id,
        permission
    )
    .fetch_one(pool)
    .await?;

    Ok(autorise)
}

pub async fn require_permission_anywhere(
    pool: &PgPool,
    person_id: Uuid,
    permission: &str,
) -> Result<()> {
    if has_permission_anywhere(pool, person_id, permission).await? {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}

/// Périmètre d'administration. Les trois cas restent distincts : les confondre
/// affiche une liste vide là où il faut un refus d'accès.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminScope {
    pub is_global: bool,
    pub event_ids: Vec<Uuid>,
}

impl AdminScope {
    /// Le filtre canonique : `WHERE is_global OR event_id = ANY(event_ids)`.
    pub fn allows(&self, event_id: Uuid) -> bool {
        self.is_global || self.event_ids.contains(&event_id)
    }

    /// Aucun droit d'administration. Se refuse explicitement — jamais par une
    /// liste vide.
    pub fn is_empty(&self) -> bool {
        !self.is_global && self.event_ids.is_empty()
    }
}

pub async fn administered_events(pool: &PgPool, person_id: Uuid) -> Result<AdminScope> {
    // Les deux `!` reprennent l'invariant que la fonction SQL porte par
    // COALESCE : jamais de NULL. Un repli silencieux le redoublerait — et c'est
    // exactement le défaut historique que le commentaire de
    // `identity.administered_events` décrit : `event_id = ANY(NULL)` vaut NULL,
    // donc un contrôle d'accès qui échoue SANS erreur. Un NULL doit être
    // bruyant, pas replié.
    let ligne = sqlx::query!(
        r#"SELECT is_global AS "is_global!", event_ids AS "event_ids!"
             FROM identity.administered_events($1)"#,
        person_id
    )
    .fetch_one(pool)
    .await?;

    Ok(AdminScope {
        is_global: ligne.is_global,
        event_ids: ligne.event_ids,
    })
}

pub async fn require_perimeter(pool: &PgPool, person_id: Uuid) -> Result<AdminScope> {
    let perimetre = administered_events(pool, person_id).await?;
    if perimetre.is_empty() {
        return Err(ApiError::forbidden());
    }
    Ok(perimetre)
}

// -----------------------------------------------------------------------------
// Extracteurs de route
// -----------------------------------------------------------------------------

/// Personne authentifiée. Sa présence dans le contexte est posée par
/// l'intergiciel de session ; son absence est un 401.
#[derive(Debug, Clone, Copy)]
pub struct Actor(pub Uuid);

fn actor_of(req: &HttpRequest) -> Result<Uuid> {
    req.extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.actor_id)
        .ok_or_else(ApiError::unauthenticated)
}

fn pool_of(req: &HttpRequest) -> Result<PgPool> {
    req.app_data::<actix_web::web::Data<Db>>()
        .map(|db| db.pool().clone())
        .ok_or_else(|| ApiError::internal("Db absente de l'état de l'application"))
}

impl FromRequest for Actor {
    type Error = ApiError;
    type Future = std::future::Ready<Result<Self>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        std::future::ready(actor_of(req).map(Actor))
    }
}

/// Permission de **portée globale**, déclarée par un type.
///
/// Une portée ciblée n'est pas hors de portée d'un extracteur — `HttpRequest`
/// porte `match_info()`, renseigné avant l'extraction. C'est un arbitrage :
/// l'extracteur devrait alors déclarer le nom du paramètre ET son type de
/// portée, et rendre lui-même le refus quand le paramètre manque ou n'est pas
/// un UUID — c'est-à-dire décider de la forme d'une erreur loin du gestionnaire
/// qui la rend. Une portée ciblée se vérifie donc dans le gestionnaire, par
/// `require_permission(…, Scope::Event(id))`. Le point de décision reste unique.
pub trait PermissionSpec {
    const CODE: &'static str;
}

#[derive(Debug, Clone, Copy)]
pub struct Requires<P: PermissionSpec> {
    pub person_id: Uuid,
    _permission: PhantomData<P>,
}

impl<P: PermissionSpec> FromRequest for Requires<P> {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let acteur = actor_of(req);
        let pool = pool_of(req);
        Box::pin(async move {
            let person_id = acteur?;
            require_permission(&pool?, person_id, P::CODE, Scope::Global).await?;
            Ok(Requires {
                person_id,
                _permission: PhantomData,
            })
        })
    }
}

/// Permission détenue **sur n'importe quelle portée**, déclarée par un type.
/// C'est ce que le contrat écrit pour les lectures d'identité.
#[derive(Debug, Clone, Copy)]
pub struct RequiresAnyScope<P: PermissionSpec> {
    pub person_id: Uuid,
    _permission: PhantomData<P>,
}

impl<P: PermissionSpec> FromRequest for RequiresAnyScope<P> {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let acteur = actor_of(req);
        let pool = pool_of(req);
        Box::pin(async move {
            let person_id = acteur?;
            require_permission_anywhere(&pool?, person_id, P::CODE).await?;
            Ok(RequiresAnyScope {
                person_id,
                _permission: PhantomData,
            })
        })
    }
}

/// Périmètre d'administration de l'appelant. Toute liste du back-office le
/// prend en argument — y compris quand l'utilisateur forge une URL.
#[derive(Debug, Clone)]
pub struct Perimeter {
    pub person_id: Uuid,
    pub scope: AdminScope,
}

impl Perimeter {
    pub fn allows(&self, event_id: Uuid) -> bool {
        self.scope.allows(event_id)
    }

    /// Un identifiant hors périmètre se refuse comme un identifiant inexistant
    /// (principe IX) : la forme de la réponse ne les distingue pas.
    pub fn ensure(&self, event_id: Uuid) -> Result<()> {
        if self.allows(event_id) {
            Ok(())
        } else {
            Err(ApiError::not_found())
        }
    }
}

impl FromRequest for Perimeter {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let acteur = actor_of(req);
        let pool = pool_of(req);
        Box::pin(async move {
            let person_id = acteur?;
            let scope = require_perimeter(&pool?, person_id).await?;
            Ok(Perimeter { person_id, scope })
        })
    }
}
