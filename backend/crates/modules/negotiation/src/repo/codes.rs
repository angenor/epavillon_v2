//! Retrouver un code par ce que la personne a tapé.
//!
//! **Tout passe par `negotiation.v_invitation_codes`**, jamais par la table
//! nue : l'état — actif, révoqué, épuisé, terminé, pas encore ouvert — y est
//! dérivé une seule fois, et c'est la même expression qui sert le back-office et
//! le refus rendu à l'application. Deux calculs séparés divergeraient le jour où
//! l'un des deux oublierait `valid_from`.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

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
