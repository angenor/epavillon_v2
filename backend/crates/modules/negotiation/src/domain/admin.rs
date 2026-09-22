//! Les formes du back-office des codes : la liste, la fiche, les usages.
//!
//! Elles suivent `frontend/app/types/admin-negotiation.ts`, qui en est la
//! source unique. Rien n'est inventé ici : chaque valeur est une colonne de
//! `negotiation.v_invitation_codes` ou de `negotiation.v_invitation_code_uses`.
//!
//! # LE CODE EST RENDU EN CLAIR, ET C'EST VOULU
//!
//! Un code d'invitation circule sur WhatsApp, recopié à la main par tout un
//! réseau : c'est un secret **partagé**, pas un secret nominatif (FR-037). Le
//! masquer dans la liste empêcherait un administrateur de répondre à la seule
//! question qu'on lui pose — « quel est le code en cours ? » — et l'obligerait
//! à en créer un nouveau pour le savoir.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::access::AccessScopeView;

/// Une ligne de la liste des codes, avec son état déjà dérivé par la vue.
#[derive(Debug, Clone, Serialize)]
pub struct InvitationCodeRow {
    pub id: Uuid,
    pub code: String,
    pub label: String,
    /// `active`, `revoked`, `expired`, `not_yet_valid` ou `exhausted` — dérivé
    /// par `v_invitation_codes`, et par elle seule.
    pub state: String,
    pub scope: AccessScopeView,
    pub network: Option<NetworkTermView>,
    pub used_count: i32,
    /// Nul : sans limite d'entrées.
    pub max_uses: Option<i32>,
    #[serde(with = "time::serde::rfc3339")]
    pub valid_from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub valid_until: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
    pub revoked_reason: Option<String>,
    pub revoked_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub created_by_name: Option<String>,
}

/// Le réseau qu'un code fait rejoindre. Son libellé vient de la taxonomie,
/// **jamais d'un fichier de traduction** : un administrateur peut le modifier.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkTermView {
    pub id: Uuid,
    pub code: String,
    pub label: String,
}

/// `InvitationCodeListScreen` — l'écran de la liste en une réponse.
///
/// `spaces` porte les espaces de négociation offerts au filtre **et au
/// formulaire de création** : le même appel sert les deux écrans, et l'un ne
/// peut donc pas proposer une portée que l'autre ignore.
#[derive(Debug, Clone, Serialize)]
pub struct InvitationCodeListScreen {
    pub rows: Vec<InvitationCodeRow>,
    pub total: i64,
    pub spaces: Vec<SpaceOption>,
    pub networks: Vec<NetworkTermView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpaceOption {
    pub id: Uuid,
    pub name: String,
}

/// `InvitationCodeDetail` — la fiche d'un code.
///
/// `granted_uses` compte les personnes **dont l'accès tient encore** : c'est ce
/// qui dit à l'administrateur ce qu'il lui resterait à retirer après une
/// révocation, laquelle ne retire rien par elle-même (ADR-006).
#[derive(Debug, Clone, Serialize)]
pub struct InvitationCodeDetail {
    #[serde(flatten)]
    pub code: InvitationCodeRow,
    pub granted_uses: i64,
}

/// Une ligne d'usage : qui est entré, quand, et si son accès tient encore.
#[derive(Debug, Clone, Serialize)]
pub struct InvitationCodeUseRow {
    pub person_id: Uuid,
    pub display_name: String,
    pub email: String,
    #[serde(with = "time::serde::rfc3339")]
    pub used_at: OffsetDateTime,
    /// Lu dans `identity.role_assignments` par la vue : la table des usages ne
    /// porte aucun état d'accès.
    pub access_active: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub access_revoked_at: Option<OffsetDateTime>,
    pub access_revoked_reason: Option<String>,
}

/// `InvitationCodeUsesScreen` — les usages d'un code.
#[derive(Debug, Clone, Serialize)]
pub struct InvitationCodeUsesScreen {
    pub rows: Vec<InvitationCodeUseRow>,
    pub total: i64,
    pub granted_uses: i64,
}

/// `CreateInvitationCodePayload` — ce que le formulaire de création envoie.
///
/// **Le code n'y figure pas** : il est engendré par l'API. Laisser un
/// administrateur le choisir produirait des codes devinables — « COP31 »,
/// « IFDD2026 » — sur une porte que rien d'autre ne protège.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateInvitationCodePayload {
    pub label: String,
    pub scope: ScopePayload,
    /// Code de taxonomie du réseau, par exemple `women_negotiators`. Absent :
    /// le code n'ouvre aucune appartenance.
    #[serde(default)]
    pub grants_network: Option<String>,
    #[serde(default)]
    pub max_uses: Option<i32>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub valid_from: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub valid_until: Option<OffsetDateTime>,
}

/// La portée d'un code : une COP précise, ou Guide Négo en entier. **Les deux
/// seules valeurs des `allowed_scopes` du rôle `negotiator`.**
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScopePayload {
    Global,
    NegotiationSpace { id: Uuid },
}

impl ScopePayload {
    pub fn as_db(&self) -> (&'static str, Option<Uuid>) {
        match self {
            Self::Global => ("global", None),
            Self::NegotiationSpace { id } => ("negotiation_space", Some(*id)),
        }
    }
}

/// Le motif d'une révocation ou d'un retrait, toujours facultatif.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReasonPayload {
    #[serde(default)]
    pub reason: Option<String>,
}

/// `RevokeAllAccessResult` — ce que rend le retrait en bloc.
#[derive(Debug, Clone, Serialize)]
pub struct RevokeAllAccessResult {
    pub revoked: i64,
}
