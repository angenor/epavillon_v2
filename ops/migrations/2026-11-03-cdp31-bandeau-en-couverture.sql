-- CdP31 (03/11/2026), à la demande du commanditaire : la photo reprise de la v1
-- (3315 × 1860, 16:9) était rangée en `banner` (32:9 attendu) et bloquait
-- l'enregistrement des images de l'édition. Elle passe en `cover`, sa forme,
-- vide jusque-là. Le site l'affiche aux mêmes endroits (cover avant banner).
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-03-cdp31-bandeau-en-couverture', true);
DO $$
DECLARE v_n int;
BEGIN
    IF EXISTS (SELECT 1 FROM media.attachments
                WHERE owner_schema = 'event' AND owner_table = 'events'
                  AND owner_id = '01a17da1-9b46-75b2-895a-d9e226226be2' AND role = 'cover') THEN
        RAISE EXCEPTION 'Garde : la CdP31 a déjà une couverture';
    END IF;
    UPDATE media.attachments SET role = 'cover'
     WHERE owner_schema = 'event' AND owner_table = 'events'
       AND owner_id = '01a17da1-9b46-75b2-895a-d9e226226be2' AND role = 'banner';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : un bandeau CdP31 attendu, % touché', v_n; END IF;
END;
$$;
COMMIT;
