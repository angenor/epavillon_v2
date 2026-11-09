-- Aperçu des deux courriels des intervenants en ligne (08/11/2026), pour
-- validation par le commanditaire : à angenor99@gmail.com seulement, sans
-- copie, objet préfixé « [Aperçu] ». Composés par le vrai code, sur une vraie
-- activité (celle de la RDC, le 13/11) ; le lien Zoom est un lien d'exemple,
-- le lien d'inscription est le vrai lien de l'activité.
-- À jouer APRÈS la mise en ligne : le worker doit connaître ces deux travaux.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

DO $$
DECLARE
    v_seance uuid;
    v_n int;
BEGIN
    SELECT count(*), min(id::text)::uuid INTO v_n, v_seance FROM programme.sessions
     WHERE event_id = '01a17da1-9b46-75b2-895a-d9e226226be2'
       AND title ->> 'fr' ILIKE 'Autorité Nationale de Budgétisation Climat%'
       AND status <> 'cancelled';
    IF v_n <> 1 THEN
        RAISE EXCEPTION 'Garde : % activité(s) trouvée(s) au lieu d''une', v_n;
    END IF;

    INSERT INTO platform.jobs (queue, task, payload, idempotency_key) VALUES
        ('email', 'engagement.send_speaker_zoom_link',
         jsonb_build_object('participant_id', NULL, 'session_id', v_seance,
                            'preview_to', 'angenor99@gmail.com'),
         'apercu:' || clock_timestamp()),
        ('email', 'engagement.send_speaker_signup_link',
         jsonb_build_object('session_id', v_seance, 'preview_to', 'angenor99@gmail.com'),
         'apercu:' || clock_timestamp());
    RAISE NOTICE 'Deux aperçus en file pour angenor99@gmail.com (activité %)', v_seance;
END;
$$;

COMMIT;
