//! **Ce que le public lit** — aucune session, aucun périmètre.
//!
//! Le critère de diffusion n'est écrit nulle part ici : il vit dans
//! `content.v_showcase`, donc dans le modèle. La vue ne rend qu'une diapositive
//! **publiée** et **dans sa fenêtre** — `starts_at` passé ou nul, `ends_at` à
//! venir ou nul. Rejouer ce filtre côté code, c'est la v1, où chaque composant
//! comparait les dates à sa façon et où une annonce périmée survivait à
//! l'endroit qui avait oublié la comparaison.
//!
//! **L'ordre est celui du défilement**, `sort_order` puis `id` : c'est l'index
//! `ix_highlights_public` du modèle, et c'est ce que le back-office manipule
//! avec ses deux boutons. Le voir autrement sur l'accueil rendrait ces boutons
//! incompréhensibles.

use kernel::error::Result;
use sqlx::PgExecutor;

use crate::domain::showcase::ShowcaseRow;

/// Les diapositives du bandeau d'ouverture, dans l'ordre de défilement.
pub async fn bandeau<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<ShowcaseRow>> {
    let lignes = sqlx::query!(
        r#"SELECT v.id AS "id!", v.placement::text AS "placement!", v.sort_order AS "sort_order!",
                  v.nature_code AS "nature_code!", v.nature_label, v.nature_color, v.nature_icon,
                  v.title AS "title!", v.quote, v.body,
                  v.author_name, v.author_title, v.person_id,
                  v.organization_id, v.organization_name, v.organization_acronym,
                  v.country_code, v.country_name,
                  v.event_id, v.event_slug::text AS "event_slug?", v.event_title,
                  v.session_id, v.session_slug::text AS "session_slug?", v.session_title,
                  v.session_starts_at, v.session_ends_at,
                  v.session_timezone::text AS "session_timezone?",
                  v.link_url, v.link_label,
                  v.background_image, v.background_video, v.thumbnail, v.background_color_hex,
                  v.theme_codes AS "theme_codes!", v.themes AS "themes!",
                  v.starts_at, v.ends_at, v.published_at
             FROM content.v_showcase v
            WHERE v.placement = 'home_hero'
            ORDER BY v.sort_order, v.id"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ShowcaseRow {
            id: l.id,
            placement: l.placement,
            sort_order: l.sort_order,
            nature_code: l.nature_code,
            nature_label: l.nature_label,
            nature_color: l.nature_color,
            nature_icon: l.nature_icon,
            title: l.title,
            quote: l.quote,
            body: l.body,
            author_name: l.author_name,
            author_title: l.author_title,
            person_id: l.person_id,
            organization_id: l.organization_id,
            organization_name: l.organization_name,
            organization_acronym: l.organization_acronym,
            country_code: l.country_code,
            country_name: l.country_name,
            event_id: l.event_id,
            event_slug: l.event_slug,
            event_title: l.event_title,
            session_id: l.session_id,
            session_slug: l.session_slug,
            session_title: l.session_title,
            session_starts_at: l.session_starts_at,
            session_ends_at: l.session_ends_at,
            session_timezone: l.session_timezone,
            link_url: l.link_url,
            link_label: l.link_label,
            background_image: l.background_image,
            background_video: l.background_video,
            thumbnail: l.thumbnail,
            background_color_hex: l.background_color_hex,
            theme_codes: l.theme_codes,
            themes: l.themes,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            published_at: l.published_at,
        })
        .collect())
}

/// L'APERÇU DU FORMULAIRE — la même forme que la vitrine publique.
///
/// **Il ne passe pas par `content.v_showcase`**, et c'est le point : la vue ne
/// rend que ce qui est publié et dans sa fenêtre, alors que l'éditeur compose
/// un brouillon. L'aperçu résout donc les mêmes jointures sur les VALEURS EN
/// COURS DE SAISIE — nature, auteur, organisation, pays, édition, séance — et va
/// chercher les médias de la diapositive quand elle existe déjà.
///
/// C'est ce qui permet à l'aperçu d'être rendu par le composant qui sert
/// l'accueil, et non par une seconde mise en page qui divergerait au premier
/// ajustement de charte.
pub async fn apercu<'e>(
    executor: impl PgExecutor<'e>,
    id: Option<uuid::Uuid>,
    valeurs: &crate::domain::admin::ShowcaseFormValues,
) -> Result<ShowcaseRow> {
    let ligne = sqlx::query!(
        r#"SELECT n.label AS nature_label, n.color_hex AS nature_color, n.icon AS nature_icon,
                  COALESCE(p.display_name, $2) AS author_name,
                  COALESCE(o.legal_name, $3) AS organization_name,
                  o.acronym AS organization_acronym,
                  c.iso2 AS country_code, c.name AS country_name,
                  e.slug::text AS "event_slug?", e.title AS event_title,
                  s.slug::text AS "session_slug?", s.title AS session_title,
                  s.starts_at AS session_starts_at, s.ends_at AS session_ends_at,
                  s.timezone::text AS "session_timezone?",
                  CASE WHEN $8::uuid IS NULL THEN NULL
                       ELSE media.attached_image('content', 'highlights', $8, 'banner') END AS background_image,
                  CASE WHEN $8::uuid IS NULL THEN NULL
                       ELSE media.attached_image('content', 'highlights', $8, 'video') END AS background_video,
                  CASE WHEN $8::uuid IS NULL THEN NULL
                       ELSE media.attached_image('content', 'highlights', $8, 'cover') END AS thumbnail,
                  COALESCE((SELECT jsonb_agg(jsonb_build_object(
                                'code', t.code, 'label', t.label,
                                'color', t.color_hex, 'icon', t.icon)
                                ORDER BY t.sort_order, t.code)
                              FROM reference.taxonomy_terms t
                             WHERE t.taxonomy_code = 'activity_theme'
                               AND t.code = ANY($9::text[])), '[]'::jsonb) AS "themes!"
             FROM (SELECT 1) AS unite
             LEFT JOIN reference.taxonomy_terms n
                    ON n.taxonomy_code = 'highlight_nature' AND n.code = $1 AND n.is_active
             LEFT JOIN identity.people p     ON p.id = $4
             LEFT JOIN org.organizations o   ON o.id = $5
             LEFT JOIN reference.countries c ON c.id = COALESCE($6::uuid, o.country_id)
             LEFT JOIN event.events e        ON e.id = $7
             LEFT JOIN programme.sessions s  ON s.id = $10"#,
        valeurs.nature_code,
        valeurs.author_name,
        valeurs.organization_label,
        valeurs.person_id,
        valeurs.organization_id,
        valeurs.country_id,
        valeurs.event_id,
        id,
        &valeurs.theme_codes,
        valeurs.session_id
    )
    .fetch_one(executor)
    .await?;

    Ok(ShowcaseRow {
        id: id.unwrap_or_else(uuid::Uuid::nil),
        placement: valeurs.placement.clone(),
        sort_order: valeurs.sort_order,
        nature_code: valeurs.nature_code.clone(),
        nature_label: ligne.nature_label,
        nature_color: ligne.nature_color,
        nature_icon: ligne.nature_icon,
        title: valeurs.title.clone(),
        quote: valeurs.quote.clone(),
        body: valeurs.body.clone(),
        author_name: ligne.author_name,
        author_title: valeurs.author_title.clone(),
        person_id: valeurs.person_id,
        organization_id: valeurs.organization_id,
        organization_name: ligne.organization_name,
        organization_acronym: ligne.organization_acronym,
        country_code: ligne.country_code,
        country_name: ligne.country_name,
        event_id: valeurs.event_id,
        event_slug: ligne.event_slug,
        event_title: ligne.event_title,
        session_id: valeurs.session_id,
        session_slug: ligne.session_slug,
        session_title: ligne.session_title,
        session_starts_at: ligne.session_starts_at,
        session_ends_at: ligne.session_ends_at,
        session_timezone: ligne.session_timezone,
        link_url: valeurs.link_url.clone(),
        link_label: valeurs.link_label.clone(),
        background_image: ligne.background_image,
        background_video: ligne.background_video,
        thumbnail: ligne.thumbnail,
        background_color_hex: valeurs.background_color_hex.clone(),
        theme_codes: valeurs.theme_codes.clone(),
        themes: ligne.themes,
        starts_at: valeurs.starts_at,
        ends_at: valeurs.ends_at,
        published_at: None,
    })
}
