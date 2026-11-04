-- Programme de la CdP31 (04/11/2026), à la demande du commanditaire : « Table
-- ronde : Justice climatique, justice linguistique… » (CDP31-00204), non retenue
-- le 02/11 faute de créneau, est retenue sur le créneau libéré par CDP31-00194
-- (2026-11-04-programme-cdp31-avenir-de-l-eau-non-retenu) : vendredi 20/11 de
-- 13 h à 14 h, heure d'Antalya, au Stand. Sa séance (planifiée, cachée, au 19/11)
-- prend ce créneau et les réglages des autres activités du Stand : programmée,
-- publiée, avec direct. Le dossier passe par « en évaluation » (aucune
-- transition directe rejected -> accepted), puis « retenu ». Aucun inscrit.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-04-programme-cdp31-justice-linguistique-retenue', true);

DO $$
DECLARE
    v_seance uuid := '01a17da1-9bea-74ea-a3a8-7e0a508c7e3b';
    v_stand uuid := '01a17da1-9b67-7e75-b5c0-d68db6d70feb';
    v_debut timestamptz := timestamp '2026-11-20 13:00' AT TIME ZONE 'Europe/Istanbul';
    v_fin timestamptz := timestamp '2026-11-20 14:00' AT TIME ZONE 'Europe/Istanbul';
    v_n int;
BEGIN
    SELECT count(*) INTO v_n
      FROM programme.sessions s
     WHERE s.event_id = '01a17da1-9b46-75b2-895a-d9e226226be2' AND s.id <> v_seance
       AND s.room_id = v_stand AND s.status::text <> 'cancelled'
       AND s.starts_at < v_fin AND s.ends_at > v_debut;
    IF v_n <> 0 THEN RAISE EXCEPTION 'Garde : créneau du Stand déjà pris (% séance)', v_n; END IF;

    UPDATE programme.proposals
       SET status = 'under_review',
           decision_reason = 'Repêchée par l''IFDD sur le créneau libéré du 20/11.'
     WHERE reference_code = 'CDP31-00204' AND status = 'rejected';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : CDP31-00204 non retenu attendu, % touché', v_n; END IF;

    UPDATE programme.proposals
       SET status = 'accepted', decision_reason = NULL
     WHERE reference_code = 'CDP31-00204' AND status = 'under_review';

    UPDATE programme.sessions
       SET starts_at = v_debut, ends_at = v_fin, room_id = v_stand,
           status = 'scheduled', is_streamed = true, published_at = now()
     WHERE id = v_seance AND status = 'planned' AND published_at IS NULL;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : séance planifiée et cachée attendue, % touchée', v_n; END IF;
END;
$$;
COMMIT;
