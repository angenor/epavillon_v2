-- Activité de l'IFDD absente du programme, programmée en urgence le 10/11/2026
-- sans appel (proposal_id NULL, cas prévu par programme.sessions) :
-- « Lutte contre la désinformation climatique en Francophonie », portée par
-- l'IFDD seule (arbitrage du commanditaire), 10/11 de 16 h 30 à 19 h 00,
-- heure d'Antalya.
--
-- Réglages recopiés de l'activité voisine du même jour (stand, présentiel,
-- diffusée, inscription demandée) : la réunion Zoom naît à la première
-- inscription, comme pour les autres. Publiée d'office.
-- Idempotent : relancé, il ne crée rien de plus.

\set ON_ERROR_STOP on
BEGIN;

DO $$
DECLARE
    v_ref    programme.sessions%ROWTYPE;
    v_id     uuid;
    v_ifdd   uuid := '01a06241-e7fa-78de-af7c-101f2273be63';
    v_slug   text := 'lutte-contre-la-desinformation-climatique-en-francophonie';
BEGIN
    SELECT * INTO v_ref FROM programme.sessions WHERE id = '01a17da1-9bea-7eae-a833-bc3b74d15dca';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Activité de référence du 10/11 introuvable : rien n''est créé.';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM org.organizations WHERE id = v_ifdd) THEN
        RAISE EXCEPTION 'Fiche IFDD introuvable : rien n''est créé.';
    END IF;

    SELECT id INTO v_id FROM programme.sessions WHERE event_id = v_ref.event_id AND slug = v_slug;
    IF FOUND THEN
        RAISE NOTICE 'Déjà créée : %', v_id;
        RETURN;
    END IF;

    INSERT INTO programme.sessions (
        event_id, organization_id, title, slug, summary, status, format,
        starts_at, ends_at, timezone, room_id, location_note,
        registration_required, capacity, waitlist_enabled, registration_form_id,
        is_streamed, is_recorded, allows_questions, published_at)
    VALUES (
        v_ref.event_id, v_ifdd,
        jsonb_build_object('fr', 'Lutte contre la désinformation climatique en Francophonie',
                           'en', 'Tackling climate disinformation in the French-speaking world'),
        v_slug, NULL, 'scheduled', v_ref.format,
        timestamptz '2026-11-10 16:30 Europe/Istanbul',
        timestamptz '2026-11-10 19:00 Europe/Istanbul',
        v_ref.timezone, v_ref.room_id, v_ref.location_note,
        v_ref.registration_required, v_ref.capacity, v_ref.waitlist_enabled, v_ref.registration_form_id,
        v_ref.is_streamed, v_ref.is_recorded, v_ref.allows_questions, now())
    RETURNING id INTO v_id;

    RAISE NOTICE 'Créée : %', v_id;
END $$;

SELECT s.id, s.slug, s.status, s.published_at IS NOT NULL AS publiee,
       to_char(s.starts_at AT TIME ZONE s.timezone, 'DD/MM HH24:MI') AS debut,
       to_char(s.ends_at AT TIME ZONE s.timezone, 'HH24:MI') AS fin,
       s.is_streamed, s.registration_required
FROM programme.sessions s
WHERE s.slug = 'lutte-contre-la-desinformation-climatique-en-francophonie';

COMMIT;
