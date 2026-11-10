-- Ressources d'une journée (10/11/2026) : un lien de fil peut attendre sa date,
-- un moment (temps fort) peut porter un lien. Idempotent. À jouer avant l'API.
\set ON_ERROR_STOP on
BEGIN;

ALTER TABLE event.programme_track_links ADD COLUMN IF NOT EXISTS visible_from timestamptz;

ALTER TABLE event.programme_track_moments ADD COLUMN IF NOT EXISTS link_label platform.i18n_text;
ALTER TABLE event.programme_track_moments ADD COLUMN IF NOT EXISTS link_url text;
ALTER TABLE event.programme_track_moments DROP CONSTRAINT IF EXISTS ck_programme_track_moments_link;
ALTER TABLE event.programme_track_moments ADD CONSTRAINT ck_programme_track_moments_link
    CHECK ((link_url IS NULL) = (link_label IS NULL));
ALTER TABLE event.programme_track_moments DROP CONSTRAINT IF EXISTS ck_programme_track_moments_link_url;
ALTER TABLE event.programme_track_moments ADD CONSTRAINT ck_programme_track_moments_link_url
    CHECK (link_url IS NULL OR (link_url ~ '^(https?://|/)[^\s]*$' AND length(link_url) <= 2048));

COMMENT ON TABLE event.programme_track_links IS
    'Liens d''un fil vers une ressource hébergée ailleurs (page, PDF, vidéo). Remplacés en bloc par l''éditeur de la page ; publics à partir de visible_from.';

COMMIT;
