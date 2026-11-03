-- Un moment peut regrouper des activités de sa journée (03/11/2026).
-- Rejouable, additive, modèle seul (aucune donnée).
-- Modèle : docs/database/060_events.sql (event.programme_track_moments,
-- contrainte ux_programme_track_moments_track) et 075_programme_sessions.sql
-- (programme.session_tracks.moment_id, xmod_fk_session_tracks_moment).
--
-- « Les 10 minutes qui inspirent » de la journée jeunesse réunit trois
-- activités de dix minutes : elles restent publiées, la page de la journée les
-- énumère sous le moment. Le rattachement se pose ensuite au back-office
-- (composition du fil), pas ici.
--
-- Jouer AVANT de déployer l'API : elle lit la nouvelle colonne.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-03-moment-regroupe', true);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conname = 'ux_programme_track_moments_track'
                      AND conrelid = 'event.programme_track_moments'::regclass) THEN
        ALTER TABLE event.programme_track_moments
            ADD CONSTRAINT ux_programme_track_moments_track UNIQUE (id, track_id);
    END IF;
END;
$$;

ALTER TABLE programme.session_tracks
    ADD COLUMN IF NOT EXISTS moment_id uuid;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conname = 'xmod_fk_session_tracks_moment'
                      AND conrelid = 'programme.session_tracks'::regclass) THEN
        ALTER TABLE programme.session_tracks
            ADD CONSTRAINT xmod_fk_session_tracks_moment FOREIGN KEY (moment_id, track_id)
                REFERENCES event.programme_track_moments (id, track_id) ON DELETE SET NULL (moment_id);
    END IF;
END;
$$;

CREATE INDEX IF NOT EXISTS ix_session_tracks_moment
    ON programme.session_tracks (moment_id) WHERE moment_id IS NOT NULL;

COMMIT;
