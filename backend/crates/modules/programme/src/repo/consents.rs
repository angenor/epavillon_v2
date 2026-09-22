//! **Écriture hors schéma n° 3 : la preuve d'un consentement RGPD** (R22).
//!
//! # Ce que ce fichier écrit, et rien d'autre
//!
//! Une ligne dans `identity.consents`, **une seule finalité** —
//! `registration_sensitive_data` —, quel que soit le nombre de champs sensibles
//! auxquels la personne a répondu. Il n'écrit ni `identity.people`, ni les
//! comptes, ni les rôles, ni les demandes RGPD, et il ne lit d'`identity` que la
//! vue d'état courant.
//!
//! # Pourquoi un contrat d'événement ne conviendrait pas
//!
//! La preuve doit vivre **dans la transaction de la donnée qu'elle couvre**.
//! Confiée à l'outbox, elle serait écrite **après** l'inscription : refuser
//! faute de consentement deviendrait impossible à garantir, et un relais mort
//! perdrait la preuve d'une donnée déjà écrite. Il n'existe d'ailleurs aucun
//! consommateur côté `identity`.
//!
//! Le modèle prévoit exactement cet usage : la colonne `source` documente
//! `'registration_form'`.
//!
//! # Une finalité, pas une par champ
//!
//! Multiplier les finalités multiplierait les lignes de preuve sans que personne
//! l'ait demandé, et rendrait le retrait ingérable — retirer lequel ?
//!
//! # La version de la politique est celle que l'API sert
//!
//! `policy_version` est `NOT NULL` : une preuve qui ne nomme pas le texte
//! accepté n'oppose rien. Elle vient de `kernel::legal`, qui embarque le texte
//! et le sert par `GET /legal/privacy` — au site comme à Guide Négo. Elle venait
//! d'un réglage d'environnement, `PRIVACY_POLICY_VERSION`, qui pouvait nommer
//! une version dont personne ne produisait le texte. Les preuves déjà écrites
//! sous `2026-01` ne sont pas réécrites : c'est un historique.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// La finalité, **écrite ici et jamais reçue** : accepter une finalité dans une
/// charge utile ferait de ce fichier une porte sur toute la table.
pub const FINALITE: &str = "registration_sensitive_data";

/// L'origine du recueil, telle que le modèle la documente.
const SOURCE: &str = "registration_form";

/// Poser la preuve, **dans la transaction de l'inscription**.
pub async fn accorder(
    conn: &mut PgConnection,
    person_id: Uuid,
    policy_version: &str,
    ip: Option<std::net::IpAddr>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO identity.consents
             (person_id, purpose, is_granted, policy_version, source, ip_address)
         VALUES ($1, $2, true, $3, $4, $5)",
        person_id,
        FINALITE,
        policy_version,
        SOURCE,
        ip.map(sqlx::types::ipnetwork::IpNetwork::from)
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Le consentement est-il **actuellement** accordé pour cette finalité ?
///
/// L'historique complet reste la preuve ; l'état courant se lit par la vue du
/// modèle, qui retient la ligne la plus récente. Sans cette lecture, une
/// seconde inscription redemanderait un accord déjà donné.
pub async fn accorde(conn: &mut PgConnection, person_id: Uuid) -> Result<bool> {
    let accorde = sqlx::query_scalar!(
        r#"SELECT COALESCE(
               (SELECT is_granted FROM identity.current_consents
                 WHERE person_id = $1 AND purpose = $2),
               false) AS "accorde!""#,
        person_id,
        FINALITE
    )
    .fetch_one(conn)
    .await?;

    Ok(accorde)
}
