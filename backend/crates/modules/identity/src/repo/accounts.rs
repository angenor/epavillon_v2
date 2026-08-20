//! Écritures de `identity.accounts` : compteur d'échecs, verrou, dernière
//! connexion.
//!
//! Le seuil et la durée ne sont pas en base — c'est l'écart n° 18 : les colonnes
//! existent, les réglages vivent dans la configuration du service. Un seuil relu
//! en base à chaque connexion serait un point de panne, et une donnée modifiable
//! sans trace de déploiement.
//!
//! **Toutes passent par la porte d'écriture du noyau** (principe VII), y compris
//! celles du compteur d'échecs, que le chemin de la connexion appelle avant même
//! de savoir si le mot de passe est juste. Ce n'est pas gratuit — une
//! transaction demande quatre allers-retours là où une instruction seule en
//! demande un — mais ce coût-là ne se voit pas : le service lance l'écriture
//! **avant** d'attendre le hachage, et deux millisecondes se replient sans trace
//! derrière une dizaine. Ce qui aurait dû être payé, c'est de renoncer à la
//! garantie ; ça ne l'est plus.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;

use crate::domain::ids::{AccountId, PersonId};

/// Incrémente le compteur d'échecs et rend sa nouvelle valeur.
///
/// **Un verrou échu est purgé ici, et le compteur repart de un** (FR-015). C'est
/// le seul endroit qui puisse le faire : la purge doit avoir lieu avant que le
/// seuil ne soit testé, et sur les deux branches — celle du mot de passe juste
/// comme celle du mot de passe faux. Placée sur la seule branche du succès, elle
/// laissait au compte échu **une** tentative avant de le reverrouiller pour un
/// quart d'heure, indéfiniment.
///
/// Le plafond n'est pas une règle métier mais un garde-fou : `smallint`
/// déborderait au bout de trente-deux mille tentatives, et une attaque soutenue
/// sur un compte transformerait alors chaque connexion en erreur interne.
pub async fn bump_failed_attempts(conn: &mut PgConnection, account_id: AccountId) -> Result<i16> {
    let compte = sqlx::query_scalar!(
        "UPDATE identity.accounts
            SET failed_attempts = CASE
                    WHEN locked_until IS NOT NULL AND locked_until <= now() THEN 1
                    ELSE LEAST(failed_attempts + 1, 32000)
                END,
                locked_until = CASE
                    WHEN locked_until IS NOT NULL AND locked_until <= now() THEN NULL
                    ELSE locked_until
                END
          WHERE id = $1
      RETURNING failed_attempts",
        account_id.as_uuid()
    )
    .fetch_one(conn)
    .await?;

    Ok(compte)
}

/// Le mot de passe a été prouvé : le compteur retombe (FR-015). Le verrou, lui,
/// n'est pas touché — un compte verrouillé le reste jusqu'à son échéance.
pub async fn clear_attempts(conn: &mut PgConnection, account_id: AccountId) -> Result<()> {
    sqlx::query!(
        "UPDATE identity.accounts SET failed_attempts = 0 WHERE id = $1 AND failed_attempts <> 0",
        account_id.as_uuid()
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Pose le verrou, et rend **faux si quelqu'un l'a posé d'abord**.
///
/// C'est la base qui décide, pas une lecture antérieure : deux échecs
/// concurrents franchissant le seuil ensemble bloquent tous deux sur la ligne,
/// et le second retrouve un verrou déjà posé. Un franchissement, un verrou, un
/// événement — quel que soit le nombre de requêtes en vol.
///
/// Prend une transaction : l'événement `identity.account.locked` s'écrit avec le
/// changement d'état, jamais après (principe IV).
pub async fn lock(
    conn: &mut PgConnection,
    account_id: AccountId,
    jusqu_a: OffsetDateTime,
) -> Result<bool> {
    let posees = sqlx::query!(
        "UPDATE identity.accounts
            SET locked_until = $2
          WHERE id = $1 AND (locked_until IS NULL OR locked_until <= now())",
        account_id.as_uuid(),
        jusqu_a
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(posees == 1)
}

pub async fn mark_login(conn: &mut PgConnection, account_id: AccountId) -> Result<()> {
    sqlx::query!(
        "UPDATE identity.accounts
            SET failed_attempts = 0, locked_until = NULL, last_login_at = now()
          WHERE id = $1",
        account_id.as_uuid()
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Pose une nouvelle empreinte et **désarme le compte** : compteur d'échecs et
/// verrou repartent de zéro (FR-043). Quelqu'un qui s'est verrouillé à force
/// d'essayer resterait sinon bloqué un quart d'heure avec le mot de passe qu'il
/// vient de choisir.
///
/// `INSERT … ON CONFLICT` plutôt qu'un `UPDATE` : une personne dont le compte
/// est né d'une invitation n'a pas encore de ligne `password`, et un lien de
/// réinitialisation qui échouerait sur ce cas ne lui laisserait aucune issue.
/// L'inférence porte sur `ux_accounts_password_per_person`, index partiel — d'où
/// la clause `WHERE` recopiée.
pub async fn set_password(
    conn: &mut PgConnection,
    person_id: PersonId,
    empreinte: &str,
) -> Result<AccountId> {
    let id = sqlx::query_scalar!(
        "INSERT INTO identity.accounts
             (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())
         ON CONFLICT (person_id) WHERE provider = 'password'
         DO UPDATE SET password_hash = EXCLUDED.password_hash,
                       password_changed_at = now(),
                       failed_attempts = 0,
                       locked_until = NULL
         RETURNING id",
        person_id.as_uuid(),
        empreinte
    )
    .fetch_one(conn)
    .await?;

    Ok(AccountId(id))
}
