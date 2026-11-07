-- Courriel aux inscrits de l'atelier préparatoire de la CdP31 (07/11/2026), à
-- la demande du commanditaire : l'atelier du 08/11 se tient exclusivement en
-- présentiel, les diffusions en ligne commencent le 09/11.
-- APERÇU : au commanditaire seul (angenor99@gmail.com), sans copie, avant
-- l'envoi aux inscrits (2026-11-07-courriel-atelier-presentiel.sql).
-- Même chemin que l'API : brouillon figé (queued, destinataires), travail
-- engagement.send_mailing pour le worker. Gabarit « institutionnel ».
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-07-courriel-atelier-presentiel-apercu', true);

DO $$
DECLARE
    v_acteur  uuid := '01a0627c-f3be-7185-89c8-9a4015ad158e';
    v_edition uuid := '01a17da1-9b46-75b2-895a-d9e226226be2';
    v_objet   text := 'CdP31 : l''atelier préparatoire du 8 novembre se tiendra en présentiel uniquement';
    v_corps   text := '<p>Madame, Monsieur,</p>'
        || '<p>Vous êtes inscrit·e à l''atelier préparatoire de la CdP31, le dimanche 8 novembre 2026 à Antalya.</p>'
        || '<p>Pour des raisons logistiques, cet atelier se tiendra exclusivement en présentiel, à l''hôtel DoubleTree by Hilton Antalya City Centre, et ne sera pas diffusé en ligne.</p>'
        || '<p>Les diffusions en ligne du Pavillon de la Francophonie commenceront le lundi 9 novembre. Le programme est consultable ici : <a href="https://epavillonclimatique.francophonie.org/programmations?edition=cdp31-antalya-2026">https://epavillonclimatique.francophonie.org/programmations?edition=cdp31-antalya-2026</a></p>'
        || '<p>Nous vous remercions de votre compréhension.</p>'
        || '<p>Avec nos meilleures salutations,</p>'
        || '<p>L''équipe de l''IFDD<br>Institut de la Francophonie pour le développement durable</p>';
    v_adresses platform.email[];
    v_envoi   uuid;
    v_n       int;
BEGIN
    v_adresses := ARRAY['angenor99@gmail.com']::platform.email[];

    INSERT INTO engagement.mailings (event_id, subject, body_html, extra_addresses, cc,
                                     template, created_by)
    VALUES (v_edition, v_objet, v_corps, v_adresses, '{}',
            'institutionnel', v_acteur)
    RETURNING id INTO v_envoi;

    UPDATE engagement.mailings SET status = 'queued', queued_at = now(), queued_by = v_acteur
     WHERE id = v_envoi;

    INSERT INTO engagement.mailing_recipients (mailing_id, email, person_id, variables, sources, status, last_error)
    SELECT v_envoi, a.email::platform.email, a.person_id,
           jsonb_build_object('prenom', COALESCE(a.first_name, ''), 'nom', COALESCE(a.last_name, ''),
                              'organisation', COALESCE(a.organization, ''), 'email', a.email,
                              'edition', platform.t(e.title, 'fr')),
           a.sources,
           CASE WHEN a.is_suppressed THEN 'suppressed'::engagement.mailing_recipient_status
                ELSE 'queued'::engagement.mailing_recipient_status END,
           CASE WHEN a.is_suppressed THEN 'adresse sur la liste de suppression' END
      FROM event.events e
     CROSS JOIN LATERAL engagement.mailing_audience(v_edition, '[]'::jsonb, v_adresses) a
     WHERE e.id = v_edition;
    GET DIAGNOSTICS v_n = ROW_COUNT;

    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at)
    VALUES ('email', 'engagement.send_mailing', jsonb_build_object('mailing_id', v_envoi),
            'mailing:' || v_envoi || ':start', now());
    RAISE NOTICE 'Envoi % : % destinataire(s)', v_envoi, v_n;
END;
$$;
COMMIT;
