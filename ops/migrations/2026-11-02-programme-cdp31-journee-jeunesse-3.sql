-- Journée jeunesse de la CdP31, 14/11 (02/11/2026), suite, à la demande du
-- commanditaire. Heures Europe/Istanbul.
--   1. Fiche « Université Côte d'Azur » créée (candidate, France), co-organisatrice
--      de CDP31-00240 avec l'IFDD.
--   2. CDP31-00240 « Intelligence artificielle et environnement… » retenue,
--      publiée, diffusée, 9 h 50 à 10 h 40, partie « S'inspirer ».
--   3. Moment « Les 10 minutes qui inspirent » supprimé (il tombait dans l'atelier).
--   4. Hackathon 10 h 40 à 11 h 30 (temps fort) ; « Info ou Intox Climat »
--      11 h 30 à 12 h 20 ; « Les ClimArtistes en scène » 12 h 20 à 13 h 00 ;
--      libellés repris du commanditaire.
-- Aucun inscrit à la journée ni à l'atelier : aucun courriel.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-programme-cdp31-journee-jeunesse-3', true);

DO $$
DECLARE
    v_acteur   uuid := '01a0627c-f3be-7185-89c8-9a4015ad158e';
    v_track    uuid := '01a18dc9-5da9-7b2a-a15a-0b6917f228ec';
    v_inspirer uuid := '01a18dc9-5da9-7100-94f8-3583ed99a30b';
    v_ia       uuid := '01a17da1-9bea-7da2-af56-7c4e6a731f1c';
    v_canal    uuid := '01a06241-e7f7-7fd8-ae70-7e5b4565f458';
    v_france   uuid := '01a06241-ac6b-74a9-aa27-7e5891650102';
    v_dossier  uuid;
    v_uca      uuid;
    v_n int;
BEGIN
    -- 1. Université Côte d'Azur
    SELECT id INTO v_uca FROM org.organizations WHERE slug = 'universite-cote-d-azur';
    IF v_uca IS NULL THEN
        INSERT INTO org.organizations (legal_name, acronym, slug, organization_type_code, country_id,
                                       city, website, created_by, status)
        VALUES ('Université Côte d''Azur', 'UniCA', 'universite-cote-d-azur', 'academic_research',
                v_france, 'Nice', 'https://univ-cotedazur.fr', v_acteur, 'candidate')
        RETURNING id INTO v_uca;
    END IF;

    SELECT id INTO v_dossier FROM programme.proposals WHERE reference_code = 'CDP31-00240';

    INSERT INTO programme.proposal_organizations (proposal_id, organization_id, role, sort_order, added_by)
    VALUES (v_dossier, v_uca, 'co_organizer', 1, v_acteur)
    ON CONFLICT (proposal_id, organization_id) DO UPDATE SET role = 'co_organizer', sort_order = 1;

    INSERT INTO programme.session_organizations (session_id, organization_id, role, sort_order)
    VALUES (v_ia, v_uca, 'co_organizer', 1)
    ON CONFLICT (session_id, organization_id) DO UPDATE SET role = 'co_organizer', sort_order = 1;

    -- 2. L'atelier IA retenu et publié
    UPDATE programme.proposals SET status = 'accepted', decision_reason = NULL
     WHERE id = v_dossier AND status = 'under_review';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : CDP31-00240 en évaluation attendu, % touché', v_n; END IF;

    UPDATE programme.sessions
       SET starts_at = timestamp '2026-11-14 09:50' AT TIME ZONE 'Europe/Istanbul',
           ends_at   = timestamp '2026-11-14 10:40' AT TIME ZONE 'Europe/Istanbul',
           status = 'scheduled', published_at = now(),
           is_streamed = true, broadcast_channel_id = v_canal
     WHERE id = v_ia AND status = 'planned' AND room_id IS NOT NULL;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : séance IA planifiée avec salle attendue, % touchée', v_n; END IF;

    UPDATE programme.session_tracks SET part_id = v_inspirer, is_highlight = false
     WHERE session_id = v_ia AND track_id = v_track;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : séance IA dans la journée jeunesse attendue, % touchée', v_n; END IF;

    -- 3. « Les 10 minutes qui inspirent »
    DELETE FROM event.programme_track_moments
     WHERE track_id = v_track AND label->>'fr' LIKE '« Les 10 minutes qui inspirent »%';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : un moment « 10 minutes qui inspirent » attendu, % touché', v_n; END IF;

    -- 4. Les trois moments de la fin de matinée
    UPDATE event.programme_track_moments
       SET starts_at = timestamp '2026-11-14 10:40' AT TIME ZONE 'Europe/Istanbul',
           ends_at   = timestamp '2026-11-14 11:30' AT TIME ZONE 'Europe/Istanbul',
           is_highlight = true
     WHERE track_id = v_track AND label->>'fr' LIKE 'Fondation Green OpenLab : Hackathon%';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : hackathon attendu, % touché', v_n; END IF;

    UPDATE event.programme_track_moments
       SET starts_at = timestamp '2026-11-14 11:30' AT TIME ZONE 'Europe/Istanbul',
           ends_at   = timestamp '2026-11-14 12:20' AT TIME ZONE 'Europe/Istanbul',
           label = '{"fr": "Jeux concours « Info ou Intox Climat » : Qui sera le Champion ou la Championne CdP31 / OIF ?"}'
     WHERE track_id = v_track AND label->>'fr' LIKE 'Jeu concours « Info ou Intox Climat »%';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : jeu concours attendu, % touché', v_n; END IF;

    UPDATE event.programme_track_moments
       SET starts_at = timestamp '2026-11-14 12:20' AT TIME ZONE 'Europe/Istanbul',
           ends_at   = timestamp '2026-11-14 13:00' AT TIME ZONE 'Europe/Istanbul',
           label = '{"fr": "Les ClimArtistes en scène (expression artistique sur l’action climatique)"}',
           description = NULL
     WHERE track_id = v_track AND label->>'fr' = 'Les ClimArtistes en scène';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : ClimArtistes attendu, % touché', v_n; END IF;
END;
$$;
COMMIT;
