//! Jetons à usage unique — les liens reçus par courriel.
//!
//! **Ce service vivait dans `identity` jusqu'en B2, et il en est sorti sans
//! changer de comportement** (specs/002-organisations/research.md § R8). Le
//! modèle déclare cinq finalités et **trois n'appartiennent pas à `identity`** :
//! l'invitation est le geste du module Organisations, la confirmation d'un
//! intervenant sera celui de B4. Or aucun crate de module ne peut dépendre d'un
//! autre — recopier « consommer un jeton atomiquement » aurait produit deux
//! implémentations de la seule opération du lot où une divergence se paie en
//! jeton rejouable.
//!
//! Le noyau connaît déjà le schéma `identity` : c'est là que vit le garde
//! d'autorisation depuis B1, pour exactement la même raison.
//!
//! **La base ne garde que l'empreinte.** Le clair ne vit que dans le courriel et
//! dans la charge utile du travail différé, effacée dès l'envoi réussi.
//!
//! **L'expiration se dérive de la finalité, jamais de l'appelant** : `expires_at`
//! est `NOT NULL` sans valeur par défaut, et la durée est lue dans la
//! configuration à cet endroit et nulle part ailleurs.

use serde::Serialize;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::config::TokenTtls;
use crate::crypto;
use crate::error::{ApiError, Result};

/// Valeurs de `identity.token_purpose`. La finalité **détermine la durée de
/// validité** : aucun appelant ne pose d'expiration lui-même.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    EmailVerification,
    PasswordReset,
    Invitation,
    MagicLink,
    SpeakerConfirmation,
}

impl TokenPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmailVerification => "email_verification",
            Self::PasswordReset => "password_reset",
            Self::Invitation => "invitation",
            Self::MagicLink => "magic_link",
            Self::SpeakerConfirmation => "speaker_confirmation",
        }
    }
}

/// Pourquoi un jeton a été refusé.
///
/// Le modèle ne distingue pas ces trois cas — il porte `consumed_at` et
/// `expires_at`, rien de plus. L'écran, lui, ne propose pas la même suite : un
/// lien périmé se redemande, un lien déjà consommé signifie que c'est fait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenRejection {
    Invalid,
    Expired,
    AlreadyUsed,
}

impl TokenRejection {
    /// L'ordre du contrat, en un seul endroit. `consommee` avant `expiree` :
    /// inverser les deux lignes suffirait à renvoyer quelqu'un demander un
    /// courriel dont il n'a plus besoin.
    pub fn from_state(
        consommee: Option<OffsetDateTime>,
        expiration: OffsetDateTime,
        maintenant: OffsetDateTime,
    ) -> Self {
        if consommee.is_some() {
            Self::AlreadyUsed
        } else if expiration <= maintenant {
            Self::Expired
        } else {
            // Le jeton est valide : l'appelant n'aurait pas dû demander de refus.
            // Le cas n'arrive que si la ligne a changé entre deux lectures, et
            // « invalide » est alors le seul refus honnête.
            Self::Invalid
        }
    }
}

/// Un jeton consommé, et ce qu'il portait. Le clair n'existe plus à ce stade :
/// il a servi à retrouver la ligne, et rien d'autre.
#[derive(Debug, Clone)]
pub struct ConsumedToken {
    pub id: Uuid,
    pub person_id: Option<Uuid>,
    pub payload: Value,
}

/// Un jeton neuf : sa ligne, et son clair — **le seul instant où les deux
/// coexistent**. Le clair part dans la charge utile du travail d'envoi ; il
/// n'est jamais journalisé.
#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub id: Uuid,
    pub clear: String,
    pub expires_at: OffsetDateTime,
}

pub async fn create(
    conn: &mut PgConnection,
    ttls: &TokenTtls,
    person_id: Uuid,
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
        person_id,
        purpose.as_str(),
        &empreinte[..],
        payload,
        expires_at
    )
    .fetch_one(conn)
    .await?;

    Ok(IssuedToken {
        id,
        clear,
        expires_at,
    })
}

/// Invalide les jetons **non consommés** de la même finalité pour la même
/// personne.
///
/// Le modèle ne porte pas de colonne « invalidé » : l'expiration est ramenée à
/// maintenant. Le refus rendu à qui cliquerait l'ancien lien sera donc
/// « périmé » — c'est le message juste, puisqu'un lien plus récent vient
/// d'arriver. Poser `consumed_at` dirait « c'est fait », ce qui est faux.
pub async fn invalidate_pending(
    conn: &mut PgConnection,
    person_id: Uuid,
    purpose: TokenPurpose,
) -> Result<u64> {
    let touches = sqlx::query!(
        "UPDATE identity.one_time_tokens
            SET expires_at = now()
          WHERE person_id = $1
            AND purpose = $2::text::identity.token_purpose
            AND consumed_at IS NULL
            AND expires_at > now()",
        person_id,
        purpose.as_str()
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touches)
}

/// Consommation **atomique** : `WHERE consumed_at IS NULL`.
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
            id: ligne.id,
            person_id: ligne.person_id,
            payload: ligne.payload,
        }));
    }

    Ok(Err(diagnostiquer(conn, &empreinte, purpose).await?))
}

/// Contrôle d'un jeton **sans le consommer** : sert à décider d'afficher un
/// formulaire avant que la personne ait rien saisi. Le jeton est revérifié à
/// l'envoi — ce contrôle-ci ne vaut aucune garantie.
pub async fn check(
    pool: &PgPool,
    clear: &str,
    purpose: TokenPurpose,
) -> Result<std::result::Result<Uuid, TokenRejection>> {
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
        Some(id) => Ok(Ok(id)),
        // Les finalités de ce jalon renseignent toujours la personne :
        // l'invitation crée la personne AVANT d'émettre son jeton, précisément
        // pour que le lien mène à quelqu'un.
        None => Ok(Err(TokenRejection::Invalid)),
    }
}

/// Supprime les jetons périmés et consommés. Rendu au travail récurrent de
/// purge, qui **reste une tâche du module `identity`** : c'est une opération
/// d'exploitation, et la déplacer n'apporterait rien.
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

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn deja_utilise_lemporte_sur_perime() {
        let maintenant = OffsetDateTime::now_utc();
        let hier = maintenant - Duration::days(1);

        assert_eq!(
            TokenRejection::from_state(Some(hier), hier, maintenant),
            TokenRejection::AlreadyUsed,
            "un jeton consommé PUIS périmé dit que le travail est fait"
        );
        assert_eq!(
            TokenRejection::from_state(None, hier, maintenant),
            TokenRejection::Expired
        );
    }
}
