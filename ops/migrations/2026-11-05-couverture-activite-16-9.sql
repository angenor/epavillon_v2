-- Couverture d'une activité en 16:9 (05/11/2026), à la demande du commanditaire :
-- tous les écrans l'affichent en 16:9 ; le recadrage libre laissait déposer une
-- image verticale, rognée ensuite sans que le déposant le voie. La fenêtre de
-- recadrage verrouille désormais la forme. Les couvertures déjà rattachées ne
-- sont pas revérifiées (le contrôle porte sur le rattachement).
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-05-couverture-activite-16-9', true);

DO $$
DECLARE
    v_n int;
BEGIN
    UPDATE media.attachable_roles
       SET expected_aspect_ratio = 1.7778
     WHERE owner_schema = 'programme' AND owner_table = 'proposals' AND role = 'cover'
       AND expected_aspect_ratio IS NULL;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : une règle de couverture libre attendue, % touchée', v_n; END IF;
END;
$$;
COMMIT;
