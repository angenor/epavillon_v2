-- Journée jeunesse de la CdP31, 14/11 (03/11/2026), à la demande du
-- commanditaire. Heures Europe/Istanbul.
--   Hackathon (Fondation Green OpenLab) : 10 h 40 à 11 h 40 (finissait à 11 h 30).
--   Jeux concours « Info ou Intox Climat » : 11 h 40 à 12 h 20 (commençait à 11 h 30).
-- Aucun inscrit concerné par un moment : aucun courriel.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-03-journee-jeunesse-horaires', true);

DO $$
DECLARE
    v_track uuid := '01a18dc9-5da9-7b2a-a15a-0b6917f228ec';
    v_n int;
BEGIN
    UPDATE event.programme_track_moments
       SET starts_at = timestamp '2026-11-14 10:40' AT TIME ZONE 'Europe/Istanbul',
           ends_at   = timestamp '2026-11-14 11:40' AT TIME ZONE 'Europe/Istanbul'
     WHERE track_id = v_track AND label->>'fr' LIKE 'Fondation Green OpenLab : Hackathon%';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : hackathon attendu, % touché', v_n; END IF;

    UPDATE event.programme_track_moments
       SET starts_at = timestamp '2026-11-14 11:40' AT TIME ZONE 'Europe/Istanbul',
           ends_at   = timestamp '2026-11-14 12:20' AT TIME ZONE 'Europe/Istanbul'
     WHERE track_id = v_track AND label->>'fr' LIKE 'Jeux concours « Info ou Intox Climat »%';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : jeu concours attendu, % touché', v_n; END IF;
END;
$$;
COMMIT;
