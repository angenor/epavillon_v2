//! **Le seul fichier du module qui lise hors de son schéma.**
//!
//! # La règle de frontière, posée par B2 et resserrée ici
//!
//! *Un module lit hors de son schéma quand la question porte sur ses propres
//! entités, il n'y écrit jamais, et il n'appelle jamais un autre module.*
//!
//! Les rappels de ce module servent des séances et des inscriptions : la
//! question « quand cette séance commence-t-elle » et « qui y est inscrit »
//! portent bien sur ses propres entités. **Il n'écrit nulle part hors de
//! `engagement`**, et le contrôle mécanique du quickstart porte aussi sur
//! `reference` et `content`.
//!
//! # La liste exhaustive de ce qui est lu, et la question posée
//!
//! | Lecture | Question |
//! |---|---|
//! | `programme.sessions` | quel créneau, quelle édition, quel état |
//! | `programme.registrations` | quels destinataires, et dans quel état |
//! | `event.events` | quelle édition porte la règle, et son périmètre |
//! | `org.memberships` | l'organisation qui anime a-t-elle le droit de lire son calendrier |
//! | `identity.people` | la langue préférée et l'adresse du destinataire |
//! | `platform.modules` | à quel module appartient un type de notification |
//!
//! Rien d'autre.

use kernel::error::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Une séance, réduite à ce dont les rappels ont besoin.
#[derive(Debug, Clone)]
pub struct SeanceVisee {
    pub event_id: Uuid,
    pub starts_at: OffsetDateTime,
    pub timezone: String,
    /// Valeur de `programme.session_status`, en texte.
    pub status: String,
    pub title: serde_json::Value,
    /// L'organisation qui anime, via le dossier d'origine. Nulle quand l'IFDD
    /// programme directement.
    pub organization_id: Option<Uuid>,
}

pub async fn seance(pool: &PgPool, session_id: Uuid) -> Result<Option<SeanceVisee>> {
    let ligne = sqlx::query!(
        r#"SELECT s.event_id,
                  s.starts_at,
                  s.timezone,
                  s.status::text AS "status!",
                  s.title,
                  (SELECT p.organization_id
                     FROM programme.proposals p
                    WHERE p.id = s.proposal_id) AS organization_id
             FROM programme.sessions s
            WHERE s.id = $1"#,
        session_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| SeanceVisee {
        event_id: l.event_id,
        starts_at: l.starts_at,
        timezone: l.timezone,
        status: l.status,
        title: l.title,
        organization_id: l.organization_id,
    }))
}

/// Ce qu'un rappel dit de sa séance, **déjà résolu dans la langue du
/// destinataire**.
///
/// L'instant est rendu **formaté et dans le fuseau de la séance** : le convertir
/// côté Rust demanderait une base de fuseaux horaires que ce dépôt n'embarque
/// pas, et une heure de Belém affichée en UTC dans un rappel de trente minutes
/// ferait manquer la séance. Le nom du fuseau voyage avec — une heure sans son
/// fuseau n'est pas une heure.
#[derive(Debug, Clone)]
pub struct SeancePourRappel {
    pub event_id: Uuid,
    pub titre: String,
    pub slug: String,
    pub event_slug: String,
    pub timezone: String,
    /// « 20/11/2027 à 14h30 » en français, « 2027-11-20 at 14:30 » en anglais.
    /// Un format numérique dans les deux langues : `FMMonth` dépend de
    /// `lc_time`, réglage du serveur qu'aucun test ne contrôle.
    pub debut_local: String,
    pub status: String,
}

pub async fn seance_pour_rappel(
    pool: &PgPool,
    session_id: Uuid,
    locale: &str,
) -> Result<Option<SeancePourRappel>> {
    let ligne = sqlx::query!(
        r#"SELECT s.event_id,
                  platform.t(s.title, $2) AS "titre!",
                  s.slug::text AS "slug!",
                  e.slug::text AS "event_slug!",
                  s.timezone::text AS "timezone!",
                  to_char(s.starts_at AT TIME ZONE s.timezone,
                          CASE WHEN $2 LIKE 'en%' THEN 'YYYY-MM-DD "at" HH24:MI'
                               ELSE 'DD/MM/YYYY "à" HH24"h"MI' END) AS "debut_local!",
                  s.status::text AS "status!"
             FROM programme.sessions s
             JOIN event.events e ON e.id = s.event_id
            WHERE s.id = $1"#,
        session_id,
        locale
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| SeancePourRappel {
        event_id: l.event_id,
        titre: l.titre,
        slug: l.slug,
        event_slug: l.event_slug,
        timezone: l.timezone,
        debut_local: l.debut_local,
        status: l.status,
    }))
}

/// Un destinataire de rappel.
#[derive(Debug, Clone)]
pub struct Destinataire {
    pub person_id: Uuid,
    pub registration_id: Uuid,
    /// Valeur de `programme.registration_status`, en texte. **C'est sur elle que
    /// le consommateur branche**, jamais sur le type d'événement (écart n° 126).
    pub status: String,
}

/// Les inscrits d'une séance qui ont droit à un rappel — ni annulés, ni en
/// liste d'attente. La comparaison est **textuelle**, comme dans la fonction du
/// modèle : le module `programme` peut faire évoluer son énuméré sans casser
/// ceci.
pub async fn inscrits_a_rappeler(pool: &PgPool, session_id: Uuid) -> Result<Vec<Destinataire>> {
    let lignes = sqlx::query!(
        r#"SELECT r.person_id, r.id AS registration_id, r.status::text AS "status!"
             FROM programme.registrations r
            WHERE r.session_id = $1
              AND r.status::text NOT IN ('cancelled', 'waitlisted')
            ORDER BY r.person_id"#,
        session_id
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Destinataire {
            person_id: l.person_id,
            registration_id: l.registration_id,
            status: l.status,
        })
        .collect())
}

/// **Les inscrits à prévenir d'un changement de séance** — la liste d'attente
/// comprise.
///
/// Elle diffère de [`inscrits_a_rappeler`], et la différence est voulue : une
/// personne en attente n'a pas de place, donc pas de rappel — mais une séance
/// annulée la concerne, et lui laisser croire qu'elle pourrait encore être
/// promue serait faux.
pub async fn inscrits_a_prevenir(pool: &PgPool, session_id: Uuid) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT r.person_id FROM programme.registrations r
          WHERE r.session_id = $1 AND r.status::text <> 'cancelled'
          ORDER BY r.person_id",
        session_id
    )
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

/// Une inscription précise : sa séance, sa personne, son état.
#[derive(Debug, Clone)]
pub struct InscriptionVisee {
    pub session_id: Uuid,
    pub person_id: Uuid,
    pub status: String,
    pub locale: String,
}

pub async fn inscription(pool: &PgPool, registration_id: Uuid) -> Result<Option<InscriptionVisee>> {
    let ligne = sqlx::query!(
        r#"SELECT r.session_id, r.person_id, r.status::text AS "status!", r.locale
             FROM programme.registrations r
            WHERE r.id = $1"#,
        registration_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| InscriptionVisee {
        session_id: l.session_id,
        person_id: l.person_id,
        status: l.status,
        locale: l.locale,
    }))
}

/// L'édition existe-t-elle ? Un identifiant inconnu se refuse comme un
/// identifiant hors périmètre — principe IX.
pub async fn edition_existe(pool: &PgPool, event_id: Uuid) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM event.events e WHERE e.id = $1) AS "existe!""#,
        event_id
    )
    .fetch_one(pool)
    .await?;
    Ok(existe)
}

/// L'adhésion est-elle **active** ?
///
/// Une organisation n'administre rien : son accès au calendrier des rappels de
/// sa séance passe par l'adhésion, pas par un périmètre. C'est la règle posée
/// par B4, réemployée telle quelle.
pub async fn adhesion_active(
    pool: &PgPool,
    person_id: Uuid,
    organization_id: Uuid,
) -> Result<bool> {
    let active = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM org.memberships m
                WHERE m.person_id = $1 AND m.organization_id = $2 AND m.status = 'active'
           ) AS "active!""#,
        person_id,
        organization_id
    )
    .fetch_one(pool)
    .await?;
    Ok(active)
}

/// Ce qu'il faut d'une personne pour lui écrire.
#[derive(Debug, Clone)]
pub struct DestinataireCourriel {
    pub email: String,
    pub locale: String,
    pub first_name: String,
}

pub async fn personne_pour_courriel(
    pool: &PgPool,
    person_id: Uuid,
) -> Result<Option<DestinataireCourriel>> {
    let ligne = sqlx::query!(
        r#"SELECT p.primary_email::text AS "email!", p.preferred_locale, p.first_name
             FROM identity.people p
            WHERE p.id = $1"#,
        person_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| DestinataireCourriel {
        email: l.email,
        locale: l.preferred_locale,
        first_name: l.first_name,
    }))
}

/// Le module d'un type de notification est-il déployé ?
///
/// Un avis dont le module est éteint ne part pas : l'écran vers lequel il
/// renvoie n'existe pas.
pub async fn module_deploye(pool: &PgPool, module_code: &str) -> Result<bool> {
    let deploye = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM platform.modules m
                WHERE m.code = $1 AND m.deployment <> 'disabled'
           ) AS "deploye!""#,
        module_code
    )
    .fetch_one(pool)
    .await?;
    Ok(deploye)
}

/// Un destinataire d'annonce : de quoi lui écrire et lui poser un avis.
#[derive(Debug, Clone)]
pub struct DestinataireDannonce {
    pub person_id: Uuid,
    pub email: String,
    pub locale: String,
}

/// **Les comptes actifs de la plateforme.**
///
/// Actifs et à l'adresse vérifiée : écrire à une adresse jamais confirmée est le
/// meilleur moyen de rebondir, et un rebond dur coûte à **tous** les envois du
/// domaine, confirmations d'inscription comprises.
pub async fn comptes_actifs(pool: &PgPool) -> Result<Vec<DestinataireDannonce>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id, p.primary_email::text AS "email!", p.preferred_locale
             FROM identity.people p
            WHERE p.status = 'active' AND p.email_verified_at IS NOT NULL
            ORDER BY p.id"#
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| DestinataireDannonce {
            person_id: l.id,
            email: l.email,
            locale: l.preferred_locale,
        })
        .collect())
}

/// Les inscrits des séances d'une édition, **une fois chacun**.
pub async fn inscrits_de_ledition(
    pool: &PgPool,
    event_id: Uuid,
) -> Result<Vec<DestinataireDannonce>> {
    let lignes = sqlx::query!(
        r#"SELECT DISTINCT p.id, p.primary_email::text AS "email!", p.preferred_locale
             FROM programme.registrations r
             JOIN programme.sessions s ON s.id = r.session_id
             JOIN identity.people p ON p.id = r.person_id
            WHERE s.event_id = $1
              AND r.status::text NOT IN ('cancelled')
              AND p.status = 'active'
            ORDER BY p.id"#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| DestinataireDannonce {
            person_id: l.id,
            email: l.email,
            locale: l.preferred_locale,
        })
        .collect())
}

/// Le motif d'annulation d'une séance, résolu dans la langue du destinataire.
/// Il ne voyage **pas** dans la charge utile de l'annonce : le déclencheur
/// n'émet que le créneau et le titre.
pub async fn motif_dannulation(
    pool: &PgPool,
    session_id: Uuid,
    locale: &str,
) -> Result<Option<String>> {
    let motif = sqlx::query_scalar!(
        "SELECT platform.t(s.cancelled_reason, $2)
           FROM programme.sessions s WHERE s.id = $1",
        session_id,
        locale
    )
    .fetch_optional(pool)
    .await?;

    Ok(motif.flatten())
}

/// Les séances d'une édition — de quoi rassembler les règles qui la
/// concernent, celle de l'édition et celles de ses séances.
pub async fn seances_de_ledition(pool: &PgPool, event_id: Uuid) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT s.id FROM programme.sessions s WHERE s.event_id = $1 ORDER BY s.starts_at",
        event_id
    )
    .fetch_all(pool)
    .await?;

    Ok(ids)
}
