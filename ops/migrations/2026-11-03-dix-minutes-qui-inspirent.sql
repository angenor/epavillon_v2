-- Journée jeunesse de la CdP31, 14/11 (03/11/2026), à la demande du
-- commanditaire : les trois activités de 10 minutes (JVE Centrafrique 09:20,
-- Fédération L'Air et Moi 09:30, Daki Farm NRCI 09:40) sont réunies sous le
-- moment « Les 10 minutes qui inspirent » (09:20 à 09:50, partie « S'inspirer »).
-- Elles restent publiées ; la page de la journée les énumère sous le moment.
-- Après 2026-11-03-moment-regroupe.sql. Rejouable. Aucun courriel.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-03-dix-minutes-qui-inspirent', true);

DO $$
DECLARE
    v_track    uuid := '01a18dc9-5da9-7b2a-a15a-0b6917f228ec';
    v_inspirer uuid := '01a18dc9-5da9-7100-94f8-3583ed99a30b';
    v_moment   uuid;
    v_n int;
BEGIN
    SELECT id INTO v_moment FROM event.programme_track_moments
     WHERE track_id = v_track AND label->>'fr' = 'Les 10 minutes qui inspirent';
    IF v_moment IS NULL THEN
        INSERT INTO event.programme_track_moments (track_id, part_id, starts_at, ends_at, label)
        VALUES (v_track, v_inspirer,
                timestamp '2026-11-14 09:20' AT TIME ZONE 'Europe/Istanbul',
                timestamp '2026-11-14 09:50' AT TIME ZONE 'Europe/Istanbul',
                '{"fr": "Les 10 minutes qui inspirent", "en": "Ten minutes that inspire"}')
        RETURNING id INTO v_moment;
    END IF;

    UPDATE programme.session_tracks st
       SET moment_id = v_moment, part_id = v_inspirer
      FROM programme.sessions s JOIN programme.proposals p ON p.id = s.proposal_id
     WHERE st.session_id = s.id AND st.track_id = v_track
       AND p.reference_code IN ('CDP31-00177', 'CDP31-00216', 'CDP31-00148')
       AND s.published_at IS NOT NULL;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 3 THEN RAISE EXCEPTION 'Garde : trois activités publiées attendues, % touchée(s)', v_n; END IF;
END;
$$;
COMMIT;
