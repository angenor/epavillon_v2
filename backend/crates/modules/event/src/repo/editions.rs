//! Lectures et écritures de `event.events`.
//!
//! Ce que le dépôt **n'écrit jamais** : `programme_published_at`, posée par la
//! publication seule (research.md § R10), et les horodatages, posés par la base.
//!
//! **Aucun SQL n'est composé ici.** La charge utile d'une édition est totale
//! (§ R13), donc la mise à jour est un `UPDATE` complet écrit une fois — pas de
//! `UPDATE` par combinaison de champs, pas de chaîne assemblée à l'exécution.

use kernel::auth::AdminScope;
use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::domain::edition::{
    CountryOption, EditionFormPayload, EditionSeriesOption, EventEdition, TimeZoneOption,
};
use crate::domain::ids::EventId;

/// L'état d'une édition **avant** l'écriture, réduit à ce que l'annonce doit
/// comparer. Lu dans la transaction : sans lui, `event.edition.updated` ne
/// pourrait dire *ce qui* a changé, et le consommateur devrait tout relire.
#[derive(Debug, Clone)]
pub struct EtatAvant {
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub status: String,
    pub has_pavilion: bool,
    pub timezone: String,
}

pub async fn etat_avant(conn: &mut PgConnection, id: EventId) -> Result<Option<EtatAvant>> {
    let ligne = sqlx::query!(
        r#"SELECT starts_at, ends_at, status::text AS "status!", has_pavilion,
                  timezone::text AS "timezone!"
             FROM event.events WHERE id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(&mut *conn)
    .await?;

    Ok(ligne.map(|l| EtatAvant {
        starts_at: l.starts_at,
        ends_at: l.ends_at,
        status: l.status,
        has_pavilion: l.has_pavilion,
        timezone: l.timezone,
    }))
}

/// Création. `created_by` vient du **contexte de session**, jamais de la charge
/// utile : `xmod_fk_events_creator` ne doit pas pouvoir être violée par un
/// client.
///
/// **L'erreur est rendue BRUTE**, et c'est délibéré : seul le service sait à
/// quel champ du formulaire une contrainte se rapporte, et la traduire ici
/// perdrait le nom de la contrainte — la seule chose sur laquelle on ait le
/// droit de brancher. C'est l'idiome posé en B1 (`repo/admin_users.rs`).
pub async fn inserer(
    conn: &mut PgConnection,
    p: &EditionFormPayload,
    created_by: Uuid,
) -> std::result::Result<EventId, sqlx::Error> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at,
                country_id, city, address, latitude, longitude, has_pavilion,
                highlights, created_by)
           VALUES ($1, $2, $3, $4::jsonb, $5, $6::text::platform.slug, $7::jsonb,
                   $8::text::event.event_status, $9::text::event.participation_mode,
                   $10::text::platform.timezone_name, $11, $12,
                   $13, $14, $15, $16::float8::numeric(9,6), $17::float8::numeric(9,6), $18,
                   $19::jsonb, $20)
        RETURNING id"#,
        p.series_id,
        p.edition_label,
        p.edition_year,
        p.title,
        p.acronym,
        p.slug,
        p.description,
        p.status,
        p.participation_mode,
        p.timezone,
        p.starts_at,
        p.ends_at,
        p.country_id,
        p.city,
        p.address,
        p.latitude,
        p.longitude,
        p.has_pavilion,
        p.highlights,
        created_by
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(EventId::from(id))
}

/// Modification **totale**. Tous les champs modifiables sont réécrits, y compris
/// à `NULL` : c'est ce qui permet d'effacer un sigle, une ville ou des
/// coordonnées. Un `SET x = COALESCE($1, x)` rendrait la mise à nul impossible.
/// L'erreur est rendue brute, pour la même raison qu'à l'insertion.
pub async fn modifier(
    conn: &mut PgConnection,
    id: EventId,
    p: &EditionFormPayload,
) -> std::result::Result<bool, sqlx::Error> {
    let touchees = sqlx::query!(
        r#"UPDATE event.events SET
               series_id          = $2,
               edition_label      = $3,
               edition_year       = $4,
               title              = $5::jsonb,
               acronym            = $6,
               slug               = $7::text::platform.slug,
               description        = $8::jsonb,
               status             = $9::text::event.event_status,
               participation_mode = $10::text::event.participation_mode,
               timezone           = $11::text::platform.timezone_name,
               starts_at          = $12,
               ends_at            = $13,
               country_id         = $14,
               city               = $15,
               address            = $16,
               latitude           = $17::float8::numeric(9,6),
               longitude          = $18::float8::numeric(9,6),
               has_pavilion       = $19,
               highlights         = $20::jsonb
         WHERE id = $1"#,
        id.as_uuid(),
        p.series_id,
        p.edition_label,
        p.edition_year,
        p.title,
        p.acronym,
        p.slug,
        p.description,
        p.status,
        p.participation_mode,
        p.timezone,
        p.starts_at,
        p.ends_at,
        p.country_id,
        p.city,
        p.address,
        p.latitude,
        p.longitude,
        p.has_pavilion,
        p.highlights
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// **La période en dates civiles, dans le fuseau de l'édition** — calculée
/// EN BASE (research.md § R5).
///
/// La calculer en Rust demanderait une base de fuseaux qui n'est pas celle de
/// PostgreSQL, et c'est exactement le défaut qui a fait tomber le formulaire du
/// front sur `Europe/Geneva`. Ici, `platform.timezone_name` a déjà validé
/// l'identifiant contre cette base : le même dictionnaire sert à vérifier et à
/// convertir.
pub async fn periode_civile<'e>(executor: impl PgExecutor<'e>, id: EventId) -> Result<Vec<Date>> {
    let jours = sqlx::query_scalar!(
        r#"SELECT j::date AS "jour!"
             FROM event.events e,
                  LATERAL generate_series(
                      (e.starts_at AT TIME ZONE e.timezone::text)::date,
                      (e.ends_at   AT TIME ZONE e.timezone::text)::date,
                      interval '1 day') AS j
            WHERE e.id = $1
            ORDER BY 1"#,
        id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(jours)
}

/// Les colonnes d'une ligne de liste **hors décomptes** : l'édition, sa série
/// résolue, son pays résolu et son appel non annulé. Les décomptes vivent dans
/// `repo/cross.rs`, où la frontière se relit.
#[derive(Debug, Clone)]
pub struct LigneBase {
    pub id: Uuid,
    pub title: serde_json::Value,
    pub acronym: Option<String>,
    pub slug: String,
    pub series_id: Option<Uuid>,
    pub series_name: Option<serde_json::Value>,
    pub series_kind: Option<String>,
    /// **Nécessaire à la facette**, jamais à la ligne : le filtre distingue une
    /// série vive d'une série retirée du catalogue.
    pub series_is_active: Option<bool>,
    pub edition_label: Option<String>,
    pub edition_year: i16,
    pub status: String,
    pub participation_mode: String,
    pub timezone: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub country_id: Option<Uuid>,
    pub country_name: Option<serde_json::Value>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub has_pavilion: bool,
    pub programme_published_at: Option<OffsetDateTime>,
    pub call_status: Option<String>,
    pub call_deadline: Option<OffsetDateTime>,
}

/// Une ligne, par son identifiant.
pub async fn ligne<'e>(executor: impl PgExecutor<'e>, id: EventId) -> Result<Option<LigneBase>> {
    let mut lignes = lire(executor, true, &[], Some(id.as_uuid())).await?;
    Ok(lignes.pop())
}

/// **La liste, bornée par le périmètre d'administration** (règle métier n° 8).
///
/// Le filtre est le filtre canonique du noyau — `is_global OR id = ANY(ids)` —,
/// et il est posé **dans la requête** : le poser après coup ramènerait d'abord
/// toutes les éditions de la plateforme, ce qui est exactement ce qu'on veut
/// éviter.
///
/// Les trois cas du périmètre restent distincts **au-dessus** : un périmètre
/// vide ne descend jamais jusqu'ici, il est refusé par le garde (FR-008). Cette
/// fonction ne connaît donc que « tout » et « ces éditions-là ».
///
/// **Décroissante sur la date de début** : la prochaine COP est ce qu'on vient
/// chercher, pas la plus ancienne.
pub async fn lignes_du_perimetre<'e>(
    executor: impl PgExecutor<'e>,
    perimetre: &AdminScope,
) -> Result<Vec<LigneBase>> {
    lire(executor, perimetre.is_global, &perimetre.event_ids, None).await
}

/// La requête des deux lectures ci-dessus, écrite **une fois**.
///
/// Vingt-cinq colonnes recopiées dans deux requêtes voisines, ce sont deux
/// occasions de diverger — et le jour où l'une gagne une colonne, l'écran de
/// détail et l'écran de liste cessent de dire la même chose de la même édition.
///
/// **Jointures par la gauche** : une édition hors série et une édition en ligne
/// n'ont ni série ni pays, et une jointure interne les ferait disparaître de
/// leur propre liste.
///
/// L'appel est celui qui n'est **pas annulé** — `ux_calls_one_per_event` exclut
/// les annulés, il y en a donc au plus un —, et son échéance vient de
/// `event.effective_deadline()`, appelée et jamais recalculée.
async fn lire<'e>(
    executor: impl PgExecutor<'e>,
    est_globale: bool,
    editions: &[Uuid],
    une_seule: Option<Uuid>,
) -> Result<Vec<LigneBase>> {
    let lignes = sqlx::query!(
        r#"SELECT e.id, e.title, e.acronym, e.slug::text AS "slug!",
                  e.series_id, s.name AS "series_name?", s.kind::text AS "series_kind?",
                  s.is_active AS "series_is_active?",
                  e.edition_label, e.edition_year,
                  e.status::text AS "status!",
                  e.participation_mode::text AS "participation_mode!",
                  e.timezone::text AS "timezone!",
                  e.starts_at, e.ends_at,
                  e.country_id, c.name AS "country_name?",
                  e.city, e.address,
                  e.latitude::float8 AS "latitude?", e.longitude::float8 AS "longitude?",
                  e.has_pavilion, e.programme_published_at,
                  cfp.status::text AS "call_status?",
                  event.effective_deadline(cfp.id) AS "call_deadline?"
             FROM event.events e
             LEFT JOIN event.event_series s ON s.id = e.series_id
             LEFT JOIN reference.countries c ON c.id = e.country_id
             LEFT JOIN event.calls_for_proposals cfp
                    ON cfp.event_id = e.id AND cfp.status <> 'cancelled'
            WHERE ($1::boolean OR e.id = ANY($2::uuid[]))
              AND ($3::uuid IS NULL OR e.id = $3)
            ORDER BY e.starts_at DESC"#,
        est_globale,
        editions,
        une_seule
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| LigneBase {
            id: l.id,
            title: l.title,
            acronym: l.acronym,
            slug: l.slug,
            series_id: l.series_id,
            series_name: l.series_name,
            series_kind: l.series_kind,
            series_is_active: l.series_is_active,
            edition_label: l.edition_label,
            edition_year: l.edition_year,
            status: l.status,
            participation_mode: l.participation_mode,
            timezone: l.timezone,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            country_id: l.country_id,
            country_name: l.country_name,
            city: l.city,
            address: l.address,
            latitude: l.latitude,
            longitude: l.longitude,
            has_pavilion: l.has_pavilion,
            programme_published_at: l.programme_published_at,
            call_status: l.call_status,
            call_deadline: l.call_deadline,
        })
        .collect())
}

/// Les **deux textes longs** d'une édition, lus par le détail et par lui seul.
///
/// Ils ne sont pas sur la ligne de liste (FR-021) : un tableau à huit colonnes
/// n'a pas à charger deux paragraphes par édition, et le formulaire de
/// modification, lui, en a besoin.
pub async fn textes<'e>(
    executor: impl PgExecutor<'e>,
    id: EventId,
) -> Result<Option<(serde_json::Value, Option<serde_json::Value>)>> {
    let ligne = sqlx::query!(
        r#"SELECT description AS "description!", highlights
             FROM event.events WHERE id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| (l.description, l.highlights)))
}

/// **Le sélecteur d'édition du back-office**, borné par le périmètre.
///
/// C'est la **seule** lecture du module où un périmètre vide rend une liste vide
/// au lieu d'un refus : le contrat du front le veut ainsi, et c'est le store qui
/// décide alors de l'écran. Partout ailleurs, un périmètre vide se refuse
/// explicitement (FR-008).
///
/// La forme est celle de la table, sans décompte ni série résolue : un menu
/// déroulant n'en a pas besoin.
pub async fn selecteur<'e>(
    executor: impl PgExecutor<'e>,
    perimetre: &AdminScope,
) -> Result<Vec<EventEdition>> {
    let lignes = sqlx::query!(
        r#"SELECT id, series_id, edition_label, edition_year, title, acronym,
                  slug::text AS "slug!", description AS "description!",
                  status::text AS "status!",
                  participation_mode::text AS "participation_mode!",
                  timezone::text AS "timezone!",
                  starts_at, ends_at, country_id, city, address,
                  latitude::float8 AS "latitude?", longitude::float8 AS "longitude?",
                  has_pavilion, programme_published_at, highlights,
                  created_by, created_at, updated_at
             FROM event.events
            WHERE $1::boolean OR id = ANY($2::uuid[])
            ORDER BY starts_at DESC"#,
        perimetre.is_global,
        &perimetre.event_ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| EventEdition {
            id: l.id,
            series_id: l.series_id,
            edition_label: l.edition_label,
            edition_year: l.edition_year,
            title: l.title,
            acronym: l.acronym,
            slug: l.slug,
            description: l.description,
            status: l.status,
            participation_mode: l.participation_mode,
            timezone: l.timezone,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            country_id: l.country_id,
            city: l.city,
            address: l.address,
            latitude: l.latitude,
            longitude: l.longitude,
            has_pavilion: l.has_pavilion,
            programme_published_at: l.programme_published_at,
            highlights: l.highlights,
            created_by: l.created_by,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}

// -----------------------------------------------------------------------------
// Options du formulaire
// -----------------------------------------------------------------------------

/// Les séries, avec leur **décompte d'éditions** : ce qui distingue une série
/// vive d'une coquille (FR-015).
pub async fn series<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<EditionSeriesOption>> {
    let lignes = sqlx::query!(
        r#"SELECT s.id, s.name, s.kind::text AS "kind!", s.is_active,
                  count(e.id) AS "edition_count!"
             FROM event.event_series s
             LEFT JOIN event.events e ON e.series_id = s.id
            GROUP BY s.id, s.name, s.kind, s.is_active
            ORDER BY s.kind, s.name->>'fr'"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| EditionSeriesOption {
            id: l.id,
            name: l.name,
            kind: l.kind,
            is_active: l.is_active,
            edition_count: l.edition_count,
        })
        .collect())
}

/// Les fuseaux, **lus dans le dictionnaire de PostgreSQL** — celui-là même
/// contre lequel `platform.timezone_name` vérifie ce qu'on écrit.
///
/// Les entrées `posix/` sont écartées : elles doublent les mêmes fuseaux sous
/// un autre préfixe et n'ajoutent rien à une liste de saisie. Les abréviations
/// sans barre oblique — `EST5EDT`, `CET` — le sont aussi : elles ne désignent
/// pas un lieu.
pub async fn fuseaux<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<TimeZoneOption>> {
    let lignes = sqlx::query!(
        r#"SELECT name AS "name!", utc_offset AS "utc_offset!"
             FROM pg_timezone_names
            WHERE name LIKE '%/%' AND name NOT LIKE 'posix/%'
            ORDER BY name"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| TimeZoneOption {
            city: ville_du_fuseau(&l.name),
            offset_label: libelle_de_decalage(minutes_de(&l.utc_offset)),
            value: l.name,
        })
        .collect())
}

fn ville_du_fuseau(nom: &str) -> String {
    nom.rsplit('/').next().unwrap_or(nom).replace('_', " ")
}

/// Le décalage d'un fuseau en minutes. Un intervalle PostgreSQL porte trois
/// composantes ; celle des mois n'a aucun sens pour un décalage horaire et vaut
/// toujours zéro ici, mais l'ignorer sans le dire ferait croire à un oubli.
fn minutes_de(intervalle: &sqlx::postgres::types::PgInterval) -> i64 {
    let secondes = i64::from(intervalle.days) * 86_400 + intervalle.microseconds / 1_000_000;
    secondes / 60
}

/// « UTC−03:00 ». Le signe est le vrai signe moins typographique, comme partout
/// ailleurs dans l'interface.
fn libelle_de_decalage(total_minutes: i64) -> String {
    let signe = if total_minutes < 0 { '−' } else { '+' };
    let absolu = total_minutes.abs();
    format!("UTC{signe}{:02}:{:02}", absolu / 60, absolu % 60)
}

/// Les statuts atteignables, **dans l'ordre où l'énuméré du modèle les
/// déclare** — qui est l'ordre du cycle de vie d'une édition. Les recopier ici
/// ferait un second vocabulaire, à désynchroniser au premier ajout.
pub async fn statuts<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<String>> {
    let statuts = sqlx::query_scalar!(
        r#"SELECT unnest(enum_range(NULL::event.event_status))::text AS "statut!""#
    )
    .fetch_all(executor)
    .await?;

    Ok(statuts)
}

/// Les pays du référentiel. Lecture hors schéma **déclarée** : elle répond à une
/// question de ce module — « où cette édition se tient-elle ? » — et vit donc
/// aussi dans l'inventaire de `repo/cross.rs`.
pub async fn pays<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<CountryOption>> {
    crate::repo::cross::pays_du_referentiel(executor).await
}

/// **Estampiller la programmation comme publiée — une seule fois.**
///
/// La clause `WHERE programme_published_at IS NULL` rend la republication
/// inoffensive : la date d'origine ne s'écrase pas, et c'est elle que la frise
/// d'accueil affiche. Rend l'instant posé, ou `None` si l'édition portait déjà
/// une date — auquel cas rien n'a été écrit, et rien ne doit être annoncé.
pub async fn estampiller_la_publication(
    conn: &mut PgConnection,
    id: EventId,
) -> Result<Option<OffsetDateTime>> {
    let pose = sqlx::query_scalar!(
        "UPDATE event.events SET programme_published_at = now()
          WHERE id = $1 AND programme_published_at IS NULL
        RETURNING programme_published_at",
        id.as_uuid()
    )
    .fetch_optional(&mut *conn)
    .await?;

    Ok(pose.flatten())
}

/// La date de publication déjà posée, s'il y en a une. C'est ce que rend une
/// republication : la date d'origine, intacte.
pub async fn date_de_publication<'e>(
    executor: impl PgExecutor<'e>,
    id: EventId,
) -> Result<Option<OffsetDateTime>> {
    let date = sqlx::query_scalar!(
        "SELECT programme_published_at FROM event.events WHERE id = $1",
        id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(date.flatten())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_ville_est_le_dernier_segment_sans_tiret_bas() {
        assert_eq!(ville_du_fuseau("America/Belem"), "Belem");
        assert_eq!(ville_du_fuseau("America/Port-au-Prince"), "Port-au-Prince");
        assert_eq!(ville_du_fuseau("America/Argentina/Cordoba"), "Cordoba");
        assert_eq!(ville_du_fuseau("Africa/Porto-Novo"), "Porto-Novo");
        assert_eq!(ville_du_fuseau("Pacific/Port_Moresby"), "Port Moresby");
    }

    /// Belém est trois heures derrière l'UTC ; Katmandou en avance de cinq
    /// heures quarante-cinq. Un fuseau à minutes n'est pas une curiosité.
    #[test]
    fn le_decalage_porte_son_signe_et_ses_minutes() {
        assert_eq!(libelle_de_decalage(-180), "UTC−03:00");
        assert_eq!(libelle_de_decalage(60), "UTC+01:00");
        assert_eq!(libelle_de_decalage(345), "UTC+05:45");
        assert_eq!(libelle_de_decalage(0), "UTC+00:00");
    }

    #[test]
    fn lintervalle_de_postgres_se_ramene_a_des_minutes() {
        use sqlx::postgres::types::PgInterval;

        let trois_heures_derriere = PgInterval {
            months: 0,
            days: 0,
            microseconds: -3 * 3_600 * 1_000_000,
        };
        assert_eq!(minutes_de(&trois_heures_derriere), -180);
    }
}
