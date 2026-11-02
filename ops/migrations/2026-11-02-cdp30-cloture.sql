-- CdP30 (02/11) : la clôture des dépôts reprise de la v1 (21/11/2025) tombait
-- après le début de l'édition. Choix du commanditaire : 15/10/2025 en fin de
-- journée, heure de Belém, fin de la vague réelle des dépôts.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-cdp30-cloture', true);

DO $$
DECLARE v_n int;
BEGIN
    UPDATE event.calls_for_proposals
       SET closes_at = '2025-10-15 23:59:00 America/Belem'
     WHERE code = 'cdp30'
       AND event_id = (SELECT id FROM event.events WHERE slug = 'cdp30-belem-2025')
       AND closes_at > '2025-11-01';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n > 1 THEN
        RAISE EXCEPTION 'Garde : un seul appel CdP30 attendu, % touchés', v_n;
    END IF;
END;
$$;
COMMIT;
