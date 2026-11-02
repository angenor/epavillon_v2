-- Courriel « pas de créneau à ce stade » aux déposants des dossiers de la CdP31
-- en évaluation ou déposés (02/11/2026), à la demande du commanditaire.
-- ENVOI RÉEL, après l'essai (2026-11-02-courriel-sans-creneau-essai) : tous
-- les dossiers en évaluation ou déposés, hors ses deux dossiers de test ; un
-- seul courriel par déposant et titre (CDP31-00185/00186 et 00225/00227 sont
-- des doublons). En copie : angenor99@gmail.com et issa.bado@francophonie.org.
--
-- Un envoi par dossier, comme « Écrire au déposant » de la fiche d'un dossier
-- (proposal_id posé, {{activite}} résolu par destinataire) : un envoi groupé ne
-- connaît pas le titre de chacun. Même chemin que l'API : brouillon, figé
-- (status queued, destinataires), travail engagement.send_mailing pour le worker.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-courriel-sans-creneau', true);

DO $$
DECLARE
    v_acteur  uuid := '01a0627c-f3be-7185-89c8-9a4015ad158e';
    v_edition uuid := '01a17da1-9b46-75b2-895a-d9e226226be2';
    v_objet   text := 'CdP31 : votre proposition d''activité au Pavillon de la Francophonie';
    v_corps   text := '<p>Madame, Monsieur,</p>'
        || '<p>Nous vous remercions pour la proposition d''activité « {{activite}} » que vous avez déposée en vue du Pavillon de la Francophonie à la CdP31.</p>'
        || '<p>L''appel a suscité un nombre de propositions bien supérieur aux créneaux disponibles, et nous n''avons pas pu, à ce stade, inscrire votre activité au programme.</p>'
        || '<p>Le programme du Pavillon est consultable ici : <a href="https://epavillonclimatique.francophonie.org/programmations?edition=cdp31-antalya-2026">https://epavillonclimatique.francophonie.org/programmations?edition=cdp31-antalya-2026</a> . Toutes les activités sont retransmises en direct.</p>'
        || '<p>Avec nos remerciements et nos meilleures salutations,</p>'
        || '<p>L''équipe de l''IFDD<br>Institut de la Francophonie pour le développement durable</p>';
    v_d     record;
    v_envoi uuid;
    v_n     int := 0;
BEGIN
    FOR v_d IN
        SELECT DISTINCT ON (pe.primary_email, platform.t(p.title, 'fr'))
               p.id, p.reference_code, platform.t(p.title, 'fr') AS titre, pe.primary_email::text AS deposant
          FROM programme.proposals p
          JOIN identity.people pe ON pe.id = p.submitted_by
         WHERE p.event_id = v_edition AND p.deleted_at IS NULL
           AND p.reference_code NOT IN ('CDP31-00119', 'CDP31-00120')
           AND p.status IN ('under_review', 'submitted')
         ORDER BY pe.primary_email, platform.t(p.title, 'fr'), p.reference_code
    LOOP
        INSERT INTO engagement.mailings (event_id, subject, body_html, extra_addresses, cc,
                                         template, created_by, proposal_id)
        VALUES (v_edition, v_objet, v_corps, ARRAY[v_d.deposant]::platform.email[],
                ARRAY['angenor99@gmail.com', 'issa.bado@francophonie.org']::platform.email[],
                'institutionnel', v_acteur, v_d.id)
        RETURNING id INTO v_envoi;

        UPDATE engagement.mailings SET status = 'queued', queued_at = now(), queued_by = v_acteur
         WHERE id = v_envoi;

        INSERT INTO engagement.mailing_recipients (mailing_id, email, person_id, variables, sources, status, last_error)
        SELECT v_envoi, a.email::platform.email, a.person_id,
               jsonb_build_object('prenom', COALESCE(a.first_name, ''), 'nom', COALESCE(a.last_name, ''),
                                  'organisation', COALESCE(a.organization, ''), 'email', a.email,
                                  'edition', platform.t(e.title, 'fr'), 'activite', v_d.titre),
               a.sources,
               CASE WHEN a.is_suppressed THEN 'suppressed'::engagement.mailing_recipient_status
                    ELSE 'queued'::engagement.mailing_recipient_status END,
               CASE WHEN a.is_suppressed THEN 'adresse sur la liste de suppression' END
          FROM event.events e
         CROSS JOIN LATERAL engagement.mailing_audience(v_edition, '[]'::jsonb, ARRAY[v_d.deposant]::platform.email[]) a
         WHERE e.id = v_edition;

        INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at)
        VALUES ('email', 'engagement.send_mailing', jsonb_build_object('mailing_id', v_envoi),
                'mailing:' || v_envoi || ':start', now());
        v_n := v_n + 1;
    END LOOP;

    IF v_n <> 60 THEN
        RAISE EXCEPTION 'Garde : 60 courriels attendus, % préparés', v_n;
    END IF;
END;
$$;
COMMIT;
