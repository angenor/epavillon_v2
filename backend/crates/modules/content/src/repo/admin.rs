//! Les lectures et les écritures du back-office de la vitrine.
//!
//! # Le périmètre est dans la requête, jamais après
//!
//! `WHERE $1 OR h.event_id = ANY($2)` : le filtre d'administration voyage dans
//! le SQL, comme partout dans le projet. Filtrer après coup, c'est charger ce
//! qu'on n'a pas le droit de lire et compter dessus par accident.
//!
//! **Un contenu de plateforme (`event_id` nul) n'est visible qu'en portée
//! globale.** Il parle au nom de la plateforme entière : le laisser voir à une
//! administratrice d'un seul événement lui montrerait ce qu'elle ne peut pas
//! modifier.
//!
//! # L'ordre se renumérote, il ne s'échange pas
//!
//! Monter d'un cran échange deux rangs, puis TOUT l'emplacement est renuméroté
//! par pas de dix. Sans cette remise à plat, une insertion finit par tomber sur
//! un rang déjà pris et l'ordre devient celui de `id` — c'est ce qui est arrivé
//! au carrousel de la v1.

use kernel::error::Result;
use serde_json::Value;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin::{
    etat_de_diffusion, ShowcaseCountryOption, ShowcaseEventOption, ShowcaseFormValues,
    ShowcaseListRow, ShowcaseMediaSlot, ShowcaseNatureOption, ShowcaseOrganizationOption,
    ShowcasePersonOption, ShowcaseSessionOption,
};

/// Pas de renumérotation. Dix, pour qu'une insertion manuelle reste possible
/// entre deux rangs sans tout redéplacer.
const PAS: i16 = 10;

// ---------------------------------------------------------------------------
// Lecture — la liste
// ---------------------------------------------------------------------------

/// Toutes les lignes du périmètre, triées par emplacement puis rang, avec
/// `is_first` / `is_last` déjà posés.
pub async fn lignes<'e>(
    executor: impl PgExecutor<'e>,
    is_global: bool,
    event_ids: &[Uuid],
    maintenant: OffsetDateTime,
) -> Result<Vec<ShowcaseListRow>> {
    let lignes = sqlx::query!(
        r#"SELECT h.id, h.placement::text AS "placement!", h.status::text AS "status!",
                  h.sort_order AS "sort_order!", h.nature_code AS "nature_code!",
                  n.label AS "nature_label?", n.color_hex AS nature_color, n.icon AS nature_icon,
                  h.title AS "title!",
                  COALESCE(p.display_name, h.author_name) AS author_name,
                  h.author_title,
                  COALESCE(o.legal_name, h.organization_label) AS organization_name,
                  o.acronym AS organization_acronym,
                  c.name AS "country_name?",
                  h.event_id, e.title AS "event_title?", e.slug::text AS "event_slug?",
                  h.session_id, s.title AS "session_title?",
                  media.attached_image('content', 'highlights', h.id, 'cover')  AS thumbnail,
                  media.attached_image('content', 'highlights', h.id, 'banner') AS background_image,
                  (media.attached_image('content', 'highlights', h.id, 'video') IS NOT NULL) AS "has_video!",
                  h.background_color_hex,
                  h.starts_at, h.ends_at, h.published_at, h.updated_at AS "updated_at!"
             FROM content.highlights h
             LEFT JOIN reference.taxonomy_terms n
                    ON n.taxonomy_code = 'highlight_nature' AND n.code = h.nature_code AND n.is_active
             LEFT JOIN identity.people p     ON p.id = h.person_id
             LEFT JOIN org.organizations o   ON o.id = h.organization_id
             LEFT JOIN reference.countries c ON c.id = COALESCE(h.country_id, o.country_id)
             LEFT JOIN event.events e        ON e.id = h.event_id
             LEFT JOIN programme.sessions s  ON s.id = h.session_id
            WHERE ($1::bool AND TRUE)
               OR (h.event_id IS NOT NULL AND h.event_id = ANY($2::uuid[]))
            ORDER BY h.placement, h.sort_order, h.id"#,
        is_global,
        event_ids
    )
    .fetch_all(executor)
    .await?;

    let mut rows: Vec<ShowcaseListRow> = lignes
        .into_iter()
        .map(|l| ShowcaseListRow {
            broadcast_state: etat_de_diffusion(&l.status, l.starts_at, l.ends_at, maintenant)
                .to_owned(),
            id: l.id,
            placement: l.placement,
            status: l.status,
            sort_order: l.sort_order,
            nature_code: l.nature_code,
            nature_label: l.nature_label,
            nature_color: l.nature_color,
            nature_icon: l.nature_icon,
            title: l.title,
            author_name: l.author_name,
            author_title: l.author_title,
            organization_name: l.organization_name,
            organization_acronym: l.organization_acronym,
            country_name: l.country_name,
            event_id: l.event_id,
            event_title: l.event_title,
            event_slug: l.event_slug,
            session_id: l.session_id,
            session_title: l.session_title,
            thumbnail: l.thumbnail,
            background_image: l.background_image,
            has_video: l.has_video,
            background_color_hex: l.background_color_hex,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            published_at: l.published_at,
            updated_at: l.updated_at,
            is_first: false,
            is_last: false,
        })
        .collect();

    poser_les_extremites(&mut rows);
    Ok(rows)
}

/// `is_first` / `is_last`, par emplacement. Les lignes arrivent déjà triées.
fn poser_les_extremites(rows: &mut [ShowcaseListRow]) {
    let emplacements: Vec<String> = {
        let mut vus: Vec<String> = rows.iter().map(|r| r.placement.clone()).collect();
        vus.dedup();
        vus
    };
    for emplacement in emplacements {
        let indices: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.placement == emplacement)
            .map(|(i, _)| i)
            .collect();
        if let Some(&premier) = indices.first() {
            rows[premier].is_first = true;
        }
        if let Some(&dernier) = indices.last() {
            rows[dernier].is_last = true;
        }
    }
}

/// Une ligne, par identifiant — la forme que la liste affiche.
pub async fn ligne<'e>(
    executor: impl PgExecutor<'e>,
    id: Uuid,
    is_global: bool,
    event_ids: &[Uuid],
    maintenant: OffsetDateTime,
) -> Result<Option<ShowcaseListRow>> {
    let toutes = lignes(executor, is_global, event_ids, maintenant).await?;
    Ok(toutes.into_iter().find(|r| r.id == id))
}

// ---------------------------------------------------------------------------
// Lecture — le formulaire
// ---------------------------------------------------------------------------

/// Les valeurs saisissables d'une diapositive, thématiques comprises.
pub async fn valeurs<'e>(
    executor: impl PgExecutor<'e>,
    id: Uuid,
) -> Result<Option<ShowcaseFormValues>> {
    let ligne = sqlx::query!(
        r#"SELECT h.id, h.placement::text AS "placement!", h.status::text AS "status!",
                  h.nature_code AS "nature_code!", h.sort_order AS "sort_order!",
                  h.title AS "title!", h.quote, h.body,
                  h.person_id, h.author_name, h.author_title,
                  h.organization_id, h.organization_label, h.country_id,
                  h.event_id, h.session_id, h.link_url::text AS "link_url?", h.link_label,
                  h.background_color_hex, h.starts_at, h.ends_at,
                  reference.terms_of('content', 'highlights', h.id, 'activity_theme') AS "theme_codes!"
             FROM content.highlights h
            WHERE h.id = $1"#,
        id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| ShowcaseFormValues {
        id: Some(l.id),
        placement: l.placement,
        status: l.status,
        nature_code: l.nature_code,
        sort_order: l.sort_order,
        title: l.title,
        quote: l.quote,
        body: l.body,
        person_id: l.person_id,
        author_name: l.author_name,
        author_title: l.author_title,
        organization_id: l.organization_id,
        organization_label: l.organization_label,
        country_id: l.country_id,
        event_id: l.event_id,
        session_id: l.session_id,
        link_url: l.link_url,
        link_label: l.link_label,
        background_color_hex: l.background_color_hex,
        starts_at: l.starts_at,
        ends_at: l.ends_at,
        theme_codes: l.theme_codes,
    }))
}

/// L'édition portée par une diapositive — `None` si elle n'existe pas, et
/// `Some(None)` pour un contenu de plateforme.
pub async fn edition_de<'e>(
    executor: impl PgExecutor<'e>,
    id: Uuid,
) -> Result<Option<Option<Uuid>>> {
    let ligne = sqlx::query!("SELECT event_id FROM content.highlights WHERE id = $1", id)
        .fetch_optional(executor)
        .await?;
    Ok(ligne.map(|l| l.event_id))
}

// ---------------------------------------------------------------------------
// Lecture — les référentiels du formulaire
// ---------------------------------------------------------------------------

pub async fn natures<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<ShowcaseNatureOption>> {
    let lignes = sqlx::query!(
        r#"SELECT code AS "code!", label AS "label!", color_hex, icon
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = 'highlight_nature' AND is_active
            ORDER BY sort_order, code"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ShowcaseNatureOption {
            code: l.code,
            label: l.label,
            color: l.color_hex,
            icon: l.icon,
        })
        .collect())
}

/// Les éditions du périmètre — celles auxquelles une diapositive peut se
/// rattacher.
pub async fn editions<'e>(
    executor: impl PgExecutor<'e>,
    is_global: bool,
    event_ids: &[Uuid],
) -> Result<Vec<ShowcaseEventOption>> {
    let lignes = sqlx::query!(
        r#"SELECT id, title AS "title!", acronym, edition_year AS "edition_year!",
                  slug::text AS "slug!"
             FROM event.events
            WHERE ($1::bool OR id = ANY($2::uuid[]))
              AND status <> 'draft'
            ORDER BY starts_at DESC"#,
        is_global,
        event_ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ShowcaseEventOption {
            id: l.id,
            title: l.title,
            acronym: l.acronym,
            edition_year: l.edition_year,
            slug: l.slug,
        })
        .collect())
}

/// Les séances **publiées** d'une édition, pour la cascade « édition → séance ».
pub async fn seances<'e>(
    executor: impl PgExecutor<'e>,
    event_id: Uuid,
) -> Result<Vec<ShowcaseSessionOption>> {
    let lignes = sqlx::query!(
        r#"SELECT id, event_id AS "event_id!", title AS "title!",
                  starts_at AS "starts_at!", timezone::text AS "timezone!"
             FROM programme.sessions
            WHERE event_id = $1 AND published_at IS NOT NULL
            ORDER BY starts_at"#,
        event_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ShowcaseSessionOption {
            id: l.id,
            event_id: l.event_id,
            title: l.title,
            starts_at: l.starts_at,
            timezone: l.timezone,
        })
        .collect())
}

/// Le répertoire, borné : un formulaire n'a pas besoin de cinq mille lignes
/// dans un menu déroulant, et la recherche fine a son propre écran.
pub async fn organisations<'e>(
    executor: impl PgExecutor<'e>,
) -> Result<Vec<ShowcaseOrganizationOption>> {
    let lignes = sqlx::query!(
        r#"SELECT id, legal_name AS "legal_name!", acronym
             FROM org.organizations
            ORDER BY legal_name
            LIMIT 500"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ShowcaseOrganizationOption {
            id: l.id,
            legal_name: l.legal_name,
            acronym: l.acronym,
        })
        .collect())
}

pub async fn personnes<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<ShowcasePersonOption>> {
    let lignes = sqlx::query!(
        r#"SELECT id, display_name AS "display_name!"
             FROM identity.people
            WHERE status = 'active'
            ORDER BY display_name
            LIMIT 500"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ShowcasePersonOption {
            id: l.id,
            display_name: l.display_name,
        })
        .collect())
}

pub async fn pays<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<ShowcaseCountryOption>> {
    let lignes = sqlx::query!(
        r#"SELECT id, iso2 AS "iso2!", name AS "name!"
             FROM reference.countries
            ORDER BY iso2"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ShowcaseCountryOption {
            id: l.id,
            iso2: l.iso2,
            name: l.name,
        })
        .collect())
}

/// Les thématiques offertes — `reference.term_badges()` n'accepte qu'une entité
/// existante, la liste est donc composée depuis la taxonomie elle-même.
pub async fn themes_disponibles<'e>(executor: impl PgExecutor<'e>) -> Result<Value> {
    let ligne = sqlx::query!(
        r#"SELECT COALESCE(jsonb_agg(jsonb_build_object(
                      'code', t.code, 'label', t.label,
                      'color', t.color_hex, 'icon', t.icon)
                      ORDER BY t.sort_order, t.code), '[]'::jsonb) AS "themes!"
             FROM reference.taxonomy_terms t
            WHERE t.taxonomy_code = 'activity_theme' AND t.is_active"#
    )
    .fetch_one(executor)
    .await?;

    Ok(ligne.themes)
}

/// Les trois emplacements de média du modèle, et ce qui y est rattaché.
pub async fn media<'e>(
    executor: impl PgExecutor<'e>,
    highlight_id: Option<Uuid>,
) -> Result<Vec<ShowcaseMediaSlot>> {
    let lignes = sqlx::query!(
        r#"SELECT r.role::text AS "role!", r.label AS "label!",
                  r.allowed_mime_prefixes AS "allowed_mime_prefixes!",
                  r.max_byte_size AS "max_byte_size!",
                  CASE WHEN $1::uuid IS NULL THEN NULL
                       ELSE media.attached_image('content', 'highlights', $1, r.role) END AS current,
                  COALESCE((SELECT count(*) > 0
                              FROM media.attachments a
                              JOIN media.assets s ON s.id = a.asset_id
                             WHERE a.owner_schema = 'content' AND a.owner_table = 'highlights'
                               AND a.owner_id = $1 AND a.role::text = r.role::text
                               AND s.status <> 'ready' AND s.deleted_at IS NULL), false) AS "is_pending!"
             FROM media.attachable_roles r
            WHERE r.owner_schema = 'content' AND r.owner_table = 'highlights' AND r.is_active
            ORDER BY r.role"#,
        highlight_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ShowcaseMediaSlot {
            role: l.role,
            label: l.label,
            allowed_mime_prefixes: l.allowed_mime_prefixes,
            max_byte_size: l.max_byte_size,
            current: l.current,
            is_pending: l.is_pending,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Écriture
// ---------------------------------------------------------------------------

/// Crée une diapositive **en fin d'emplacement**. La placer en tête
/// déplacerait silencieusement tout le reste du bandeau.
pub async fn creer(
    tx: &mut sqlx::PgConnection,
    valeurs: &ShowcaseFormValues,
    auteur: Uuid,
) -> Result<Uuid> {
    let rang = prochain_rang(&mut *tx, &valeurs.placement).await?;

    let ligne = sqlx::query!(
        r#"INSERT INTO content.highlights
               (placement, status, nature_code, sort_order, title, quote, body,
                person_id, author_name, author_title, organization_id, organization_label,
                country_id, event_id, session_id, link_url, link_label,
                background_color_hex, starts_at, ends_at, published_at, created_by)
           VALUES ($1::text::content.highlight_placement, $2::text::content.highlight_status,
                   $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                   $16::text::platform.url, $17, $18, $19, $20,
                   CASE WHEN $2 = 'published' THEN now() END, $21)
           RETURNING id"#,
        valeurs.placement,
        valeurs.status,
        valeurs.nature_code,
        rang,
        valeurs.title,
        valeurs.quote,
        valeurs.body,
        valeurs.person_id,
        valeurs.author_name,
        valeurs.author_title,
        valeurs.organization_id,
        valeurs.organization_label,
        valeurs.country_id,
        valeurs.event_id,
        valeurs.session_id,
        valeurs.link_url,
        valeurs.link_label,
        valeurs.background_color_hex,
        valeurs.starts_at,
        valeurs.ends_at,
        auteur
    )
    .fetch_one(&mut *tx)
    .await?;

    Ok(ligne.id)
}

/// Met à jour une diapositive. `published_at` **ne se rejoue jamais** : c'est le
/// déclencheur du modèle qui le pose au premier passage en `published`, et le
/// back-office ne peut pas le contredire.
pub async fn modifier(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
    valeurs: &ShowcaseFormValues,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE content.highlights
              SET placement = $2::text::content.highlight_placement,
                  status = $3::text::content.highlight_status,
                  nature_code = $4, title = $5, quote = $6, body = $7,
                  person_id = $8, author_name = $9, author_title = $10,
                  organization_id = $11, organization_label = $12, country_id = $13,
                  event_id = $14, session_id = $15,
                  link_url = $16::text::platform.url, link_label = $17,
                  background_color_hex = $18, starts_at = $19, ends_at = $20,
                  updated_at = now()
            WHERE id = $1"#,
        id,
        valeurs.placement,
        valeurs.status,
        valeurs.nature_code,
        valeurs.title,
        valeurs.quote,
        valeurs.body,
        valeurs.person_id,
        valeurs.author_name,
        valeurs.author_title,
        valeurs.organization_id,
        valeurs.organization_label,
        valeurs.country_id,
        valeurs.event_id,
        valeurs.session_id,
        valeurs.link_url,
        valeurs.link_label,
        valeurs.background_color_hex,
        valeurs.starts_at,
        valeurs.ends_at
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}

/// Les thématiques, réécrites en entier : le rattachement générique de
/// `reference.entity_terms` n'a pas de notion de « delta », et un ajout sans
/// retrait laisserait une thématique décochée en base.
pub async fn poser_les_themes(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
    codes: &[String],
) -> Result<()> {
    sqlx::query!(
        r#"DELETE FROM reference.entity_terms
            WHERE entity_schema = 'content' AND entity_table = 'highlights' AND entity_id = $1
              AND term_id IN (SELECT id FROM reference.taxonomy_terms WHERE taxonomy_code = 'activity_theme')"#,
        id
    )
    .execute(&mut *tx)
    .await?;

    if codes.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        r#"INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id)
           SELECT 'content', 'highlights', $1, t.id
             FROM reference.taxonomy_terms t
            WHERE t.taxonomy_code = 'activity_theme' AND t.is_active AND t.code = ANY($2::text[])
           ON CONFLICT DO NOTHING"#,
        id,
        codes
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}

/// Change le statut. `published_at` se pose au **premier** passage en
/// `published` et ne bouge plus ensuite.
pub async fn changer_le_statut(tx: &mut sqlx::PgConnection, id: Uuid, statut: &str) -> Result<()> {
    sqlx::query!(
        r#"UPDATE content.highlights
              SET status = $2::text::content.highlight_status,
                  published_at = CASE WHEN $2 = 'published' THEN COALESCE(published_at, now())
                                      ELSE published_at END,
                  updated_at = now()
            WHERE id = $1"#,
        id,
        statut
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}

/// Échange le rang avec le voisin, dans le sens demandé. Rend faux quand il n'y
/// a pas de voisin — aux extrémités, l'écran a déjà désactivé le bouton, et un
/// message d'erreur pour une action qu'il n'offrait pas serait du bruit.
pub async fn deplacer(tx: &mut sqlx::PgConnection, id: Uuid, vers_le_haut: bool) -> Result<bool> {
    let voisin = sqlx::query!(
        r#"SELECT v.id, v.sort_order AS "sort_order!", h.sort_order AS "rang_courant!",
                  h.placement::text AS "placement!"
             FROM content.highlights h
             JOIN content.highlights v
               ON v.placement = h.placement AND v.id <> h.id
              AND (($2::bool AND (v.sort_order, v.id) < (h.sort_order, h.id))
                OR (NOT $2::bool AND (v.sort_order, v.id) > (h.sort_order, h.id)))
            WHERE h.id = $1
            ORDER BY CASE WHEN $2 THEN -v.sort_order ELSE v.sort_order END, v.id
            LIMIT 1"#,
        id,
        vers_le_haut
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(voisin) = voisin else {
        return Ok(false);
    };

    sqlx::query!(
        "UPDATE content.highlights SET sort_order = $2, updated_at = now() WHERE id = $1",
        id,
        voisin.sort_order
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE content.highlights SET sort_order = $2, updated_at = now() WHERE id = $1",
        voisin.id,
        voisin.rang_courant
    )
    .execute(&mut *tx)
    .await?;

    renumeroter(&mut *tx, &voisin.placement).await?;
    Ok(true)
}

/// Duplique en **brouillon**, en fin d'emplacement : voir sortir sur l'accueil
/// la copie d'un contenu publié serait une publication que personne n'a
/// demandée. Les thématiques suivent — c'est le geste qui remet un témoignage
/// de la COP30 à la COP31.
pub async fn dupliquer(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
    auteur: Uuid,
) -> Result<Option<Uuid>> {
    let Some(valeurs) = valeurs(&mut *tx, id).await? else {
        return Ok(None);
    };

    let copie = ShowcaseFormValues {
        id: None,
        status: "draft".to_owned(),
        ..valeurs
    };

    let nouvel_id = creer(&mut *tx, &copie, auteur).await?;
    poser_les_themes(&mut *tx, nouvel_id, &copie.theme_codes).await?;
    Ok(Some(nouvel_id))
}

/// Le rang suivant dans un emplacement — le plus grand, plus un pas.
async fn prochain_rang(tx: &mut sqlx::PgConnection, placement: &str) -> Result<i16> {
    let ligne = sqlx::query!(
        r#"SELECT COALESCE(max(sort_order), 0) AS "max!"
             FROM content.highlights
            WHERE placement = $1::text::content.highlight_placement"#,
        placement
    )
    .fetch_one(&mut *tx)
    .await?;

    Ok(ligne.max.saturating_add(PAS as i32) as i16)
}

/// Remet l'emplacement à plat : 10, 20, 30… dans l'ordre courant.
pub async fn renumeroter(tx: &mut sqlx::PgConnection, placement: &str) -> Result<()> {
    sqlx::query!(
        r#"WITH ordonnees AS (
               SELECT id, (row_number() OVER (ORDER BY sort_order, id) * $2)::smallint AS rang
                 FROM content.highlights
                WHERE placement = $1::text::content.highlight_placement
           )
           UPDATE content.highlights h
              SET sort_order = o.rang
             FROM ordonnees o
            WHERE h.id = o.id AND h.sort_order <> o.rang"#,
        placement,
        i64::from(PAS)
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}
