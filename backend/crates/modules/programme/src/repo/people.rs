//! **Écriture hors schéma n° 2 : l'intervenant inconnu** (R12, écart n° 26).
//!
//! # Pourquoi ce n'est pas une invention, mais un précédent
//!
//! Le module Organisations crée **déjà** la personne visée par une invitation
//! dont l'adresse est inconnue (`org::service::membership`). L'écriture est de
//! même nature, et bornée de la même façon.
//!
//! # Pourquoi un contrat d'événement ne conviendrait pas
//!
//! `proposal_speakers.person_id` est `NOT NULL`, et le contrat exige une
//! **réponse synchrone** portant la personne : le formulaire l'affiche, la
//! rattache, et détecte le doublon au clavier suivant. Une création différée
//! rendrait une réponse sans identifiant, et le doublon serait indétectable au
//! moment où le déposant est encore devant son écran.
//!
//! Refuser un intervenant inconnu reviendrait à refuser la moitié des dossiers :
//! un expert invité n'a pas de compte sur la plateforme.
//!
//! # La différence avec le précédent, et elle compte
//!
//! L'invitation ne connaît que l'adresse et pose donc un libellé neutre. **Ici,
//! le déposant a saisi le prénom et le nom : on les écrit, et on ne déduit rien
//! de l'adresse** (FR-026). Un « a.diallo » extrait d'un courriel est un nom que
//! plus personne ne corrigera — il s'afficherait sur toutes les participations
//! futures de cette personne.
//!
//! # Ce que cette écriture ne fait JAMAIS
//!
//! Ni compte, ni mot de passe, ni rôle, ni adresse secondaire, ni visibilité
//! d'annuaire modifiée. Adresse, prénom, nom, civilité — et c'est tout.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// L'identité d'un intervenant, telle que le déposant l'a saisie.
pub struct IdentiteSaisie<'a> {
    pub email: &'a str,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub civility: Option<&'a str>,
}

/// Retrouver la personne par son adresse, la créer sinon.
///
/// **L'adresse est la seule clé de rapprochement.** `platform.email` est un
/// domaine `citext` : la comparaison est insensible à la casse, et
/// « A.Diallo@ex.org » retrouve « a.diallo@ex.org ». C'est ce qui évite la
/// seconde fiche pour la même personne — le défaut n° 1 de la v1, transposé de
/// l'organisation à l'intervenant, et bien moins visible.
///
/// La lecture précède l'insertion plutôt que de s'appuyer sur un `ON CONFLICT` :
/// `ux_people_primary_email` est un index **partiel** — il ne porte que sur les
/// personnes non anonymisées —, et une clause de conflit devrait redire sa
/// condition. C'est le patron déjà retenu par le harnais de B3.
pub async fn trouver_ou_creer(
    conn: &mut PgConnection,
    identite: IdentiteSaisie<'_>,
) -> Result<Uuid> {
    if let Some(id) = trouver(conn, identite.email).await? {
        return Ok(id);
    }

    let id = sqlx::query_scalar!(
        r#"INSERT INTO identity.people
               (primary_email, first_name, last_name, civility, status)
           VALUES ($1::text::platform.email, $2, $3, $4, 'active')
        RETURNING id"#,
        identite.email,
        identite.first_name.trim(),
        identite.last_name.trim(),
        identite.civility,
    )
    .fetch_one(conn)
    .await?;

    Ok(id)
}

/// La personne portant cette adresse, si elle existe.
///
/// **Les personnes anonymisées sont écartées** : leur adresse a été remplacée,
/// et une correspondance y serait un accident. Le statut le dit.
pub async fn trouver(conn: &mut PgConnection, email: &str) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM identity.people
          WHERE primary_email = $1::text::platform.email AND status <> 'anonymized'",
        email
    )
    .fetch_optional(conn)
    .await?;

    Ok(id)
}

/// Corriger l'identité d'une personne **qui n'a pas de compte**.
///
/// # Pourquoi cette écriture existe, alors que le plan n'en prévoyait qu'une
///
/// Le contrat du front distingue trois cas d'intervenant : inconnu (créé),
/// **connu sans compte** — « elle reste modifiable » —, et connu avec compte,
/// dont l'identité est verrouillée. Le plan n'avait retenu que la création.
///
/// Ne rien écrire pour le cas du milieu produirait le pire des comportements :
/// le déposant corrige « Awa Sow » en « Awa Sow Fall », l'écran accepte,
/// l'enregistrement réussit, et **rien ne change** — sans un mot. Un refus
/// serait défendable ; un succès qui n'écrit pas ne l'est pas.
///
/// # Ce qui la borne
///
/// Trois colonnes, et **seulement quand la personne n'a pas de compte** : dès
/// qu'un compte existe, l'identité appartient à son titulaire et le service
/// refuse par `PROPOSAL_SPEAKER_IDENTITY_LOCKED`. Ni adresse, ni rôle, ni
/// compte, ni visibilité d'annuaire.
pub async fn corriger_identite(
    conn: &mut PgConnection,
    person_id: Uuid,
    first_name: &str,
    last_name: &str,
    civility: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE identity.people
            SET first_name = $2, last_name = $3, civility = $4
          WHERE id = $1
            AND NOT EXISTS (SELECT 1 FROM identity.accounts a WHERE a.person_id = $1)",
        person_id,
        first_name.trim(),
        last_name.trim(),
        civility
    )
    .execute(conn)
    .await?;

    Ok(())
}
