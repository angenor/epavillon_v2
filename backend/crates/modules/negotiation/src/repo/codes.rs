//! Retrouver un code par ce que la personne a tapé.
//!
//! **Tout passe par `negotiation.v_invitation_codes`**, jamais par la table
//! nue : l'état — actif, révoqué, épuisé, terminé, pas encore ouvert — y est
//! dérivé une seule fois, et c'est la même expression qui sert le back-office et
//! le refus rendu à l'application. Deux calculs séparés divergeraient le jour où
//! l'un des deux oublierait `valid_from`.

use kernel::auth::ScopeType;
use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::access::AccessScopeView;
use crate::domain::admin::{InvitationCodeRow, NetworkTermView, SpaceOption};

/// Un code, avec son état déjà calculé.
#[derive(Debug, Clone)]
pub struct CodeTrouve {
    pub id: Uuid,
    pub code: String,
    pub label: String,
    /// `active`, `revoked`, `expired`, `not_yet_valid` ou `exhausted`.
    pub state: String,
    pub scope_type: String,
    pub space_id: Option<Uuid>,
    pub space_name: Option<String>,
    pub grants_network_term_id: Option<Uuid>,
    pub network_code: Option<String>,
    pub network_label: Option<String>,
    pub valid_from: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

/// **La recherche se fait sur la forme normalisée**, qui est une colonne
/// `GENERATED` et non un calcul de l'appelant : `nego-024`, `NEGO 024` et
/// `Nego024` tombent tous sur la même ligne (FR-010).
///
/// L'unicité de cette colonne **couvre les codes révoqués** : c'est ce qui
/// permet de répondre « révoqué le 8 novembre » au lieu de « code inconnu », et
/// donc de ne pas envoyer la personne chercher une faute de frappe qu'elle n'a
/// pas faite.
pub async fn par_forme_normalisee(
    conn: &mut PgConnection,
    normalise: &str,
    locale: &str,
) -> Result<Option<CodeTrouve>> {
    let ligne = sqlx::query!(
        r#"SELECT c.id                              AS "id!",
                  c.code                            AS "code!",
                  c.label                           AS "label!",
                  c.state                           AS "state!",
                  c.scope_type::text                AS "scope_type!",
                  c.space_id,
                  platform.t(c.space_name, $2)      AS space_name,
                  c.grants_network_term_id,
                  c.network_code,
                  platform.t(c.network_label, $2)   AS network_label,
                  c.valid_from                      AS "valid_from!",
                  c.revoked_at
             FROM negotiation.v_invitation_codes c
            WHERE c.code_normalized = $1"#,
        normalise,
        locale
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| CodeTrouve {
        id: l.id,
        code: l.code,
        label: l.label,
        state: l.state,
        scope_type: l.scope_type,
        space_id: l.space_id,
        space_name: l.space_name,
        grants_network_term_id: l.grants_network_term_id,
        network_code: l.network_code,
        network_label: l.network_label,
        valid_from: l.valid_from,
        revoked_at: l.revoked_at,
    }))
}

/// Ce que l'écriture d'un usage a produit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    Ecrit,
    /// La personne était déjà passée par ce code. Rien n'a été écrit, et le
    /// compteur du code n'a pas bougé : l'opération est idempotente.
    DejaFait,
    /// Le quota du code était atteint. **C'est la base qui l'a dit**, pas une
    /// lecture préalable.
    Epuise,
}

/// L'usage du code par cette personne.
///
/// **Aucune vérification préalable.** `ux_invitation_code_uses_person` refuse le
/// doublon, et `ck_invitation_codes_quota` refuse le dépassement : l'incrément
/// du trigger prend le verrou de la ligne du code, ce qui sérialise deux entrées
/// simultanées sur le dernier usage. Un `SELECT used_count` avant l'insertion
/// lirait la même valeur dans les deux requêtes, et le code de 120 usages en
/// accorderait 121 (FR-038 bis).
///
/// La violation du quota est **rendue et non levée** : elle sort en 200 sous
/// l'issue `exhausted`, avec son message et ses sorties.
pub async fn enregistrer_usage(
    conn: &mut PgConnection,
    code_id: Uuid,
    person_id: Uuid,
    session_id: Option<Uuid>,
) -> Result<Usage> {
    // **Le verrou de la ligne du code se prend ICI, et explicitement.**
    //
    // Sans cette ligne, deux entrées simultanées s'interbloquent — mesuré, pas
    // supposé : l'insertion de l'usage prend d'abord un verrou PARTAGÉ sur la
    // ligne du code, par la clé étrangère, puis le trigger d'incrément tente de
    // le hausser en verrou exclusif. Deux transactions qui détiennent chacune le
    // verrou partagé s'attendent alors l'une l'autre, PostgreSQL en tue une, et
    // la seconde personne reçoit une panne (40P01) au lieu du « code épuisé »
    // que FR-038 bis lui promet.
    //
    // Ce n'est pas une vérification préalable de l'invariant : `used_count`
    // n'est pas lu, et c'est toujours `ck_invitation_codes_quota` qui refuse le
    // dépassement. C'est une prise de verrou dans le bon ordre.
    sqlx::query_scalar!(
        "SELECT id FROM negotiation.invitation_codes WHERE id = $1 FOR UPDATE",
        code_id
    )
    .fetch_optional(&mut *conn)
    .await?;

    let insere = sqlx::query_scalar!(
        "INSERT INTO negotiation.invitation_code_uses (code_id, person_id, session_id)
         VALUES ($1, $2, $3)
         ON CONFLICT (code_id, person_id) DO NOTHING
         RETURNING id",
        code_id,
        person_id,
        session_id
    )
    .fetch_optional(conn)
    .await;

    match insere {
        Ok(Some(_)) => Ok(Usage::Ecrit),
        Ok(None) => Ok(Usage::DejaFait),
        Err(erreur) if kernel::pg_error::constraint(&erreur) == Some(QUOTA) => Ok(Usage::Epuise),
        Err(erreur) => Err(erreur.into()),
    }
}

/// Le nom est lu dans `100_negotiations.sql` et non supposé : c'est lui qui
/// distingue « code épuisé » d'une vraie panne d'écriture.
const QUOTA: &str = "ck_invitation_codes_quota";

// ---------------------------------------------------------------------------
// Le back-office — lectures
// ---------------------------------------------------------------------------

/// Ce que les filtres d'URL du back-office portent, traduit en bornes de
/// requête. `etat` reprend les cinq valeurs dérivées par la vue, jamais une
/// sixième inventée ici.
#[derive(Debug, Clone, Default)]
pub struct Filtre<'a> {
    pub etat: Option<&'a str>,
    /// Vrai : les seuls codes ouvrant Guide Négo en entier.
    pub global_seulement: bool,
    pub space_id: Option<Uuid>,
    /// Cherche dans le libellé **et dans le code**, celui-ci sous sa forme
    /// normalisée : qui tape « nego 24 » cherche `NEGO-024`.
    pub q: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

pub const LISTE_LIMITE_DEFAUT: i64 = 25;
pub const LISTE_LIMITE_MAX: i64 = 100;

/// La liste, avec son total avant pagination.
///
/// **Le total vient de la même requête** (`count(*) OVER ()`) : une seconde
/// requête de comptage compterait un autre instant, et la pagination sauterait
/// une ligne le jour où un code se crée entre les deux.
pub async fn lister(
    conn: &mut PgConnection,
    filtre: &Filtre<'_>,
    locale: &str,
) -> Result<(Vec<InvitationCodeRow>, i64)> {
    let lignes = sqlx::query!(
        r#"SELECT c.id                              AS "id!",
                  c.code                            AS "code!",
                  c.label                           AS "label!",
                  c.state                           AS "state!",
                  c.scope_type::text                AS "scope_type!",
                  c.space_id,
                  platform.t(c.space_name, $7)      AS space_name,
                  c.grants_network_term_id,
                  c.network_code,
                  platform.t(c.network_label, $7)   AS network_label,
                  c.used_count                      AS "used_count!",
                  c.max_uses,
                  c.valid_from                      AS "valid_from!",
                  c.valid_until,
                  c.revoked_at,
                  c.revoked_reason,
                  rv.display_name                   AS revoked_by_name,
                  c.created_at                      AS "created_at!",
                  cr.display_name                   AS created_by_name,
                  count(*) OVER ()                  AS "total!"
             FROM negotiation.v_invitation_codes c
             LEFT JOIN identity.people cr ON cr.id = c.created_by
             LEFT JOIN identity.people rv ON rv.id = c.revoked_by
            WHERE ($1::text IS NULL OR c.state = $1)
              AND ($2::boolean IS NOT TRUE OR c.scope_type = 'global')
              AND ($3::uuid IS NULL OR c.space_id = $3)
              AND ($4::text IS NULL
                   OR c.label ILIKE '%' || $4 || '%'
                   OR c.code_normalized LIKE
                      '%' || upper(regexp_replace($4, '[^A-Za-z0-9]', '', 'g')) || '%')
            ORDER BY c.created_at DESC
            LIMIT $5 OFFSET $6"#,
        filtre.etat,
        filtre.global_seulement,
        filtre.space_id,
        filtre.q,
        filtre.limit,
        filtre.offset,
        locale
    )
    .fetch_all(conn)
    .await?;

    let total = lignes.first().map(|l| l.total).unwrap_or(0);

    let rows = lignes
        .into_iter()
        .map(|l| {
            composer(
                l.id,
                l.code,
                l.label,
                l.state,
                &l.scope_type,
                l.space_id,
                l.space_name,
                l.grants_network_term_id,
                l.network_code,
                l.network_label,
                l.used_count,
                l.max_uses,
                l.valid_from,
                l.valid_until,
                l.revoked_at,
                l.revoked_reason,
                l.revoked_by_name,
                l.created_at,
                l.created_by_name,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    Ok((rows, total))
}

/// Une fiche. `None` : le code n'existe pas — et c'est aussi ce que reçoit un
/// administrateur d'événement, dont la garde a déjà refusé la route.
pub async fn fiche(
    conn: &mut PgConnection,
    code_id: Uuid,
    locale: &str,
) -> Result<Option<InvitationCodeRow>> {
    let ligne = sqlx::query!(
        r#"SELECT c.id                              AS "id!",
                  c.code                            AS "code!",
                  c.label                           AS "label!",
                  c.state                           AS "state!",
                  c.scope_type::text                AS "scope_type!",
                  c.space_id,
                  platform.t(c.space_name, $2)      AS space_name,
                  c.grants_network_term_id,
                  c.network_code,
                  platform.t(c.network_label, $2)   AS network_label,
                  c.used_count                      AS "used_count!",
                  c.max_uses,
                  c.valid_from                      AS "valid_from!",
                  c.valid_until,
                  c.revoked_at,
                  c.revoked_reason,
                  rv.display_name                   AS revoked_by_name,
                  c.created_at                      AS "created_at!",
                  cr.display_name                   AS created_by_name
             FROM negotiation.v_invitation_codes c
             LEFT JOIN identity.people cr ON cr.id = c.created_by
             LEFT JOIN identity.people rv ON rv.id = c.revoked_by
            WHERE c.id = $1"#,
        code_id,
        locale
    )
    .fetch_optional(conn)
    .await?;

    ligne
        .map(|l| {
            composer(
                l.id,
                l.code,
                l.label,
                l.state,
                &l.scope_type,
                l.space_id,
                l.space_name,
                l.grants_network_term_id,
                l.network_code,
                l.network_label,
                l.used_count,
                l.max_uses,
                l.valid_from,
                l.valid_until,
                l.revoked_at,
                l.revoked_reason,
                l.revoked_by_name,
                l.created_at,
                l.created_by_name,
            )
        })
        .transpose()
}

/// Les espaces offerts au filtre et au formulaire de création.
pub async fn espaces(conn: &mut PgConnection, locale: &str) -> Result<Vec<SpaceOption>> {
    let lignes = sqlx::query!(
        r#"SELECT s.id                                       AS "id!",
                  COALESCE(platform.t(s.name, $1), s.slug)   AS "name!"
             FROM negotiation.spaces s
            WHERE s.archived_at IS NULL
            ORDER BY s.opened_at DESC NULLS LAST, s.slug"#,
        locale
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| SpaceOption {
            id: l.id,
            name: l.name,
        })
        .collect())
}

/// Les réseaux qu'un code peut faire rejoindre. **Lus dans la taxonomie**, et
/// jamais écrits dans un fichier de traduction : un administrateur les modifie
/// depuis le back-office des vocabulaires.
pub async fn reseaux(conn: &mut PgConnection, locale: &str) -> Result<Vec<NetworkTermView>> {
    let lignes = sqlx::query!(
        r#"SELECT t.id                                      AS "id!",
                  t.code                                    AS "code!",
                  COALESCE(platform.t(t.label, $1), t.code) AS "label!"
             FROM reference.taxonomy_terms t
            WHERE t.taxonomy_code = 'negotiation_network' AND t.is_active
            ORDER BY t.sort_order, t.code"#,
        locale
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| NetworkTermView {
            id: l.id,
            code: l.code,
            label: l.label,
        })
        .collect())
}

/// Le terme de réseau désigné par son code de taxonomie. `None` : il n'existe
/// pas, et le formulaire l'apprend sur le champ qui l'a envoyé.
pub async fn terme_de_reseau(conn: &mut PgConnection, code: &str) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'negotiation_network' AND code = $1",
        code
    )
    .fetch_optional(conn)
    .await?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// Le back-office — écritures
// ---------------------------------------------------------------------------

pub struct NouveauCode<'a> {
    pub code: &'a str,
    pub label: &'a str,
    pub scope_type: &'a str,
    pub space_id: Option<Uuid>,
    pub grants_network_term_id: Option<Uuid>,
    pub max_uses: Option<i32>,
    pub valid_from: Option<OffsetDateTime>,
    pub valid_until: Option<OffsetDateTime>,
    pub created_by: Uuid,
}

/// Insère le code. `None` : la forme normalisée existe déjà.
///
/// **Le conflit est absorbé plutôt que levé** : le code est tiré au sort, et
/// une collision se rejoue avec un autre tirage. Une erreur de contrainte
/// invaliderait la transaction entière et obligerait à tout reprendre, pour un
/// événement qui n'a rien d'anormal.
pub async fn creer(conn: &mut PgConnection, neuf: NouveauCode<'_>) -> Result<Option<Uuid>> {
    let insere = sqlx::query_scalar!(
        "INSERT INTO negotiation.invitation_codes
             (code, label, scope_type, space_id, grants_network_term_id,
              max_uses, valid_from, valid_until, created_by)
         VALUES ($1, $2, $3::text::identity.scope_type, $4, $5, $6,
                 COALESCE($7, now()), $8, $9)
         ON CONFLICT (code_normalized) DO NOTHING
         RETURNING id",
        neuf.code,
        neuf.label,
        neuf.scope_type,
        neuf.space_id,
        neuf.grants_network_term_id,
        neuf.max_uses,
        neuf.valid_from,
        neuf.valid_until,
        neuf.created_by
    )
    .fetch_optional(conn)
    .await?;

    Ok(insere)
}

/// Ce que la révocation d'un code apprend à celui qui la demande.
#[derive(Debug, Clone)]
pub struct Revocation {
    pub scope_type: String,
    pub space_id: Option<Uuid>,
    /// Accès **toujours ouverts** accordés par ce code. Révoquer n'en retire
    /// aucun (ADR-006) : ce nombre dit à l'administrateur ce qu'il lui reste
    /// éventuellement à faire.
    pub granted_uses: i32,
}

/// Révoque le code. `None` : il n'existe pas, ou il était déjà révoqué —
/// l'opération est idempotente et ne réécrit ni la date ni l'auteur du premier
/// geste.
pub async fn revoquer(
    conn: &mut PgConnection,
    code_id: Uuid,
    acteur: Uuid,
    motif: Option<&str>,
) -> Result<Option<Revocation>> {
    let ligne = sqlx::query!(
        r#"UPDATE negotiation.invitation_codes
              SET revoked_at = now(), revoked_by = $2, revoked_reason = $3
            WHERE id = $1 AND revoked_at IS NULL
        RETURNING scope_type::text AS "scope_type!", space_id,
                  (SELECT count(*)::integer
                     FROM negotiation.v_invitation_code_uses u
                    WHERE u.code_id = $1 AND u.access_active) AS "granted_uses!""#,
        code_id,
        acteur,
        motif
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| Revocation {
        scope_type: l.scope_type,
        space_id: l.space_id,
        granted_uses: l.granted_uses,
    }))
}

/// Compose une ligne de liste ou de fiche. La portée est convertie une seule
/// fois, et un énuméré inconnu signale que le modèle et le code ont divergé.
#[allow(clippy::too_many_arguments)]
fn composer(
    id: Uuid,
    code: String,
    label: String,
    state: String,
    scope_type: &str,
    space_id: Option<Uuid>,
    space_name: Option<String>,
    network_term_id: Option<Uuid>,
    network_code: Option<String>,
    network_label: Option<String>,
    used_count: i32,
    max_uses: Option<i32>,
    valid_from: OffsetDateTime,
    valid_until: Option<OffsetDateTime>,
    revoked_at: Option<OffsetDateTime>,
    revoked_reason: Option<String>,
    revoked_by_name: Option<String>,
    created_at: OffsetDateTime,
    created_by_name: Option<String>,
) -> Result<InvitationCodeRow> {
    let kind = ScopeType::from_db(scope_type).ok_or_else(|| {
        ApiError::internal(format!(
            "portée « {scope_type} » inconnue du code : le modèle et l'énuméré ont divergé"
        ))
    })?;

    let network = match (network_term_id, network_code) {
        (Some(id), Some(code)) => Some(NetworkTermView {
            id,
            label: network_label.unwrap_or_else(|| code.clone()),
            code,
        }),
        _ => None,
    };

    Ok(InvitationCodeRow {
        id,
        code,
        label,
        state,
        scope: AccessScopeView {
            kind,
            id: space_id,
            name: space_name,
        },
        network,
        used_count,
        max_uses,
        valid_from,
        valid_until,
        revoked_at,
        revoked_reason,
        revoked_by_name,
        created_at,
        created_by_name,
    })
}
