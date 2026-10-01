-- La programmation publique rend le logo et le type de l'organisation porteuse
-- (logo et drapeau sur la liste). Modèle : docs/database/080_live.sql.
BEGIN;
CREATE OR REPLACE VIEW programme.v_public_schedule AS
SELECT
    s.id,
    s.event_id,
    s.event_day_id,
    s.proposal_id,
    s.slug,
    s.title,
    s.summary,
    s.starts_at,
    s.ends_at,
    s.timezone,
    s.format,
    s.status,
    s.room_id,
    r.name                AS room_name,
    s.organization_id,
    o.legal_name          AS organization_name,
    o.acronym             AS organization_acronym,
    c.iso2                AS organization_country_code,
    c.name                AS organization_country,
    s.is_streamed,
    s.broadcast_channel_id,
    s.capacity,
    COALESCE((
        SELECT jsonb_agg(jsonb_build_object(
                   'slug', t.slug, 'title', t.title, 'color', t.color_hex, 'kind', t.kind)
               ORDER BY st.sort_order, t.sort_order)
        FROM programme.session_tracks st
        JOIN event.programme_tracks t ON t.id = st.track_id
        WHERE st.session_id = s.id AND t.published_at IS NOT NULL
    ), '[]'::jsonb) AS tracks,
    COALESCE(
        media.attached_image('programme', 'sessions',  s.id,          'cover'),
        media.attached_image('programme', 'proposals', s.proposal_id, 'cover')
    ) AS cover,
    CASE
        WHEN s.status = 'cancelled'          THEN 'cancelled'
        WHEN s.status = 'postponed'          THEN 'postponed'
        WHEN now() < s.starts_at             THEN 'upcoming'
        WHEN now() BETWEEN s.starts_at AND s.ends_at THEN 'ongoing'
        ELSE 'past'
    END AS temporal_state,
    (SELECT count(*) FROM programme.registrations rg
      WHERE rg.session_id = s.id AND rg.status IN ('registered', 'attended')) AS registered_count,
    reference.terms_of('programme', 'sessions', s.id, 'activity_theme') AS theme_codes,
    reference.term_badges('programme', 'sessions', s.id, 'activity_theme') AS themes,
    s.waitlist_enabled,
    s.registration_required,
    s.registration_opens_at,
    s.registration_closes_at,
    (SELECT count(*) FROM programme.registrations rg
      WHERE rg.session_id = s.id AND rg.status = 'waitlisted') AS waitlisted_count,
    s.listing_changed_at,
    p.language_codes,
    rp.url              AS replay_url,
    rp.duration_seconds AS replay_duration_seconds,
    media.attached_image('org', 'organizations', o.id, 'logo') AS organization_logo,
    o.organization_type_code
FROM programme.sessions s
LEFT JOIN event.rooms r          ON r.id = s.room_id
LEFT JOIN org.organizations o    ON o.id = s.organization_id
LEFT JOIN reference.countries c  ON c.id = o.country_id
LEFT JOIN programme.proposals p  ON p.id = s.proposal_id
LEFT JOIN LATERAL (
    SELECT COALESCE(ls.replay_url::text, ls.watch_url::text,
                    live.build_embed_url(ls.provider, ls.embed_id)) AS url,
           COALESCE(round(a.duration_seconds)::int,
                    round(oa.duration_seconds)::int,
                    round(extract(epoch FROM ls.ended_at - ls.started_at))::int,
                    round(extract(epoch FROM og.ended_at - og.started_at))::int) AS duration_seconds
      FROM live.streams ls
      LEFT JOIN media.assets a  ON a.id  = ls.recording_asset_id
      LEFT JOIN live.streams og ON og.id = ls.replay_of_id
      LEFT JOIN media.assets oa ON oa.id = og.recording_asset_id
     WHERE ls.session_id = s.id
       AND ls.status <> 'cancelled'
       AND (ls.kind = 'replay' OR ls.replay_url IS NOT NULL)
       AND (ls.replay_available_at IS NULL OR ls.replay_available_at <= now())
       AND COALESCE(ls.replay_url::text, ls.watch_url::text,
                    live.build_embed_url(ls.provider, ls.embed_id)) IS NOT NULL
     ORDER BY ls.is_primary DESC, ls.replay_available_at DESC NULLS LAST, ls.id DESC
     LIMIT 1
) rp ON true
WHERE s.published_at IS NOT NULL;

COMMENT ON COLUMN programme.v_public_schedule.organization_logo IS
    'Logo de l''organisation porteuse, rendu par media.attached_image() comme la couverture ; nul sans organisation ou sans logo servable.';
COMMENT ON COLUMN programme.v_public_schedule.organization_type_code IS
    'Type de l''organisation porteuse (code de la taxonomie organization_type) : public_national_institution fait afficher le drapeau de son pays. Nul sans organisation.';
COMMIT;
