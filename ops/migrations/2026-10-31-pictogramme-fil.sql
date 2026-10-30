-- Pictogramme d'un fil (31/10) : `event.programme_tracks.icon_code`.
-- Rejouable, additive, aucune donnée touchée.
-- Modèle : docs/database/060_events.sql § 4 ter (après venue_id).
--
-- La liste des dessins vit dans le code du site (frontend/app/utils/track-icons.ts) :
-- la base ne contraint que la forme du code.
--
-- Jouer AVANT de démarrer la nouvelle API : ses requêtes nomment la colonne.
-- Les pictogrammes des fils de la CdP31 : ops/contenu/2026-10-31-pictogrammes-cdp31.sql, APRÈS.
BEGIN;

ALTER TABLE event.programme_tracks ADD COLUMN IF NOT EXISTS icon_code text;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ck_programme_tracks_icon_code') THEN
        ALTER TABLE event.programme_tracks ADD CONSTRAINT ck_programme_tracks_icon_code
            CHECK (icon_code ~ '^[a-z][a-z0-9-]{1,40}$');
    END IF;
END;
$$;

COMMENT ON COLUMN event.programme_tracks.icon_code IS
    'Pictogramme du fil, code d''un dessin connu du site (« coins », « sprout »). NULL : aucun. Forme seule contrainte : la liste vit dans le code du site.';

COMMIT;
