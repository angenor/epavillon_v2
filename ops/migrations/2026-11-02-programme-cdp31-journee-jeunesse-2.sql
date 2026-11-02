-- Journée jeunesse de la CdP31, 14/11 (02/11/2026), suite, à la demande du
-- commanditaire : trois activités de 10 min à la suite de 09 h 20 à 09 h 50
-- (Europe/Istanbul) : JVE Centrafrique, Fédération L'Air et Moi, NRCI (Daki
-- Farm). L'atelier IA de l'IFDD (CDP31-00240) repasse « en évaluation » et
-- quitte le site public le temps que l'IFDD lui trouve un créneau.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-programme-cdp31-journee-jeunesse-2', true);

DO $$
DECLARE
    v_track uuid := '01a18dc9-5da9-7b2a-a15a-0b6917f228ec';
    v_ia    uuid := '01a17da1-9bea-7da2-af56-7c4e6a731f1c';
    v_cas   record;
    v_n     int;
BEGIN
    FOR v_cas IN
        SELECT * FROM (VALUES
            ('Jeunes Volontaires pour l''Environnement en Centrafrique', '09:20', '09:30'),
            ('Fédération L''Air et Moi',                                '09:30', '09:40'),
            ('NRCI',                                                    '09:40', '09:50')
        ) AS t(organisation, debut, fin)
    LOOP
        UPDATE programme.sessions s
           SET starts_at = ('2026-11-14 ' || v_cas.debut)::timestamp AT TIME ZONE 'Europe/Istanbul',
               ends_at   = ('2026-11-14 ' || v_cas.fin)::timestamp AT TIME ZONE 'Europe/Istanbul'
          FROM programme.session_tracks st, org.organizations o
         WHERE st.session_id = s.id AND st.track_id = v_track
           AND o.id = s.organization_id AND o.legal_name = v_cas.organisation
           AND s.status = 'scheduled';
        GET DIAGNOSTICS v_n = ROW_COUNT;
        IF v_n <> 1 THEN
            RAISE EXCEPTION 'Garde : une activité de % attendue dans la journée, % touchée', v_cas.organisation, v_n;
        END IF;
    END LOOP;

    UPDATE programme.sessions
       SET status = 'planned', published_at = NULL, is_streamed = false
     WHERE id = v_ia AND status = 'scheduled'
       AND NOT EXISTS (SELECT 1 FROM programme.registrations r
                        WHERE r.session_id = v_ia AND r.status <> 'cancelled');
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : séance IA publiée sans inscrit attendue, % touchée', v_n; END IF;

    UPDATE programme.proposals
       SET status = 'under_review',
           decision_reason = 'En attente d''un créneau trouvé par l''IFDD.',
           decided_at = NULL, decided_by = NULL
     WHERE reference_code = 'CDP31-00240' AND status = 'accepted';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : CDP31-00240 retenu attendu, % touché', v_n; END IF;
END;
$$;
COMMIT;
