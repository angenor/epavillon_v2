-- Atelier préparatoire de la CdP31 (05/11/2026), à la demande du commanditaire :
-- Guide Négo n'est pas encore prêt, l'encart du Guide des négociations n'en parle
-- plus (texte) et perd son lien « Ouvrir dans Guide Négo ». Le Guide et son Résumé
-- se téléchargent depuis les documents du fil, déposés au back-office.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-05-atelier-guide-sans-guide-nego', true);

DO $$
DECLARE
    v_n int;
BEGIN
    UPDATE event.programme_tracks
       SET feature_body = '{"fr":"Le Guide et son Résumé sont présentés l''après-midi par leurs auteurs."}'::jsonb,
           feature_link_label = NULL,
           feature_link_url = NULL
     WHERE id = '01a18dc9-5da9-7f16-876a-663a2e08683c' AND feature_link_url = '/guide-nego';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : encart de l''atelier avec lien Guide Négo attendu, % touché', v_n; END IF;
END;
$$;
COMMIT;
