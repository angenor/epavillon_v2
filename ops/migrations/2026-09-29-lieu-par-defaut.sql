-- Chaque édition naît avec son lieu et sa salle unique ; les éditions existantes
-- sans salle reçoivent les leurs. Modèle : docs/database/060_events.sql.
BEGIN;
CREATE OR REPLACE FUNCTION event.tg_create_default_venue()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_venue uuid;
BEGIN
    IF NEW.participation_mode = 'online' THEN
        INSERT INTO event.venues (event_id, name, kind)
        VALUES (NEW.id, '{"fr":"En ligne","en":"Online"}', 'virtual')
        RETURNING id INTO v_venue;
        INSERT INTO event.rooms (venue_id, name, code, is_virtual, has_streaming)
        VALUES (v_venue, '{"fr":"En ligne","en":"Online"}', 'en-ligne', true, true);
    ELSIF NEW.has_pavilion THEN
        INSERT INTO event.venues (event_id, name, kind)
        VALUES (NEW.id, '{"fr":"Pavillon de la Francophonie","en":"Francophonie Pavilion"}', 'pavilion')
        RETURNING id INTO v_venue;
        INSERT INTO event.rooms (venue_id, name, code, has_streaming)
        VALUES (v_venue, '{"fr":"Stand","en":"Stand"}', 'stand', true);
    ELSE
        INSERT INTO event.venues (event_id, name, kind)
        VALUES (NEW.id, '{"fr":"Lieu principal","en":"Main venue"}', 'other')
        RETURNING id INTO v_venue;
        INSERT INTO event.rooms (venue_id, name, code)
        VALUES (v_venue, '{"fr":"Salle principale","en":"Main room"}', 'salle-principale');
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_events_default_venue
    AFTER INSERT ON event.events
    FOR EACH ROW EXECUTE FUNCTION event.tg_create_default_venue();

-- Le déclencheur ne vaut que pour les éditions à venir : on rejoue sa règle sur
-- celles qui n'ont aucune salle.
DO $$
DECLARE
    v_event record;
    v_venue uuid;
BEGIN
    FOR v_event IN
        SELECT e.id, e.participation_mode, e.has_pavilion FROM event.events e
         WHERE NOT EXISTS (SELECT 1 FROM event.venues v JOIN event.rooms r ON r.venue_id = v.id
                            WHERE v.event_id = e.id)
    LOOP
        IF v_event.participation_mode = 'online' THEN
            INSERT INTO event.venues (event_id, name, kind)
            VALUES (v_event.id, '{"fr":"En ligne","en":"Online"}', 'virtual') RETURNING id INTO v_venue;
            INSERT INTO event.rooms (venue_id, name, code, is_virtual, has_streaming)
            VALUES (v_venue, '{"fr":"En ligne","en":"Online"}', 'en-ligne', true, true);
        ELSIF v_event.has_pavilion THEN
            INSERT INTO event.venues (event_id, name, kind)
            VALUES (v_event.id, '{"fr":"Pavillon de la Francophonie","en":"Francophonie Pavilion"}', 'pavilion') RETURNING id INTO v_venue;
            INSERT INTO event.rooms (venue_id, name, code, has_streaming)
            VALUES (v_venue, '{"fr":"Stand","en":"Stand"}', 'stand', true);
        ELSE
            INSERT INTO event.venues (event_id, name, kind)
            VALUES (v_event.id, '{"fr":"Lieu principal","en":"Main venue"}', 'other') RETURNING id INTO v_venue;
            INSERT INTO event.rooms (venue_id, name, code)
            VALUES (v_venue, '{"fr":"Salle principale","en":"Main room"}', 'salle-principale');
        END IF;
    END LOOP;
END;
$$;
COMMIT;
