-- Programme de la CdP31 (01/11, demande du commanditaire) : toutes les
-- activités ont lieu au Stand ; la Fresque du Climat et COP Bike Ride ne se
-- chevauchent plus le 09/11. Comme PUT /sessions/{id}/schedule : salle, créneau,
-- journée remise à nul pour que la base la redéduise. Aucun inscrit, rien de publié.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-01-programme-cdp31-stand', true);

DO $$
DECLARE
    v_event  uuid := '01a17da1-9b46-75b2-895a-d9e226226be2';
    v_stand  uuid := '01a17da1-9b67-7e75-b5c0-d68db6d70feb';
    v_fresque uuid := '01a17da1-9bea-73e0-b3bf-ce02b544070f';
    v_velo   uuid := '01a17da1-9bea-7f38-92f4-e81b507f6d8e';
    v_n int;
BEGIN
    SELECT count(*) INTO v_n FROM programme.sessions
     WHERE id IN (v_fresque, v_velo) AND event_id = v_event
       AND starts_at = '2026-11-09 09:00+03' AND ends_at = '2026-11-09 11:15+03'
       AND published_at IS NULL;
    IF v_n <> 2 THEN RAISE EXCEPTION 'Garde : Fresque et vélo pas au créneau attendu (%)', v_n; END IF;
    IF EXISTS (SELECT 1 FROM programme.registrations r JOIN programme.sessions s ON s.id = r.session_id
                WHERE s.event_id = v_event AND r.status <> 'cancelled') THEN
        RAISE EXCEPTION 'Garde : des inscrits existent, rien n''est fait';
    END IF;

    UPDATE programme.sessions SET room_id = v_stand
     WHERE event_id = v_event AND room_id IS NULL;

    UPDATE programme.sessions
       SET room_id = v_stand, starts_at = '2026-11-09 09:00+03', ends_at = '2026-11-09 10:00+03', event_day_id = NULL
     WHERE id = v_fresque;
    UPDATE programme.sessions
       SET room_id = v_stand, starts_at = '2026-11-09 10:15+03', ends_at = '2026-11-09 11:15+03', event_day_id = NULL
     WHERE id = v_velo;
END;
$$;
COMMIT;
