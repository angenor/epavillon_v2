//! Ce que les tests d'intégration du module partagent : une base jetable, un
//! état de module, et le décor minimal de l'admission — un espace, un code, un
//! compte.
//!
//! Aucun double en mémoire (principe X) : chaque test travaille sur une vraie
//! base, recopiée d'un modèle chargé depuis `docs/database/`.
//!
//! # POURQUOI LA VRAIE APPLICATION N'EST PAS MONTÉE ICI
//!
//! La monter demanderait à ce crate une dépendance de développement vers `api`,
//! et `cargo tree` — le contrôle de frontière du principe II — liste aussi les
//! dépendances de développement : l'arête le ferait échouer. Le périmètre et la
//! permission vivent donc dans `service/`, et les tests les appellent
//! directement. Les tests de bout en bout sont dans `crates/api/tests/`.

#![allow(dead_code)]

use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::testing::TestDb;
use kernel::Db;
use negotiation::state::NegotiationState;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct Bac {
    pub base: TestDb,
    pub state: NegotiationState,
    pub config: Arc<Config>,
}

impl Bac {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let state = NegotiationState::new(base.db(), config.clone());
        Self {
            base,
            state,
            config,
        }
    }

    pub fn db(&self) -> Db {
        self.base.db()
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        self.base.pool()
    }

    /// Le contexte d'écriture, acteur posé : c'est lui qui alimente l'audit
    /// (principe VII). Un test qui écrit sans acteur laisse une trace anonyme,
    /// et le test d'audit passerait alors pour la mauvaise raison.
    pub fn ctx(&self, acteur: Uuid) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr").with_actor(acteur)
    }

    /// Le contexte de lecture, sans acteur.
    pub fn ctx_anonyme(&self) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr")
    }
}

// ---------------------------------------------------------------------------
// Le décor : un espace, un second hors périmètre, un code, deux comptes
// ---------------------------------------------------------------------------

pub struct Decor {
    pub space_id: Uuid,
    /// Un second espace, **hors du périmètre détaché** : la cible naturelle
    /// d'une URL forgée (règle métier n° 8).
    pub autre_space_id: Uuid,
    /// Code de portée `negotiation_space`, qui donne aussi l'appartenance au
    /// réseau des négociatrices.
    pub code_id: Uuid,
    pub code: String,
    pub network_term_id: Uuid,
    /// La personne qui entre.
    pub person_id: Uuid,
    /// La personne qui administre le premier espace, et lui seul.
    pub admin_id: Uuid,
}

pub async fn decor(bac: &Bac) -> Decor {
    let space_id = espace(bac, "cop31-test", "COP31 — test").await;
    let autre_space_id = espace(bac, "cop30-test", "COP30 — test").await;
    let network_term_id = terme_reseau(bac, "women_negotiators").await;

    let person_id = personne(bac, "awa.diallo@example.org").await;
    let admin_id = personne(bac, "admin.cop31@example.org").await;

    let code = "NEGO-001".to_owned();
    let code_id = semer_code(
        bac,
        &code,
        "Réseau des négociatrices",
        Some(space_id),
        Some(network_term_id),
    )
    .await;

    Decor {
        space_id,
        autre_space_id,
        code_id,
        code,
        network_term_id,
        person_id,
        admin_id,
    }
}

pub async fn espace(bac: &Bac, slug: &str, nom: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO negotiation.spaces (slug, name, visibility)
           VALUES ($1::text::platform.slug, jsonb_build_object('fr', $2::text), 'listed')
           RETURNING id"#,
        slug,
        nom
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'espace")
}

/// Le terme du réseau, **lu et jamais inventé** : il est semé par
/// `020_reference.sql`, et un test qui l'insérerait masquerait son absence.
pub async fn terme_reseau(bac: &Bac, code: &str) -> Uuid {
    sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'negotiation_network' AND code = $1",
        code
    )
    .fetch_one(bac.pool())
    .await
    .expect("terme de la taxonomie negotiation_network")
}

pub async fn personne(bac: &Bac, email: &str) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', now())
         RETURNING id",
        email
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la personne")
}

/// Un code d'invitation. `space_id` à `None` donne un code de portée `global` —
/// Guide Négo en entier —, ce que `ck_invitation_codes_scope` exige.
pub async fn semer_code(
    bac: &Bac,
    code: &str,
    libelle: &str,
    space_id: Option<Uuid>,
    network_term_id: Option<Uuid>,
) -> Uuid {
    let scope_type = if space_id.is_some() {
        "negotiation_space"
    } else {
        "global"
    };

    sqlx::query_scalar!(
        "INSERT INTO negotiation.invitation_codes
             (code, label, scope_type, space_id, grants_network_term_id)
         VALUES ($1, $2, $3::text::identity.scope_type, $4, $5)
         RETURNING id",
        code,
        libelle,
        scope_type,
        space_id,
        network_term_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du code")
}

/// Une attribution de rôle, avec sa portée. C'est **la portée** qui distingue
/// « administrateur de la plateforme » de « administrateur de la COP31 » : le
/// nom du rôle est le même.
pub async fn attribuer(
    bac: &Bac,
    person_id: Uuid,
    role_code: &str,
    scope_type: &str,
    scope_id: Option<Uuid>,
) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, $2, $3::text::identity.scope_type, $4)
         RETURNING id",
        person_id,
        role_code,
        scope_type,
        scope_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("attribution du rôle")
}

/// L'état d'un code tel que le back-office et le refus le lisent — **la même
/// expression**, celle de `v_invitation_codes`.
pub async fn etat_du_code(bac: &Bac, code_id: Uuid) -> String {
    sqlx::query_scalar!(
        "SELECT state FROM negotiation.v_invitation_codes WHERE id = $1",
        code_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'état du code")
    .expect("état non nul")
}

/// Les événements déposés dans l'outbox pour un agrégat. C'est ce qui prouve
/// qu'un effet inter-modules part dans la transaction du changement d'état
/// (principe IV).
pub async fn evenements(bac: &Bac, aggregate_id: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT event_type FROM platform.outbox_events
          WHERE aggregate_id = $1 ORDER BY occurred_at",
        aggregate_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'outbox")
}

/// Le back-office de Guide Négo est-il ouvert à cette personne ?
///
/// **La portée globale, et elle seule** (tranché le 21/09) : le rôle `admin`
/// porte `negotiation.space.manage` et s'attribue aussi sur un événement, si
/// bien qu'un test « sur n'importe quelle portée » passerait pour un
/// administrateur d'une seule édition — alors qu'aucun espace de négociation
/// n'est rattaché à une édition.
///
/// `identity.administered_events()` n'est **pas** appelée par ce module : elle
/// ne rend que des portées `event`.
pub async fn administre_guide_nego(bac: &Bac, person_id: Uuid) -> bool {
    kernel::auth::has_permission(
        bac.pool(),
        person_id,
        negotiation::domain::permissions::SPACE_MANAGE,
        kernel::auth::Scope::Global,
    )
    .await
    .expect("lecture de la permission")
}

// ---------------------------------------------------------------------------
// L'admission : régler, semer, saisir, constater
// ---------------------------------------------------------------------------

/// Saisir un code, comme la route le fait. **Le service est appelé
/// directement** : monter l'application demanderait à ce crate une dépendance
/// de développement vers `api`, et `cargo tree` la refuserait (principe II).
pub async fn saisir(
    bac: &Bac,
    person_id: Uuid,
    code: &str,
    device_id: Option<&str>,
) -> negotiation::domain::redeem::RedeemResult {
    negotiation::service::redeem::redeem(
        &bac.state,
        &bac.ctx(person_id),
        negotiation::service::redeem::Saisie {
            person_id,
            session_id: None,
            code,
            device_id,
            locale: "fr",
        },
    )
    .await
    .expect("saisie du code")
}

/// Le mode d'admission, **écrit en base** : c'est là qu'il vit, et c'est de là
/// qu'il est relu à chaque tentative.
pub async fn regler_le_mode(bac: &Bac, mode: &str) {
    sqlx::query!(
        "UPDATE platform.settings SET value = to_jsonb($1::text)
          WHERE key = 'negotiation.admission_mode'",
        mode
    )
    .execute(bac.pool())
    .await
    .expect("réglage du mode d'admission");
}

pub async fn regler_les_essais(bac: &Bac, max: i32, fenetre_minutes: i32, verrou_minutes: i32) {
    sqlx::query!(
        r#"UPDATE platform.settings
              SET value = jsonb_build_object('max', $1::integer,
                                             'window_minutes', $2::integer,
                                             'lock_minutes', $3::integer)
            WHERE key = 'negotiation.invitation_attempts'"#,
        max,
        fenetre_minutes,
        verrou_minutes
    )
    .execute(bac.pool())
    .await
    .expect("réglage de la limite d'essais");
}

/// Un code semé avec tout ce qui fait varier son état. Les champs à `None`
/// laissent le défaut du modèle.
#[derive(Default)]
pub struct Graine<'a> {
    pub code: &'a str,
    pub libelle: &'a str,
    pub space_id: Option<Uuid>,
    pub network_term_id: Option<Uuid>,
    pub max_uses: Option<i32>,
    pub valid_from: Option<OffsetDateTime>,
    pub valid_until: Option<OffsetDateTime>,
}

pub async fn semer(bac: &Bac, graine: Graine<'_>) -> Uuid {
    let scope_type = if graine.space_id.is_some() {
        "negotiation_space"
    } else {
        "global"
    };

    sqlx::query_scalar!(
        "INSERT INTO negotiation.invitation_codes
             (code, label, scope_type, space_id, grants_network_term_id,
              max_uses, valid_from, valid_until)
         VALUES ($1, $2, $3::text::identity.scope_type, $4, $5, $6,
                 COALESCE($7, now()), $8)
         RETURNING id",
        graine.code,
        graine.libelle,
        scope_type,
        graine.space_id,
        graine.network_term_id,
        graine.max_uses,
        graine.valid_from,
        graine.valid_until
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du code")
}

/// Révoquer sans passer par le back-office, qui n'existe pas encore.
pub async fn revoquer(bac: &Bac, code_id: Uuid, quand: OffsetDateTime) {
    sqlx::query!(
        "UPDATE negotiation.invitation_codes SET revoked_at = $2 WHERE id = $1",
        code_id,
        quand
    )
    .execute(bac.pool())
    .await
    .expect("révocation du code");
}

/// L'accès effectif, **tel que la plateforme le teste** : par permission et par
/// portée, jamais par nom de rôle.
pub async fn a_lacces(bac: &Bac, person_id: Uuid, space_id: Option<Uuid>) -> bool {
    let portee = match space_id {
        Some(id) => kernel::auth::Scope::NegotiationSpace(id),
        None => kernel::auth::Scope::Global,
    };
    kernel::auth::has_permission(
        bac.pool(),
        person_id,
        negotiation::domain::permissions::SPACE_ACCESS,
        portee,
    )
    .await
    .expect("lecture de la permission")
}

/// Les attributions vivantes d'une personne, portée comprise.
pub async fn attributions(bac: &Bac, person_id: Uuid) -> Vec<(String, Option<Uuid>)> {
    sqlx::query!(
        r#"SELECT ra.scope_type::text AS "scope_type!", ra.scope_id
             FROM identity.role_assignments ra
            WHERE ra.person_id = $1 AND ra.revoked_at IS NULL
            ORDER BY ra.granted_at"#,
        person_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des attributions")
    .into_iter()
    .map(|l| (l.scope_type, l.scope_id))
    .collect()
}

/// Les réseaux rejoints, par leur code de taxonomie.
pub async fn reseaux(bac: &Bac, person_id: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        r#"SELECT t.code AS "code!"
             FROM negotiation.network_memberships m
             JOIN reference.taxonomy_terms t ON t.id = m.network_term_id
            WHERE m.person_id = $1 AND m.left_at IS NULL
            ORDER BY m.joined_at"#,
        person_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des appartenances")
}

/// Combien d'usages ce code a-t-il, et que dit son compteur ?
pub async fn usages(bac: &Bac, code_id: Uuid) -> (i64, i32) {
    let ligne = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM negotiation.invitation_code_uses u
                    WHERE u.code_id = c.id)      AS "lignes!",
                  c.used_count                   AS "compteur!"
             FROM negotiation.invitation_codes c
            WHERE c.id = $1"#,
        code_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture des usages");

    (ligne.lignes, ligne.compteur)
}

/// Les essais enregistrés pour une personne, dans l'ordre.
pub async fn essais(bac: &Bac, person_id: Uuid) -> Vec<(String, Option<String>)> {
    sqlx::query!(
        r#"SELECT a.outcome::text AS "outcome!", a.device_id
             FROM negotiation.invitation_code_attempts a
            WHERE a.person_id = $1
            ORDER BY a.attempted_at, a.id"#,
        person_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des essais")
    .into_iter()
    .map(|l| (l.outcome, l.device_id))
    .collect()
}

/// Ce que « Mon accès » rend.
pub async fn mon_acces(bac: &Bac, person_id: Uuid) -> negotiation::domain::access::MyAccess {
    negotiation::service::access::mon_acces(&bac.state, person_id, "fr")
        .await
        .expect("lecture de l'accès")
}

// ---------------------------------------------------------------------------
// Le back-office : créer, révoquer, retirer, trancher
// ---------------------------------------------------------------------------

/// Créer un code comme la route le fait. Le libellé suffit : la portée et le
/// réseau se passent quand le test les regarde.
pub async fn creer_un_code(
    bac: &Bac,
    acteur: Uuid,
    libelle: &str,
    space_id: Option<Uuid>,
    reseau: Option<&str>,
) -> negotiation::domain::admin::InvitationCodeRow {
    use negotiation::domain::admin::{CreateInvitationCodePayload, ScopePayload};

    let scope = match space_id {
        Some(id) => ScopePayload::NegotiationSpace { id },
        None => ScopePayload::Global,
    };

    negotiation::service::admin_codes::creer(
        &bac.state,
        &bac.ctx(acteur),
        acteur,
        &CreateInvitationCodePayload {
            label: libelle.to_owned(),
            scope,
            grants_network: reseau.map(str::to_owned),
            max_uses: None,
            valid_from: None,
            valid_until: None,
        },
        "fr",
    )
    .await
    .expect("création du code")
}

/// L'écran de la liste des codes, tel que la route le rend : les lignes, les
/// espaces et **les réseaux avec leur compte de membres**.
pub async fn liste_des_codes(bac: &Bac) -> negotiation::domain::admin::InvitationCodeListScreen {
    use negotiation::repo::codes::{Filtre, LISTE_LIMITE_DEFAUT};

    negotiation::service::admin_codes::liste(
        &bac.state,
        &Filtre {
            limit: LISTE_LIMITE_DEFAUT,
            ..Filtre::default()
        },
        "fr",
    )
    .await
    .expect("liste des codes")
}

pub async fn revoquer_le_code(
    bac: &Bac,
    acteur: Uuid,
    code_id: Uuid,
    motif: Option<&str>,
) -> Option<negotiation::domain::admin::InvitationCodeRow> {
    negotiation::service::admin_codes::revoquer(
        &bac.state,
        &bac.ctx(acteur),
        acteur,
        code_id,
        motif,
        "fr",
    )
    .await
    .expect("révocation du code")
}

pub async fn retirer_lacces(
    bac: &Bac,
    acteur: Uuid,
    code_id: Uuid,
    person_id: Uuid,
    motif: Option<&str>,
) -> bool {
    negotiation::service::admin_codes::retirer_un_acces(
        &bac.state,
        &bac.ctx(acteur),
        acteur,
        code_id,
        person_id,
        motif,
    )
    .await
    .expect("retrait de l'accès")
}

pub async fn retirer_tous_les_acces(bac: &Bac, acteur: Uuid, code_id: Uuid) -> i64 {
    negotiation::service::admin_codes::retirer_tous_les_acces(
        &bac.state,
        &bac.ctx(acteur),
        acteur,
        code_id,
        Some("code compromis"),
    )
    .await
    .expect("retrait de tous les accès")
    .revoked
}

/// Les usages d'un code, tels que l'écran du back-office les lit.
pub async fn usages_du_code(
    bac: &Bac,
    code_id: Uuid,
) -> negotiation::domain::admin::InvitationCodeUsesScreen {
    negotiation::service::admin_codes::usages(&bac.state, code_id, 100, 0)
        .await
        .expect("lecture des usages")
}

/// Les écritures tracées pour **une ligne précise**, avec l'auteur que
/// `Db::write` a posé. C'est ce qui prouve FR-046 : la trace ne demande aucun
/// code, seulement d'écrire par la bonne porte.
///
/// Le filtre porte sur la ligne et non sur la table : le décor sème lui-même en
/// SQL nu, et ses insertions sont anonymes à bon droit.
pub async fn traces(bac: &Bac, table: &str, entity_id: Uuid) -> Vec<(String, Option<Uuid>)> {
    sqlx::query!(
        r#"SELECT a.action AS "action!", a.actor_id
             FROM platform.audit_log a
            WHERE a.entity_table = $1 AND a.entity_id = $2
            ORDER BY a.occurred_at"#,
        table,
        entity_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'audit")
    .into_iter()
    .map(|l| (l.action, l.actor_id))
    .collect()
}

/// L'attribution de rôle qui ouvre l'espace à cette personne, quelle que soit
/// son issue : c'est la ligne dont on relit la trace.
pub async fn attribution_de(bac: &Bac, person_id: Uuid, space_id: Option<Uuid>) -> Uuid {
    sqlx::query_scalar!(
        "SELECT id FROM identity.role_assignments
          WHERE person_id = $1 AND role_code = 'negotiator'
            AND scope_id IS NOT DISTINCT FROM $2
          ORDER BY granted_at DESC LIMIT 1",
        person_id,
        space_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("attribution introuvable")
}

// ---------------------------------------------------------------------------
// Les demandes d'accès
// ---------------------------------------------------------------------------

/// Demander l'accès, comme la route le fait.
pub async fn demander(
    bac: &Bac,
    person_id: Uuid,
    space_id: Option<Uuid>,
    message: Option<&str>,
) -> kernel::error::Result<negotiation::domain::access::AccessRequestView> {
    use negotiation::domain::requests::CreateAccessRequestPayload;

    negotiation::service::requests::demander(
        &bac.state,
        &bac.ctx(person_id),
        person_id,
        &CreateAccessRequestPayload {
            space_id,
            message: message.map(str::to_owned),
        },
    )
    .await
}

pub async fn annuler_sa_demande(
    bac: &Bac,
    person_id: Uuid,
    request_id: Uuid,
) -> kernel::error::Result<()> {
    negotiation::service::requests::annuler(&bac.state, &bac.ctx(person_id), person_id, request_id)
        .await
}

pub async fn admettre(
    bac: &Bac,
    acteur: Uuid,
    request_id: Uuid,
    motif: Option<&str>,
) -> kernel::error::Result<()> {
    negotiation::service::admin_requests::admettre(
        &bac.state,
        &bac.ctx(acteur),
        acteur,
        request_id,
        motif,
        "fr",
    )
    .await
}

pub async fn refuser(
    bac: &Bac,
    acteur: Uuid,
    request_id: Uuid,
    motif: Option<&str>,
) -> kernel::error::Result<()> {
    negotiation::service::admin_requests::refuser(
        &bac.state,
        &bac.ctx(acteur),
        acteur,
        request_id,
        motif,
        "fr",
    )
    .await
}

/// La file du back-office, sans filtre.
pub async fn file_des_demandes(bac: &Bac) -> negotiation::domain::requests::AccessRequestQueue {
    negotiation::service::admin_requests::file(
        &bac.state,
        &negotiation::repo::requests::FiltreDemandes {
            etat: None,
            limit: 100,
            offset: 0,
        },
        "fr",
    )
    .await
    .expect("lecture de la file")
}

/// Le mode d'admission tel que le back-office le bascule — par le service,
/// pas par un UPDATE : c'est le chemin dont SC-002 dépend.
pub async fn basculer_le_mode(
    bac: &Bac,
    acteur: Uuid,
    mode: &str,
) -> kernel::error::Result<negotiation::domain::admission::AdmissionSettings> {
    negotiation::service::admission::ecrire(&bac.state, &bac.ctx(acteur), acteur, mode).await
}

/// Les travaux de courriel en file, avec leur tâche et leur destinataire.
/// C'est ce qui prouve qu'un envoi naît dans la transaction de la décision.
pub async fn courriels_en_file(bac: &Bac) -> Vec<(String, String)> {
    sqlx::query!(
        r#"SELECT j.task AS "task!", (j.payload ->> 'to') AS "destinataire?"
             FROM platform.jobs j
            WHERE j.task LIKE 'negotiation.%'
            ORDER BY j.created_at"#
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de la file de travaux")
    .into_iter()
    .map(|l| (l.task, l.destinataire.unwrap_or_default()))
    .collect()
}

/// L'état d'une demande, lu en base.
pub async fn etat_de_la_demande(bac: &Bac, request_id: Uuid) -> String {
    sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM negotiation.access_requests WHERE id = $1"#,
        request_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'état de la demande")
}
