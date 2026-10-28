-- Lot 100 (phase 2) : rediffusions de la CdP30 attendues, sans leurs vidéos.
--
-- `lot100_video` (activités v1 dont `youtube_link` est rempli), écrite par
-- 100-rediffusions.sh. La chaîne YouTube a été piratée : les vidéos reviendront
-- après la CdP31, déposées sur Garage ou sur une nouvelle chaîne (commanditaire,
-- 28/10). D'ici là, la page d'une activité passée, enregistrée et sans
-- rediffusion annonce « Vidéo prochainement disponible ».
--
-- Choix (commanditaire, 28/10) :
--   - toutes les séances publiées de la CdP30 ont été filmées, y compris les 28
--     dont la v1 n'avait pas encore la vidéo : `is_recorded` vrai partout (déjà le
--     cas après le lot 50, posé ici pour que la règle soit écrite) ;
--   - les 33 activités v1 qui avaient une vidéo sont consignées `skipped` sous
--     `public.activities-youtube_link`, pour savoir lesquelles avaient déjà la leur.
-- Rien ne part : `is_recorded` n'est surveillé par aucun déclencheur d'événement.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-100', true) \g /dev/null

CREATE TEMP TABLE l100_seance ON COMMIT DROP AS
SELECT s.id, s.is_recorded,
       EXISTS (SELECT 1 FROM legacy.id_map m JOIN lot100_video v ON m.legacy_id = v.activite::text
                WHERE m.legacy_table = 'public.activities' AND m.target_id = s.proposal_id) AS video_v1
FROM programme.sessions s
JOIN event.events e ON e.id = s.event_id
WHERE e.slug = 'cdp30-belem-2025';

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM legacy.id_map WHERE legacy_table = 'public.activities-youtube_link') THEN
        RAISE EXCEPTION 'lot 100 : déjà joué';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM l100_seance) THEN
        RAISE EXCEPTION 'lot 100 : aucune séance de la CdP30';
    END IF;
END $$;

UPDATE programme.sessions s
   SET is_recorded = true
 WHERE s.id IN (SELECT id FROM l100_seance) AND s.published_at IS NOT NULL AND NOT s.is_recorded;

-- Correspondance ----------------------------------------------------------------------

SELECT legacy.remember('public.activities-youtube_link', v.activite::text, 'programme', 'sessions', NULL,
                       'skipped',
                       'vidéo YouTube perdue (chaîne piratée) : à reposer après la CdP31')
FROM lot100_video v \g /dev/null

-- Outbox et travaux : rien ne doit naître de ces lignes.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-100') THEN
        RAISE EXCEPTION 'lot 100 : événement d''outbox inattendu';
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now()) THEN
        RAISE EXCEPTION 'lot 100 : travail créé par la transaction';
    END IF;
END $$;

-- Compte rendu ---------------------------------------------------------------------

DO $$
DECLARE
    extraites bigint := (SELECT count(*) FROM lot100_video);
    consignees bigint := (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'public.activities-youtube_link');
BEGIN
    IF extraites <> consignees THEN
        RAISE EXCEPTION 'lot 100 : % vidéos extraites, % consignées', extraites, consignees;
    END IF;
    RAISE NOTICE 'lot 100 : vidéos v1 %, séances CdP30 publiées % (vidéo v1 %, enregistrées %)',
        extraites,
        (SELECT count(*) FROM programme.sessions s JOIN l100_seance l USING (id) WHERE s.published_at IS NOT NULL),
        (SELECT count(*) FROM programme.sessions s JOIN l100_seance l USING (id) WHERE s.published_at IS NOT NULL AND l.video_v1),
        (SELECT count(*) FROM programme.sessions s JOIN l100_seance l USING (id) WHERE s.published_at IS NOT NULL AND s.is_recorded);
END $$;

COMMIT;
