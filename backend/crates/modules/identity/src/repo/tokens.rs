//! Jetons à usage unique : création, consommation, invalidation.
//!
//! **La base ne garde que l'empreinte.** Le clair ne vit que dans le courriel
//! et dans la charge utile du travail différé, effacée dès l'envoi réussi
//! (research.md § R8).
//!
//! **L'expiration se dérive de la finalité, jamais de l'appelant** (FR-018) :
//! `expires_at` est `NOT NULL` sans valeur par défaut — c'est précisément
//! l'écart n° 19 —, et la durée est lue dans la configuration à cet endroit et
//! nulle part ailleurs. Sans cela, deux liens de finalités différentes vivraient
//! des durées différentes sans que personne l'ait décidé.

use kernel::config::TokenTtls;
use kernel::crypto;
use kernel::error::{ApiError, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::domain::ids::{PersonId, TokenId};
use crate::domain::token::{ConsumedToken, TokenPurpose, TokenRejection};

/// Un jeton neuf : sa ligne, et son clair — **le seul instant où les deux
/// coexistent**. Le clair part dans la charge utile du travail d'envoi ; il
/// n'est jamais journalisé.
#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub id: TokenId,
    pub clear: String,
    pub expires_at: OffsetDateTime,
}

pub async fn create(
    conn: &mut PgConnection,
    ttls: &TokenTtls,
    person_id: PersonId,
    purpose: TokenPurpose,
    payload: Value,
) -> Result<IssuedToken> {
    let duree = ttls.for_purpose(purpose.as_str()).ok_or_else(|| {
        ApiError::internal(format!(
            "aucune durée configurée pour la finalité « {} »",
            purpose.as_str()
        ))
    })?;
    let expires_at = OffsetDateTime::now_utc() + duree;

    let clear = crypto::random_token();
    let empreinte = crypto::token_hash(&clear);

    let id = sqlx::query_scalar!(
        "INSERT INTO identity.one_time_tokens
             (person_id, purpose, token_hash, payload, expires_at)
         VALUES ($1, $2::text::identity.token_purpose, $3, $4, $5)
         RETURNING id",
        person_id.as_uuid(),
        purpose.as_str(),
        &empreinte[..],
        payload,
        expires_at
    )
    .fetch_one(conn)
    .await?;

    Ok(IssuedToken {
        id: TokenId(id),
        clear,
        expires_at,
    })
}

/// Invalide les jetons **non consommés** de la même finalité pour la même
/// personne (FR-040).
///
/// Le modèle ne porte pas de colonne « invalidé » : l'expiration est ramenée à
/// maintenant. Le refus rendu à qui cliquerait l'ancien lien sera donc
/// « périmé » — c'est le message juste, puisqu'un lien plus récent vient
/// d'arriver. Poser `consumed_at` dirait « c'est fait », ce qui est faux.
pub async fn invalidate_pending(
    conn: &mut PgConnection,
    person_id: PersonId,
    purpose: TokenPurpose,
) -> Result<u64> {
    let touches = sqlx::query!(
        "UPDATE identity.one_time_tokens
            SET expires_at = now()
          WHERE person_id = $1
            AND purpose = $2::text::identity.token_purpose
            AND consumed_at IS NULL
            AND expires_at > now()",
        person_id.as_uuid(),
        purpose.as_str()
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touches)
}

/// Consommation **atomique** : `WHERE consumed_at IS NULL` (FR-041).
///
/// Deux clics simultanés n'aboutissent qu'une fois — c'est la base qui
/// tranche, pas une lecture suivie d'une écriture. Le second appel retombe sur
/// le diagnostic, qui rend « déjà utilisé ».
///
/// La finalité entre dans le filtre : un jeton de réinitialisation présenté à
/// la vérification d'adresse est **invalide**, pas « déjà utilisé ».
pub async fn consume(
    conn: &mut PgConnection,
    clear: &str,
    purpose: TokenPurpose,
) -> Result<std::result::Result<ConsumedToken, TokenRejection>> {
    let empreinte = crypto::token_hash(clear);

    let consomme = sqlx::query!(
        r#"UPDATE identity.one_time_tokens
              SET consumed_at = now()
            WHERE token_hash = $1
              AND purpose = $2::text::identity.token_purpose
              AND consumed_at IS NULL
              AND expires_at > now()
        RETURNING id, person_id, payload"#,
        &empreinte[..],
        purpose.as_str()
    )
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(ligne) = consomme {
        return Ok(Ok(ConsumedToken {
            id: TokenId(ligne.id),
            person_id: ligne.person_id.map(PersonId),
            payload: ligne.payload,
        }));
    }

    Ok(Err(diagnostiquer(conn, &empreinte, purpose).await?))
}

/// Contrôle d'un jeton **sans le consommer** : sert à décider d'afficher un
/// formulaire avant que la personne ait rien saisi. Le jeton est revérifié à
/// l'envoi (FR-042) — ce contrôle-ci ne vaut aucune garantie.
pub async fn check(
    pool: &PgPool,
    clear: &str,
    purpose: TokenPurpose,
) -> Result<std::result::Result<PersonId, TokenRejection>> {
    let empreinte = crypto::token_hash(clear);

    let ligne = sqlx::query!(
        "SELECT person_id, consumed_at, expires_at
           FROM identity.one_time_tokens
          WHERE token_hash = $1 AND purpose = $2::text::identity.token_purpose",
        &empreinte[..],
        purpose.as_str()
    )
    .fetch_optional(pool)
    .await?;

    let Some(ligne) = ligne else {
        return Ok(Err(TokenRejection::Invalid));
    };

    let maintenant = OffsetDateTime::now_utc();
    if ligne.consumed_at.is_some() || ligne.expires_at <= maintenant {
        return Ok(Err(TokenRejection::from_state(
            ligne.consumed_at,
            ligne.expires_at,
            maintenant,
        )));
    }

    match ligne.person_id {
        Some(id) => Ok(Ok(PersonId(id))),
        // Les deux finalités de ce jalon renseignent toujours la personne ;
        // l'invitation d'un inconnu, elle, ne le fera pas.
        None => Ok(Err(TokenRejection::Invalid)),
    }
}

/// Supprime les jetons périmés et consommés (FR-044). Rendu au travail
/// récurrent de purge.
pub async fn purge(conn: &mut PgConnection) -> Result<u64> {
    let supprimes = sqlx::query!(
        "DELETE FROM identity.one_time_tokens
          WHERE consumed_at IS NOT NULL OR expires_at <= now()"
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(supprimes)
}

async fn diagnostiquer(
    conn: &mut PgConnection,
    empreinte: &[u8; 32],
    purpose: TokenPurpose,
) -> Result<TokenRejection> {
    let ligne = sqlx::query!(
        "SELECT consumed_at, expires_at
           FROM identity.one_time_tokens
          WHERE token_hash = $1 AND purpose = $2::text::identity.token_purpose",
        &empreinte[..],
        purpose.as_str()
    )
    .fetch_optional(conn)
    .await?;

    Ok(match ligne {
        None => TokenRejection::Invalid,
        Some(l) => {
            TokenRejection::from_state(l.consumed_at, l.expires_at, OffsetDateTime::now_utc())
        }
    })
}
