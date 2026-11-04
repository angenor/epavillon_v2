-- « Association Gorichka » (04/11/2026), à la demande du commanditaire : le
-- sigle « n/a », saisi dans la v1 faute de sigle et repris tel quel, s'affichait
-- comme un vrai sigle dans la programmation. Vidé : le site montre les initiales.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-04-gorichka-sans-sigle', true);

DO $$
DECLARE
    v_n int;
BEGIN
    UPDATE org.organizations SET acronym = NULL
     WHERE id = '01a17da1-98f5-7343-b853-91ca765b128d' AND acronym = 'n/a';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : sigle « n/a » attendu, % ligne touchée', v_n; END IF;
END;
$$;
COMMIT;
