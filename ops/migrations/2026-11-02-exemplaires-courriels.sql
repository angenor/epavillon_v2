-- Un exemplaire de chaque courriel automatique au commanditaire (02/11/2026),
-- pour qu'il voie à quoi ils ressemblent. Un travail platform.jobs par courriel,
-- sur la file que déclare son gestionnaire ; le worker les envoie aussitôt.
--
-- À jouer APRÈS la mise en ligne du code du 02/11 (confirmations d'inscription,
-- rappels de journée) et AVANT 2026-11-02-copie-courriels-automatiques.sql :
-- une fois la copie posée, l'équipe recevrait aussi ces exemplaires. Le script
-- refuse de tourner si le réglage de copie nomme une autre adresse que la sienne.
--
-- Aucun courriel ne peut partir ailleurs que chez lui :
-- - les travaux qui prennent l'adresse dans leur charge reçoivent
--   angenor99@gmail.com ;
-- - ceux qui relisent la personne en base reçoivent son identifiant, et partent
--   à son adresse principale telle qu'enregistrée (affichée en NOTICE) ;
-- - ceux qui partent d'une inscription ne sont posés que sur SES inscriptions
--   existantes, sinon sautés (NOTICE). Aucune ligne métier n'est créée.
-- Exclus : engagement.send_reminder (il lit et marque une ligne de rappel
-- programmé), engagement.regie_notice (il écrit à tous les inscrits d'une
-- activité ; ses deux courriels sont posés ici directement en
-- engagement.notification_email), negotiation.session_change_email pour une
-- session de négociation ou une réunion non annoncée (suivi et changement réel
-- dans la tranche de dix minutes exigés).
--
-- Jetons et liens personnels factices (« exemple ») : les liens mènent à une
-- page qui refuse le jeton. Rejouable : les clés d'unicité ne reposent rien.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-exemplaires-courriels', true);

DO $$
DECLARE
    v_moi       uuid := '01a0627c-f3be-7185-89c8-9a4015ad158e';
    v_edition   uuid := '01a17da1-9b46-75b2-895a-d9e226226be2';
    v_adresse   text := 'angenor99@gmail.com';
    v_jour      text := '2026-11-02';
    v_essais    smallint := 3;
    v_personne  record;
    v_copie     jsonb;
    v_s         record;
    v_espace    text;
    v_terme     text;
    v_slug      text;
    v_reunion   uuid;
    v_inscr     uuid;
    v_journee   record;
    v_n         int;
BEGIN
    SELECT p.first_name, p.display_name, p.primary_email::text AS email, p.status::text AS status
      INTO v_personne
      FROM identity.people p WHERE p.id = v_moi;
    IF NOT FOUND OR v_personne.status <> 'active' THEN
        RAISE EXCEPTION 'fiche du commanditaire % absente ou inactive', v_moi;
    END IF;
    RAISE NOTICE 'courriels relus en base : vers %', v_personne.email;
    IF engagement.is_email_suppressed(v_personne.email::platform.email)
       OR engagement.is_email_suppressed(v_adresse::platform.email) THEN
        RAISE WARNING 'adresse sur la liste de suppression : la garde d''envoi écartera ces courriels';
    END IF;

    -- La copie automatique ajouterait l'équipe à tout courriel hors liens personnels.
    SELECT value INTO v_copie FROM platform.settings WHERE key = 'engagement.mail.automatic_cc';
    IF EXISTS (SELECT 1 FROM jsonb_array_elements_text(COALESCE(v_copie, '[]'::jsonb)) a
                WHERE lower(a) <> v_adresse) THEN
        RAISE EXCEPTION 'réglage engagement.mail.automatic_cc déjà posé (%) : jouer ce script avant lui, ou le vider le temps de l''envoi', v_copie;
    END IF;

    -- Activité réelle de la CdP31 : programmée, publiée, à venir, issue d'un dossier.
    SELECT s.id, s.slug::text AS slug, e.slug::text AS event_slug, s.timezone::text AS fuseau,
           platform.t(s.title, 'fr') AS titre_fr, platform.t(s.title, 'en') AS titre_en,
           to_char(s.starts_at AT TIME ZONE s.timezone, 'DD/MM/YYYY "à" HH24"h"MI') AS debut_fr,
           to_char(s.starts_at AT TIME ZONE s.timezone, 'YYYY-MM-DD "at" HH24:MI') AS debut_en,
           p.id AS dossier, COALESCE(p.title->>'fr', p.reference_code) AS dossier_fr,
           COALESCE(p.title->>'en', p.title->>'fr', p.reference_code) AS dossier_en,
           o.legal_name AS organisation
      INTO v_s
      FROM programme.sessions s
      JOIN event.events e ON e.id = s.event_id
      JOIN programme.proposals p ON p.id = s.proposal_id AND p.deleted_at IS NULL
      JOIN org.organizations o ON o.id = p.organization_id
     WHERE s.event_id = v_edition AND s.status = 'scheduled'
       AND s.published_at IS NOT NULL AND s.starts_at > now()
     ORDER BY s.starts_at, s.id
     LIMIT 1;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'aucune activité programmée et publiée à venir pour l''édition %', v_edition;
    END IF;
    RAISE NOTICE 'activité d''exemple : % (%)', v_s.titre_fr, v_s.organisation;

    -- 1. Compte (file « default », adresse dans la charge)
    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts) VALUES
        ('default', 'identity.send_verification_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'token', 'exemple', 'client', 'web'),
         'exemplaire:identity.send_verification_email:' || v_jour, now(), v_essais),
        ('default', 'identity.send_existing_account_notice',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name),
         'exemplaire:identity.send_existing_account_notice:' || v_jour, now(), v_essais),
        ('default', 'identity.send_password_reset_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'token', 'exemple', 'client', 'web'),
         'exemplaire:identity.send_password_reset_email:' || v_jour, now(), v_essais)
    ON CONFLICT DO NOTHING;

    -- 2. Organisations (file « default », adresse dans la charge)
    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts) VALUES
        ('default', 'org.membership.invitation_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'organization_name', v_s.organisation, 'token', 'exemple'),
         'exemplaire:org.membership.invitation_email:' || v_jour, now(), v_essais),
        ('default', 'org.membership.request_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'organization_name', v_s.organisation,
                            'requester_name', v_personne.display_name),
         'exemplaire:org.membership.request_email:' || v_jour, now(), v_essais),
        ('default', 'org.membership.approved_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'organization_name', v_s.organisation),
         'exemplaire:org.membership.approved_email:' || v_jour, now(), v_essais)
    ON CONFLICT DO NOTHING;

    -- 3. Guide Négo (file « default », adresse dans la charge)
    SELECT platform.t(sp.name, 'fr') INTO v_espace
      FROM negotiation.spaces sp WHERE sp.archived_at IS NULL ORDER BY sp.id LIMIT 1;
    SELECT g.term, g.slug INTO v_terme, v_slug
      FROM negotiation.glossary_entries g WHERE g.status = 'published' ORDER BY g.id LIMIT 1;
    IF NOT FOUND THEN
        v_terme := 'Objectif mondial d''adaptation';
        v_slug  := 'objectif-mondial-adaptation';
    END IF;

    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts) VALUES
        ('default', 'negotiation.access_request.approved_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'space_name', v_espace),
         'exemplaire:negotiation.access_request.approved_email:' || v_jour, now(), v_essais),
        ('default', 'negotiation.access_request.rejected_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'reason', 'Votre nom ne figure pas sur la liste de délégation transmise par votre point focal.'),
         'exemplaire:negotiation.access_request.rejected_email:' || v_jour, now(), v_essais),
        ('default', 'negotiation.expert_question.answered_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'question', 'Quelle différence entre l''objectif mondial d''adaptation et les plans nationaux d''adaptation ?'),
         'exemplaire:negotiation.expert_question.answered_email:' || v_jour, now(), v_essais),
        ('default', 'negotiation.glossary_proposal.published_email',
         jsonb_build_object('to', v_adresse, 'locale', 'fr', 'first_name', v_personne.first_name,
                            'term', v_terme, 'slug', v_slug),
         'exemplaire:negotiation.glossary_proposal.published_email:' || v_jour, now(), v_essais)
    ON CONFLICT DO NOTHING;

    -- 4. Avis par courriel (file « email ») : la personne est relue en base.
    -- Textes composés comme comment_circle.rs (message du comité) et
    -- regie_notice.rs (annulée, maintenue). Clés hors du préfixe « regie: » :
    -- la régie ne les compte pas parmi ses envois.
    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts) VALUES
        ('email', 'engagement.notification_email',
         jsonb_build_object(
             'person_id', v_moi,
             'type_code', 'programme.comment.addressed',
             'title', jsonb_build_object('fr', 'Message du comité : ' || v_s.dossier_fr,
                                         'en', 'Committee message: ' || v_s.dossier_en),
             'body', jsonb_build_object(
                 'fr', v_personne.display_name || ' vous a adressé un message sur « ' || v_s.dossier_fr || ' ».',
                 'en', v_personne.display_name || ' sent you a message about "' || v_s.dossier_en || '".'),
             'link_path', '/admin/propositions/' || v_s.dossier),
         'exemplaire:engagement.notification_email:comite:' || v_jour, now(), v_essais),
        ('email', 'engagement.notification_email',
         jsonb_build_object(
             'person_id', v_moi,
             'type_code', 'programme.session.cancelled',
             'title', jsonb_build_object('fr', 'Activité annulée : ' || v_s.titre_fr,
                                         'en', 'Activity cancelled: ' || v_s.titre_en),
             'body', jsonb_build_object(
                 'fr', 'L''activité « ' || v_s.titre_fr || ' » prévue le ' || v_s.debut_fr || ' (' || v_s.fuseau
                       || ') est annulée. Nous vous prions de nous en excuser.',
                 'en', 'The activity "' || v_s.titre_en || '" scheduled for ' || v_s.debut_en || ' (' || v_s.fuseau
                       || ') is cancelled. We apologise for the inconvenience.'),
             'link_path', '/activity/' || v_s.event_slug || '/' || v_s.slug),
         'exemplaire:engagement.notification_email:annulee:' || v_jour, now(), v_essais),
        ('email', 'engagement.notification_email',
         jsonb_build_object(
             'person_id', v_moi,
             'type_code', 'programme.session.restored',
             'title', jsonb_build_object('fr', 'Finalement, l''activité est maintenue : ' || v_s.titre_fr,
                                         'en', 'The activity is going ahead after all: ' || v_s.titre_en),
             'body', jsonb_build_object(
                 'fr', 'Finalement, l''activité « ' || v_s.titre_fr || ' » est maintenue le ' || v_s.debut_fr || ' ('
                       || v_s.fuseau || '). Ne tenez pas compte du message d''annulation reçu plus tôt.',
                 'en', 'The activity "' || v_s.titre_en || '" is going ahead after all on ' || v_s.debut_en || ' ('
                       || v_s.fuseau || '). Please disregard the cancellation message sent earlier.'),
             'link_path', '/activity/' || v_s.event_slug || '/' || v_s.slug),
         'exemplaire:engagement.notification_email:maintenue:' || v_jour, now(), v_essais)
    ON CONFLICT DO NOTHING;

    -- 5. Sur SES inscriptions seulement, sautés sinon.

    -- Réunion de la Francophonie modifiée : il doit être dans son audience.
    SELECT m.id INTO v_reunion
      FROM negotiation.meetings m
      JOIN negotiation.meeting_registrations r ON r.meeting_id = m.id
     WHERE r.person_id = v_moi AND r.status = 'registered'
       AND m.kind IN ('preparatory_workshop', 'francophone_consultation')
       AND m.source_key IS NULL AND m.status NOT IN ('draft', 'cancelled')
     ORDER BY m.start_at DESC LIMIT 1;
    IF FOUND THEN
        INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts) VALUES
            ('default', 'negotiation.session_change_email',
             jsonb_build_object('cible', 'francophone_meeting', 'id', v_reunion, 'person_id', v_moi,
                                'tranche', floor(extract(epoch FROM now()) / 600)::bigint,
                                'etat', 'deplacee'),
             'exemplaire:negotiation.session_change_email:' || v_jour, now(), v_essais),
            ('default', 'negotiation.meeting_promotion_email',
             jsonb_build_object('meeting_id', v_reunion, 'person_id', v_moi),
             'exemplaire:negotiation.meeting_promotion_email:' || v_jour, now(), v_essais)
        ON CONFLICT DO NOTHING;
    ELSE
        RAISE NOTICE 'sauté : réunion modifiée et place obtenue (aucune inscription à une réunion de la Francophonie)';
    END IF;

    -- Confirmation d'inscription à une activité seule.
    SELECT r.id INTO v_inscr
      FROM programme.registrations r
      JOIN programme.sessions s ON s.id = r.session_id
     WHERE r.person_id = v_moi AND r.track_registration_id IS NULL
       AND r.status IN ('registered', 'waitlisted')
     ORDER BY (s.event_id = v_edition) DESC, r.id DESC LIMIT 1;
    IF FOUND THEN
        INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts) VALUES
            ('email', 'engagement.send_registration',
             jsonb_build_object('registration_id', v_inscr),
             'exemplaire:engagement.send_registration:' || v_jour, now(), v_essais)
        ON CONFLICT DO NOTHING;
    ELSE
        RAISE NOTICE 'sauté : confirmation d''inscription à une activité (aucune inscription)';
    END IF;

    -- Confirmation et rappel d'une journée. Le rappel ne part que si starts_at
    -- vaut le début actuel de la journée : calculé comme journee_pour_courriel.
    SELECT tr.id, tr.status::text AS status,
           (SELECT min(x.d) FROM (
                SELECT s.starts_at AS d
                  FROM programme.session_tracks st
                  JOIN programme.sessions s ON s.id = st.session_id
                 WHERE st.track_id = tr.track_id AND s.status <> 'cancelled'
                UNION ALL
                SELECT m.starts_at FROM event.programme_track_moments m
                 WHERE m.track_id = tr.track_id) x) AS debut
      INTO v_journee
      FROM programme.track_registrations tr
      JOIN event.programme_tracks t ON t.id = tr.track_id
     WHERE tr.person_id = v_moi AND tr.status IN ('registered', 'attended')
     ORDER BY (t.event_id = v_edition) DESC, tr.id DESC LIMIT 1;
    IF FOUND THEN
        INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts) VALUES
            ('email', 'engagement.send_day_registration',
             jsonb_build_object('track_registration_id', v_journee.id, 'resent', false),
             'exemplaire:engagement.send_day_registration:' || v_jour, now(), v_essais)
        ON CONFLICT DO NOTHING;
        IF v_journee.debut IS NOT NULL THEN
            INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts) VALUES
                ('email', 'engagement.send_day_reminder',
                 jsonb_build_object('track_registration_id', v_journee.id, 'offset_minutes', 1440,
                                    'starts_at', floor(extract(epoch FROM v_journee.debut))::bigint),
                 'exemplaire:engagement.send_day_reminder:' || v_jour, now(), v_essais)
            ON CONFLICT DO NOTHING;
        ELSE
            RAISE NOTICE 'sauté : rappel de journée (journée sans activité ni moment daté)';
        END IF;
    ELSE
        RAISE NOTICE 'sauté : confirmation et rappel de journée (aucune inscription à une journée)';
    END IF;

    SELECT count(*) INTO v_n FROM platform.jobs
     WHERE idempotency_key LIKE 'exemplaire:%:' || v_jour AND status <> 'cancelled';
    RAISE NOTICE '% exemplaires en file', v_n;
END
$$;

COMMIT;
