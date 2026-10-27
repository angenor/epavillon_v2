-- Intervenant sans adresse électronique (décision D16, 27/10). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql § 4, 075_programme_sessions.sql § 2.
--
-- 1. proposal_speakers et session_speakers : person_id facultatif ; sans personne,
--    la ligne porte civilité, prénom et nom (ck_*_identity : une seule source).
-- 2. tg_check_submission_eligibility : la civilité d'un intervenant sans personne
--    se lit sur la ligne.
--
-- Déployer l'API avec : elle écrit et lit ces colonnes.
BEGIN;

ALTER TABLE programme.proposal_speakers ALTER COLUMN person_id DROP NOT NULL;
ALTER TABLE programme.proposal_speakers
    ADD COLUMN IF NOT EXISTS civility text CHECK (civility IN ('mme', 'm', 'dr', 'pr', 'other')),
    ADD COLUMN IF NOT EXISTS first_name text,
    ADD COLUMN IF NOT EXISTS last_name text;
ALTER TABLE programme.proposal_speakers DROP CONSTRAINT IF EXISTS ck_proposal_speakers_identity;
ALTER TABLE programme.proposal_speakers ADD CONSTRAINT ck_proposal_speakers_identity CHECK (
    (person_id IS NOT NULL AND civility IS NULL AND first_name IS NULL AND last_name IS NULL)
    OR (person_id IS NULL AND length(btrim(first_name)) > 0 AND length(btrim(last_name)) > 0));

ALTER TABLE programme.session_speakers ALTER COLUMN person_id DROP NOT NULL;
ALTER TABLE programme.session_speakers
    ADD COLUMN IF NOT EXISTS civility text CHECK (civility IN ('mme', 'm', 'dr', 'pr', 'other')),
    ADD COLUMN IF NOT EXISTS first_name text,
    ADD COLUMN IF NOT EXISTS last_name text;
ALTER TABLE programme.session_speakers DROP CONSTRAINT IF EXISTS ck_session_speakers_identity;
ALTER TABLE programme.session_speakers ADD CONSTRAINT ck_session_speakers_identity CHECK (
    (person_id IS NOT NULL AND civility IS NULL AND first_name IS NULL AND last_name IS NULL)
    OR (person_id IS NULL AND length(btrim(first_name)) > 0 AND length(btrim(last_name)) > 0));

-- Copie de docs/database/070_programme_proposals.sql (LEFT JOIN identity.people).
CREATE OR REPLACE FUNCTION programme.tg_check_submission_eligibility()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_call  event.calls_for_proposals%ROWTYPE;
    v_count integer;
    v_is_first_submission boolean;
BEGIN
    IF NEW.status <> 'submitted' OR (TG_OP = 'UPDATE' AND OLD.status = 'submitted') THEN
        RETURN NEW;
    END IF;

    IF NEW.call_id IS NULL THEN
        RETURN NEW;  -- proposition hors appel, créée par l'IFDD
    END IF;

    -- Premier dépôt : insertion, ou passage depuis le brouillon. Tout autre
    -- chemin vers `submitted` est un RENVOI — aujourd'hui depuis
    -- `changes_requested`, seul état que la machine autorise à y revenir.
    v_is_first_submission := (TG_OP = 'INSERT') OR (OLD.status = 'draft');

    SELECT * INTO v_call FROM event.calls_for_proposals WHERE id = NEW.call_id;

    IF v_is_first_submission
       AND (v_call.status <> 'open'
            OR now() < v_call.opens_at
            OR now() > COALESCE(v_call.extended_until, v_call.closes_at)) THEN
        RAISE EXCEPTION 'L''appel « % » n''accepte plus de soumission (échéance : %).',
            platform.t(v_call.title), COALESCE(v_call.extended_until, v_call.closes_at)
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF v_call.max_proposals_per_organization IS NOT NULL THEN
        SELECT count(*) INTO v_count
        FROM programme.proposals p
        WHERE p.call_id = NEW.call_id
          AND p.organization_id = NEW.organization_id
          AND p.id <> NEW.id
          AND p.status NOT IN ('draft', 'withdrawn', 'rejected')
          AND p.deleted_at IS NULL;

        IF v_count >= v_call.max_proposals_per_organization THEN
            RAISE EXCEPTION 'Plafond atteint : % proposition(s) au maximum par organisation sur cet appel.',
                v_call.max_proposals_per_organization
                USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;

    IF v_call.requires_verified_organization THEN
        PERFORM 1 FROM org.organizations o
        WHERE o.id = NEW.organization_id AND o.verified_at IS NOT NULL;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'Cet appel est réservé aux organisations vérifiées par l''IFDD.'
                USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;

    -- LES INTERVENANTS D'UN DOSSIER DÉPOSÉ SONT COMPLETS (ajouté le 17/09). Le
    -- formulaire l'exigeait seul : un dossier écrit hors de lui partait au comité
    -- avec un intervenant sans civilité ni organisation. Vérifié au passage vers
    -- `submitted` par mise à jour — dépôt et renvoi — et non à l'insertion
    -- directe d'un dossier déjà déposé, qui n'a pas encore d'intervenants : ce
    -- chemin est celui des reprises de données, qui répondent de leur contenu.
    IF TG_OP = 'UPDATE' THEN
        SELECT count(*) INTO v_count
        FROM programme.proposal_speakers s
        WHERE s.proposal_id = NEW.id;

        IF v_count < v_call.min_speakers OR v_count > v_call.max_speakers THEN
            RAISE EXCEPTION 'Cet appel demande entre % et % intervenant(s) ; le dossier en compte %.',
                v_call.min_speakers, v_call.max_speakers, v_count
                USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_proposal_speakers_count';
        END IF;

        PERFORM 1
        FROM programme.proposal_speakers s
        LEFT JOIN identity.people p ON p.id = s.person_id
        WHERE s.proposal_id = NEW.id
          AND (COALESCE(s.civility, p.civility) IS NULL
               OR btrim(COALESCE(s.job_title_snapshot, '')) = ''
               OR btrim(COALESCE(s.organization_snapshot, '')) = '');
        IF FOUND THEN
            RAISE EXCEPTION 'Chaque intervenant doit avoir une civilité, une fonction et une organisation.'
                USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_proposal_speakers_complete';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

COMMIT;
