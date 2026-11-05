-- Liens vers une page de téléchargement sur la page d'un fil (05/11/2026), à la
-- demande du commanditaire : à côté des PDF déposés, mener vers la page où
-- l'éditeur publie un document (le Guide des négociations sur le site de l'OIF).
-- Additive et rejouable, modèle seul : À JOUER AVANT DE DÉPLOYER L'API, qui lit
-- la table sur la page publique de chaque fil.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

CREATE TABLE IF NOT EXISTS event.programme_track_links (
    id         uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    track_id   uuid        NOT NULL REFERENCES event.programme_tracks(id) ON DELETE CASCADE,
    label      platform.i18n_text NOT NULL,
    url        text        NOT NULL,
    sort_order smallint    NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_programme_track_links_url
        CHECK (url ~ '^(https?://|/)[^\s]*$' AND length(url) <= 2048)
);

CREATE INDEX IF NOT EXISTS ix_programme_track_links_track
    ON event.programme_track_links (track_id, sort_order);

COMMENT ON TABLE event.programme_track_links IS
    'Liens d''un fil vers la page où un document se télécharge chez son éditeur. Remplacés en bloc par l''éditeur de la page ; affichés après les PDF déposés.';

DROP TRIGGER IF EXISTS tg_programme_track_links_public_signal ON event.programme_track_links;
CREATE TRIGGER tg_programme_track_links_public_signal
    AFTER INSERT OR UPDATE OR DELETE ON event.programme_track_links
    FOR EACH ROW EXECUTE FUNCTION event.tg_programme_track_page_signal();

COMMIT;
