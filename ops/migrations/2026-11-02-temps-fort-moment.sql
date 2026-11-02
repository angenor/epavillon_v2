-- Temps fort d'une journée porté par un moment (02/11/2026). Rejouable.
-- Modèle : docs/database/060_events.sql (event.programme_track_moments.is_highlight).
-- Données, accord du commanditaire : journée jeunesse de la CdP31 (14/11),
-- moment « Fondation Green OpenLab : Hackathon… » de 10 h 20 à 11 h 20
-- (Europe/Istanbul) dans la partie « Créer et jouer », temps fort de la
-- journée ; l'atelier IA de l'IFDD, retiré du calendrier, n'est plus temps fort.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-temps-fort-moment', true);

ALTER TABLE event.programme_track_moments
    ADD COLUMN IF NOT EXISTS is_highlight boolean NOT NULL DEFAULT false;

DO $$
DECLARE
    v_track uuid := '01a18dc9-5da9-7b2a-a15a-0b6917f228ec';
    v_part  uuid := '01a18dc9-5da9-74a1-a3bd-609bf24531ac';
    v_label text := 'Fondation Green OpenLab : Hackathon francophone sur l’IA appliquée au climat, à la biodiversité et à la finance durable';
    v_n int;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM event.programme_track_parts
                    WHERE id = v_part AND track_id = v_track AND label->>'fr' = 'Créer et jouer') THEN
        RAISE EXCEPTION 'Garde : partie « Créer et jouer » de la journée jeunesse introuvable';
    END IF;

    INSERT INTO event.programme_track_moments (track_id, part_id, starts_at, ends_at, label, is_highlight)
    SELECT v_track, v_part,
           timestamp '2026-11-14 10:20' AT TIME ZONE 'Europe/Istanbul',
           timestamp '2026-11-14 11:20' AT TIME ZONE 'Europe/Istanbul',
           jsonb_build_object('fr', v_label), true
     WHERE NOT EXISTS (SELECT 1 FROM event.programme_track_moments
                        WHERE track_id = v_track AND label->>'fr' = v_label);

    UPDATE event.programme_track_moments SET is_highlight = true
     WHERE track_id = v_track AND label->>'fr' = v_label AND NOT is_highlight;

    UPDATE programme.session_tracks SET is_highlight = false
     WHERE track_id = v_track AND is_highlight;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n > 1 THEN
        RAISE EXCEPTION 'Garde : un seul temps fort d''activité attendu, % touchés', v_n;
    END IF;
END;
$$;
COMMIT;
