-- Organisation hôte du pavillon (03/11). Rejouable, additive.
-- Modèle : docs/database/040_organizations.sql (org.organizations.hosts_pavilion)
-- et 080_live.sql (programme.v_public_schedule : colonne
-- `organization_hosts_pavilion` en queue).
--
-- L'IFDD tient le stand : le programme public signale ses activités par le
-- cercle de l'OIF. Une case de la fiche (« Organisation hôte du pavillon »),
-- posée par l'équipe au back-office, plutôt qu'un nom reconnu dans le code.
-- La fiche de l'IFDD est cochée d'office ; le script s'arrête s'il n'en trouve
-- pas exactement une (sigle IFDD, fiche non fusionnée).
--
-- La vue remplacée est identique à celle posée par 2026-11-02-sigle-bien-connu.sql
-- (comparée au dépôt le 03/11) ; aucune vue n'en dépend.
--
-- Jouer AVANT de déployer l'API : elle lit la nouvelle colonne.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-03-organisation-hote', true);

ALTER TABLE org.organizations
    ADD COLUMN IF NOT EXISTS hosts_pavilion boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN org.organizations.hosts_pavilion IS
    'Organisation hôte du pavillon (l''IFDD) : le programme public signale ses activités par le cercle de l''OIF. Posée par l''équipe au back-office, fausse par défaut.';

DO $$
DECLARE
    trouvees integer;
BEGIN
    SELECT count(*) INTO trouvees
      FROM org.organizations
     WHERE acronym_normalized = platform.normalize_label('IFDD')
       AND merged_into_id IS NULL;
    IF trouvees <> 1 THEN
        RAISE EXCEPTION 'Fiche IFDD : % trouvée(s), une attendue. Cocher la case au back-office.', trouvees;
    END IF;
    UPDATE org.organizations
       SET hosts_pavilion = true
     WHERE acronym_normalized = platform.normalize_label('IFDD')
       AND merged_into_id IS NULL
       AND NOT hosts_pavilion;
END $$;

-- Copie de docs/database/080_live.sql.
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
                   'slug', t.slug, 'title', t.title, 'color', t.color_hex, 'kind', t.kind,
                   'is_highlight', st.is_highlight)
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
    COALESCE(p.language_codes, (
        SELECT array_agg(DISTINCT l ORDER BY l)
        FROM programme.session_proposals sp
        JOIN programme.proposals pp ON pp.id = sp.proposal_id
        CROSS JOIN unnest(pp.language_codes) AS l
        WHERE sp.session_id = s.id
    )) AS language_codes,
    rp.url              AS replay_url,
    rp.duration_seconds AS replay_duration_seconds,
    media.attached_image('org', 'organizations', o.id, 'logo') AS organization_logo,
    o.organization_type_code,
    -- Séance commune (01/11) : les dossiers réunis dans l'ordre d'affichage,
    -- chacun avec son titre d'origine, sa couverture et son organisation
    -- porteuse ; [] pour une séance ordinaire. Les colonnes organization_*
    -- portent alors le premier dossier (organization_id de la séance).
    COALESCE((
        SELECT jsonb_agg(jsonb_build_object(
                   'proposal_id', sp.proposal_id,
                   'sort_order', sp.sort_order,
                   'title', sp.title,
                   'cover', media.attached_image('programme', 'proposals', sp.proposal_id, 'cover'),
                   'organization', jsonb_build_object(
                       'id', po.id,
                       'name', po.legal_name,
                       'acronym', po.acronym,
                       'acronym_well_known', po.acronym_well_known,
                       'country_code', pc.iso2,
                       'country', pc.name,
                       'type_code', po.organization_type_code,
                       'logo', media.attached_image('org', 'organizations', po.id, 'logo')))
               ORDER BY sp.sort_order, sp.added_at)
        FROM programme.session_proposals sp
        JOIN programme.proposals pp   ON pp.id = sp.proposal_id
        JOIN org.organizations po     ON po.id = pp.organization_id
        LEFT JOIN reference.countries pc ON pc.id = po.country_id
        WHERE sp.session_id = s.id
    ), '[]'::jsonb) AS proposals,
    COALESCE(o.acronym_well_known, false) AS organization_acronym_well_known,
    COALESCE(o.hosts_pavilion, false)     AS organization_hosts_pavilion
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

COMMENT ON COLUMN programme.v_public_schedule.organization_hosts_pavilion IS
    'L''organisation porteuse est l''hôte du pavillon (l''IFDD) : le site signale l''activité par le cercle de l''OIF. Faux sans organisation.';

COMMIT;
