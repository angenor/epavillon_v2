-- Sigle bien connu (02/11). Rejouable, additive.
-- Modèle : docs/database/040_organizations.sql (org.organizations.acronym_well_known)
-- et 080_live.sql (programme.v_public_schedule : clé `acronym_well_known` des
-- organisations de `proposals`, colonne `organization_acronym_well_known` en queue).
--
-- Certaines organisations se reconnaissent à leur sigle (OIF, FAO), d'autres
-- non (RF-EFH). L'équipe coche la case au back-office ; là où la place manque,
-- le programme public n'affiche alors que le sigle, sinon le nom complet.
-- Neuf sigles cochés d'office, à la demande du commanditaire.
--
-- La vue remplacée était identique en production à celle du dépôt avant ce
-- changement (lu le 02/11) ; aucune vue n'en dépend.
--
-- Jouer AVANT de déployer l'API : elle lit la nouvelle colonne.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-sigle-bien-connu', true);

ALTER TABLE org.organizations
    ADD COLUMN IF NOT EXISTS acronym_well_known boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN org.organizations.acronym_well_known IS
    'Sigle bien connu du public : coché, le programme public affiche le sigle seul là où la place manque ; sinon le nom complet. Posé par l''équipe, sans effet sans sigle.';

UPDATE org.organizations
   SET acronym_well_known = true
 WHERE acronym IN ('OIF', 'AFD', 'ADEME', 'FAO', 'OCDE', 'BAD', 'CILSS', 'AUC', 'GGGI')
   AND NOT acronym_well_known;

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
    COALESCE(o.acronym_well_known, false) AS organization_acronym_well_known
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

COMMENT ON COLUMN programme.v_public_schedule.proposals IS
    'Séance commune : dossiers réunis (titre d''origine, couverture, organisation porteuse avec logo, pays et sigle bien connu), dans l''ordre d''affichage ; [] pour une séance ordinaire.';
COMMENT ON COLUMN programme.v_public_schedule.organization_acronym_well_known IS
    'Le sigle de l''organisation porteuse est bien connu du public : là où la place manque, le site n''affiche que lui. Faux sans organisation.';

COMMIT;
