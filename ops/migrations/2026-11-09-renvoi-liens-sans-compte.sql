-- Donne leur lien personnel aux personnes déjà inscrites SANS COMPTE à une
-- activité seule encore à venir ou en cours : elles n'avaient aucun moyen
-- d'entrer. ENVOIE DE VRAIS COURRIELS : à jouer seulement avec l'accord du
-- commanditaire, APRÈS la mise en ligne (le worker doit lire le jeton).
-- Rejouer ne renvoie rien à qui a déjà un lien.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

CREATE TEMP TABLE liens ON COMMIT DROP AS
SELECT r.id AS registration_id,
       replace(gen_random_uuid()::text, '-', '') || replace(gen_random_uuid()::text, '-', '') AS jeton
  FROM programme.registrations r
  JOIN programme.sessions s ON s.id = r.session_id
 WHERE r.status IN ('registered', 'waitlisted')
   AND r.track_registration_id IS NULL
   AND r.manage_token_hash IS NULL
   AND programme.day_track_of(s.id) IS NULL
   AND s.is_streamed
   AND s.status NOT IN ('cancelled', 'postponed', 'completed')
   AND s.ends_at > now()
   AND NOT EXISTS (SELECT 1 FROM identity.accounts a WHERE a.person_id = r.person_id);

UPDATE programme.registrations r
   SET manage_token_hash = sha256(convert_to(l.jeton, 'UTF8'))
  FROM liens l
 WHERE r.id = l.registration_id;

INSERT INTO platform.jobs (queue, task, payload, idempotency_key)
SELECT 'email', 'engagement.send_registration',
       jsonb_build_object('registration_id', l.registration_id, 'token', l.jeton, 'resent', true),
       l.registration_id || ':lien-sans-compte-2026-11-09'
  FROM liens l;

DO $$
DECLARE v_n int;
BEGIN
    SELECT count(*) INTO v_n FROM liens;
    RAISE NOTICE '% lien(s) personnel(s) mis en file', v_n;
END;
$$;

COMMIT;
