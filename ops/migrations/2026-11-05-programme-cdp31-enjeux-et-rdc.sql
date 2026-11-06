-- Programme de la CdP31 (05/11/2026), à la demande du commanditaire :
--   « Enjeux de financements pour les régions et les municipalités » (séance
--   commune AIRF et CASAD-Benin) passe à 13 h 00 ;
--   « Autorité Nationale de Budgétisation Climat… » (RDC) passe à 11 h 30.
-- Même jour, même durée, même salle : seule l'heure de début change, en heure
-- d'Antalya. Comme PUT /sessions/{id}/schedule, l'UPDATE déclenche
-- programme.session.rescheduled : réunion Zoom recalée par le worker cinq
-- minutes après, avis « Session reprogrammée » aux inscrits, ligne dans « Ce qui
-- a changé ». Un chevauchement passager entre les deux n'est jamais bloqué
-- (règle 2) et disparaît au second UPDATE.
-- Séances retrouvées par leur titre : la garde exige une séance et une seule
-- pour chacun, ni annulée, ni en direct, ni terminée.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-05-programme-cdp31-enjeux-et-rdc', true);

DO $$
DECLARE
    v_edition constant uuid := '01a17da1-9b46-75b2-895a-d9e226226be2';
    v_fuseau constant text := 'Europe/Istanbul';
    v_motifs constant text[] := ARRAY['Enjeux de financements pour les régions et les municipalités%',
                                      'Autorité Nationale de Budgétisation Climat%'];
    v_heures constant time[] := ARRAY['13:00'::time, '11:30'::time];
    v_s record;
    v_n int;
    v_debut timestamptz;
BEGIN
    FOR i IN 1 .. 2 LOOP
        SELECT count(*) INTO v_n FROM programme.sessions
         WHERE event_id = v_edition AND title ->> 'fr' ILIKE v_motifs[i]
           AND status::text <> 'cancelled';
        IF v_n <> 1 THEN
            RAISE EXCEPTION 'Garde : une séance attendue pour « % », % trouvée(s)', v_motifs[i], v_n;
        END IF;

        SELECT id, status::text AS etat, starts_at, ends_at INTO v_s
          FROM programme.sessions
         WHERE event_id = v_edition AND title ->> 'fr' ILIKE v_motifs[i]
           AND status::text <> 'cancelled'
           FOR UPDATE;
        IF v_s.etat IN ('live', 'completed') THEN
            RAISE EXCEPTION 'Garde : « % » est %', v_motifs[i], v_s.etat;
        END IF;

        v_debut := ((v_s.starts_at AT TIME ZONE v_fuseau)::date + v_heures[i]) AT TIME ZONE v_fuseau;
        UPDATE programme.sessions
           SET starts_at = v_debut, ends_at = v_debut + (v_s.ends_at - v_s.starts_at)
         WHERE id = v_s.id;

        RAISE NOTICE '% : % → %, heure d''Antalya',
            v_motifs[i],
            to_char(v_s.starts_at AT TIME ZONE v_fuseau, 'DD/MM HH24:MI') || '-' || to_char(v_s.ends_at AT TIME ZONE v_fuseau, 'HH24:MI'),
            to_char(v_debut AT TIME ZONE v_fuseau, 'DD/MM HH24:MI') || '-' || to_char((v_debut + (v_s.ends_at - v_s.starts_at)) AT TIME ZONE v_fuseau, 'HH24:MI');
    END LOOP;
END;
$$;

-- Ce que voit l'équipe après coup : les autres séances du même lieu qui
-- chevauchent les nouveaux créneaux (signalé, jamais refusé).
SELECT left(a.title ->> 'fr', 50) AS deplacee, left(b.title ->> 'fr', 50) AS chevauche,
       to_char(b.starts_at AT TIME ZONE 'Europe/Istanbul', 'DD/MM HH24:MI') AS debut
  FROM programme.sessions a
  JOIN programme.sessions b
    ON b.event_id = a.event_id AND b.id <> a.id AND b.room_id IS NOT DISTINCT FROM a.room_id
   AND b.status::text <> 'cancelled' AND b.starts_at < a.ends_at AND b.ends_at > a.starts_at
 WHERE a.event_id = '01a17da1-9b46-75b2-895a-d9e226226be2'
   AND (a.title ->> 'fr' ILIKE 'Enjeux de financements pour les régions et les municipalités%'
     OR a.title ->> 'fr' ILIKE 'Autorité Nationale de Budgétisation Climat%')
   AND a.status::text <> 'cancelled';

COMMIT;
