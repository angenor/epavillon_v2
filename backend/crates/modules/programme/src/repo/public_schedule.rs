//! `programme.v_public_schedule`, **telle quelle**.
//!
//! # Chaque colonne s'annote, une par une
//!
//! Une **vue** ne porte aucune contrainte de nullité, et SQLx le suppose : sans
//! annotation, les vingt-huit colonnes traverseraient en `Option`, y compris
//! l'identifiant. C'est la leçon de B3, et elle vaut ici deux fois — la vue est
//! la plus large du modèle.
//!
//! # Ce que la vue fait, et qu'on ne refait pas
//!
//! Le filtre sur `published_at`, le repli de couverture sur le dossier
//! d'origine, l'état temporel, l'agrégation des journées spéciales publiées, le
//! décompte des inscrits et les deux formes de thématiques. Les réécrire ici
//! produirait une seconde vérité, et c'est exactement ce que la vue existe pour
//! empêcher.

use kernel::error::Result;
use sqlx::PgExecutor;

use crate::domain::ids::EventId;
use crate::domain::sessions::PublicScheduleRow;

/// La programmation publique d'une édition.
///
/// **Vide — jamais une erreur — quand le programme n'est pas paru** : aucune
/// séance ne porte de date de publication, et la vue ne rend rien. C'est un état
/// normal, que l'écran annonce.
/// **Sans édition, ce sont les séances À VENIR de toutes les éditions.**
///
/// L'accueil du site compose une seule liste des prochaines séances, toutes COP
/// confondues : il n'a pas d'édition à nommer, et lui en faire choisir une le
/// ferait passer à côté de celles des autres. La lecture est alors bornée — les
/// séances passées sont écartées et le nombre de lignes est plafonné —, sans
/// quoi elle rendrait la programmation entière de toutes les éditions de
/// l'histoire de la plateforme à chaque affichage de la page d'accueil.
pub async fn programmation<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Option<EventId>,
    // `limite` nulle = aucun plafond : `LIMIT NULL` vaut `LIMIT ALL` en
    // PostgreSQL, ce qui évite de composer deux requêtes pour une seule lecture.
    limite: Option<i64>,
) -> Result<Vec<PublicScheduleRow>> {
    let lignes = sqlx::query!(
        r#"SELECT v.id AS "id!", v.event_id AS "event_id!", v.event_day_id,
                  v.proposal_id, v.slug::text AS "slug!", v.title AS "title!",
                  v.summary, v.starts_at AS "starts_at!", v.ends_at AS "ends_at!",
                  v.timezone::text AS "timezone!", v.format::text AS "format!",
                  v.status::text AS "status!",
                  v.room_id, v.room_name, v.organization_id, v.organization_name,
                  v.organization_acronym, v.organization_country_code,
                  v.organization_country,
                  v.is_streamed AS "is_streamed!", v.broadcast_channel_id,
                  v.capacity::int4 AS "capacity?",
                  v.tracks AS "tracks!", v.cover,
                  v.temporal_state AS "temporal_state!",
                  v.registered_count AS "registered_count!",
                  v.theme_codes AS "theme_codes!", v.themes AS "themes!"
             FROM programme.v_public_schedule v
            WHERE ($1::uuid IS NULL OR v.event_id = $1)
              AND ($1::uuid IS NOT NULL OR v.temporal_state IN ('upcoming', 'ongoing'))
            ORDER BY v.starts_at, v.id
            LIMIT $2"#,
        event_id.map(|e| e.0),
        limite
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PublicScheduleRow {
            id: l.id,
            event_id: l.event_id,
            event_day_id: l.event_day_id,
            proposal_id: l.proposal_id,
            slug: l.slug,
            title: l.title,
            summary: l.summary,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            timezone: l.timezone,
            format: l.format,
            status: l.status,
            room_id: l.room_id,
            room_name: l.room_name,
            organization_id: l.organization_id,
            organization_name: l.organization_name,
            organization_acronym: l.organization_acronym,
            organization_country_code: l.organization_country_code,
            organization_country: l.organization_country,
            is_streamed: l.is_streamed,
            broadcast_channel_id: l.broadcast_channel_id,
            capacity: l.capacity,
            tracks: l.tracks,
            cover: l.cover,
            temporal_state: l.temporal_state,
            registered_count: l.registered_count,
            theme_codes: l.theme_codes,
            themes: l.themes,
        })
        .collect())
}

/// Une séance **publiée**, par son adresse d'URL dans son édition.
///
/// **Une adresse inconnue et une séance non publiée rendent le même refus** : la
/// vue filtre déjà sur `published_at`, et le service n'a donc rien à distinguer.
pub async fn par_adresse<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
    slug: &str,
) -> Result<Option<PublicScheduleRow>> {
    // **Aucun plafond ici.** Cette lecture cherche une séance précise dans une
    // édition : la borner ferait rendre « introuvable » pour une séance qui
    // existe, simplement parce qu'elle arrive tard dans le programme.
    Ok(programmation(executor, Some(event_id), None)
        .await?
        .into_iter()
        .find(|s| s.slug == slug))
}
