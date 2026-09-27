-- =============================================================================
-- ePavillon v2 — migration de l'étape 4 de Guide Négo : les réunions de la
-- Francophonie
--
-- Du schéma en service après l'étape 3b (specs/015-guide-nego-signalements/
-- migration.sql) vers celui de docs/database/ après l'étape 4.
--
-- POURQUOI CE FICHIER EXISTE
--   `docs/database/` n'est chargé qu'à la création du volume : une base qui
--   porte des comptes se migre, selon le § 13 de docs/DEPLOIEMENT.md. Il vaut
--   AUSSI en local. **`down -v` n'est jamais la procédure.**
--
-- REJOUABLE SANS DÉGÂT
--   Colonnes et index sous condition ; contraintes retirées puis reposées ;
--   triggers retirés puis reposés ; fonctions remplacées ; semis sous
--   ON CONFLICT. Les textes — commentaires compris — sont ceux du modèle,
--   MOT POUR MOT.
--
-- AUCUNE SESSION IMPORTÉE N'EST TOUCHÉE
--   Toutes les règles nouvelles de negotiation.meetings ne visent que les
--   réunions saisies (source_key nul) de kind preparatory_workshop ou
--   francophone_consultation.
--
-- ORDRE
--   1. reference : le vocabulaire francophone_meeting_type
--   2. negotiation.meetings : colonnes, contraintes, index, gardes
--   3. negotiation.meeting_registrations : colonnes, contraintes, audit, jauge,
--      promotion, destinataires
--   4. semis : les deux types de notification
--
-- CONTRÔLE APRÈS COUP (§ 13, étape 6)
--   pg_dump --schema-only de cette base, comparé à celui d'une base chargée
--   depuis docs/database/. Comparer après tri des lignes.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- 1. Le vocabulaire des natures (R2)
-- -----------------------------------------------------------------------------
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('francophone_meeting_type', '{"fr":"Natures de réunion de la Francophonie","en":"Francophonie meeting types"}', '{"fr":"Nature d''une réunion organisée pour les négociatrices et négociateurs francophones : atelier préparatoire, concertation…","en":"Nature of a meeting held for Francophone negotiators: preparatory workshop, consultation…"}', false, false, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('francophone_meeting_type', 'preparatory_workshop',     '{"fr":"Atelier préparatoire","en":"Preparatory workshop"}', 10),
    ('francophone_meeting_type', 'negotiators_consultation', '{"fr":"Concertation des négociatrices et négociateurs","en":"Negotiators'' consultation"}', 20),
    ('francophone_meeting_type', 'ministerial_consultation', '{"fr":"Concertation ministérielle","en":"Ministerial consultation"}', 30)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 2. negotiation.meetings — les réunions de la Francophonie (R1, R3, R7, R9 bis)
-- -----------------------------------------------------------------------------
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS francophone_type_term_id uuid
    REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS access_audience       platform.i18n_text;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS pavilion_session_id   uuid;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS requires_registration boolean NOT NULL DEFAULT true;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS waitlist_enabled      boolean NOT NULL DEFAULT true;

ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS xmod_fk_negotiation_meetings_pavilion_session;
ALTER TABLE negotiation.meetings ADD CONSTRAINT xmod_fk_negotiation_meetings_pavilion_session
    FOREIGN KEY (pavilion_session_id) REFERENCES programme.sessions(id) ON DELETE SET NULL;

ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_francophone_type;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_francophone_type
        CHECK (source_key IS NOT NULL OR status = 'draft'
            OR kind NOT IN ('preparatory_workshop', 'francophone_consultation')
            OR francophone_type_term_id IS NOT NULL);
ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_francophone_event;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_francophone_event
        CHECK (source_key IS NOT NULL
            OR kind NOT IN ('preparatory_workshop', 'francophone_consultation')
            OR event_id IS NOT NULL);
ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_francophone_kind;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_francophone_kind
        CHECK (francophone_type_term_id IS NULL
            OR (source_key IS NULL AND kind IN ('preparatory_workshop', 'francophone_consultation')));
ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_access_audience;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_access_audience
        CHECK (source_key IS NOT NULL
            OR kind NOT IN ('preparatory_workshop', 'francophone_consultation')
            OR (is_open_access IS NOT NULL AND (is_open_access OR access_audience IS NOT NULL)));

CREATE INDEX IF NOT EXISTS ix_meetings_francophonie ON negotiation.meetings (event_id, start_at)
    WHERE kind IN ('preparatory_workshop', 'francophone_consultation') AND status <> 'draft';

DROP TRIGGER IF EXISTS tg_meetings_check_francophone_type ON negotiation.meetings;
CREATE TRIGGER tg_meetings_check_francophone_type
    BEFORE INSERT OR UPDATE OF francophone_type_term_id ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'francophone_type_term_id', 'francophone_meeting_type');

-- La nature dit le parcours : un atelier préparatoire est de kind
-- preparatory_workshop, une concertation de kind francophone_consultation
-- (specs/016-guide-nego-reunions, R9 bis). Un CHECK ne lit pas le code du
-- terme : le refus porte le nom de contrainte ck_meetings_francophone_kind,
-- que l'API traduit comme les autres.
CREATE OR REPLACE FUNCTION negotiation.tg_check_meeting_francophone_kind()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_code text;
BEGIN
    IF NEW.francophone_type_term_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT t.code INTO v_code
      FROM reference.taxonomy_terms t
     WHERE t.id = NEW.francophone_type_term_id AND t.taxonomy_code = 'francophone_meeting_type';
    -- Un terme d'une autre taxonomie est refusé par tg_meetings_check_francophone_type.
    IF v_code IS NOT NULL AND NEW.kind IS DISTINCT FROM (CASE v_code
            WHEN 'preparatory_workshop' THEN 'preparatory_workshop'
            ELSE 'francophone_consultation' END)::negotiation.meeting_kind THEN
        RAISE EXCEPTION 'La nature « % » ne correspond pas au type de réunion « % ».', v_code, NEW.kind
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_meetings_francophone_kind';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_meetings_check_francophone_kind ON negotiation.meetings;
CREATE TRIGGER tg_meetings_check_francophone_kind
    BEFORE INSERT OR UPDATE OF francophone_type_term_id, kind ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_meeting_francophone_kind();

-- Le lien au Pavillon ne vaut que pour une activité de la même édition (R7).
-- Une lecture de `programme`, jamais une écriture.
CREATE OR REPLACE FUNCTION negotiation.tg_check_meeting_pavilion_edition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.pavilion_session_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM programme.sessions s
         WHERE s.id = NEW.pavilion_session_id AND s.event_id = NEW.event_id
    ) THEN
        RAISE EXCEPTION 'L''activité du Pavillon % n''appartient pas à l''édition de la réunion.',
            NEW.pavilion_session_id
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_meetings_pavilion_edition';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_meetings_check_pavilion_edition ON negotiation.meetings;
CREATE TRIGGER tg_meetings_check_pavilion_edition
    BEFORE INSERT OR UPDATE OF pavilion_session_id, event_id ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_meeting_pavilion_edition();

COMMENT ON COLUMN negotiation.meetings.is_open_access IS
    'Ouverte (vrai) ou à accès limité (faux) : selon la source pour une session importée, selon l''IFDD pour une réunion saisie — requise pour une réunion de la Francophonie (ck_meetings_access_audience). Une information, jamais une règle de lecture ni d''inscription.';
COMMENT ON COLUMN negotiation.meetings.francophone_type_term_id IS
    'Nature d''une réunion de la Francophonie (francophone_meeting_type) ; requise hors brouillon (ck_meetings_francophone_type), accordée au kind (ck_meetings_francophone_kind). Nulle pour toute autre réunion.';
COMMENT ON COLUMN negotiation.meetings.access_audience IS
    'Public d''une réunion à accès limité, dit à la personne : « ministres et chefs de délégation ». Requis quand is_open_access est faux sur une réunion de la Francophonie.';
COMMENT ON COLUMN negotiation.meetings.pavilion_session_id IS
    'Activité du Pavillon liée, de la même édition (tg_meetings_check_pavilion_edition). Une clé, rien d''autre : l''activité supprimée, le lien tombe.';
COMMENT ON COLUMN negotiation.meetings.requires_registration IS
    'Faux : ni inscription, ni capacité, ni liste d''attente ; le lien de connexion est servi à toute personne admise.';
COMMENT ON COLUMN negotiation.meetings.waitlist_enabled IS
    'Vrai : une réunion complète met en liste d''attente. Faux : elle refuse (meeting_full).';
COMMENT ON COLUMN negotiation.meetings.registered_count IS
    'Inscrites (registered), tenu par tg_sync_registered_count en fin de ligne. Pour l''affichage seulement : la jauge se calcule par count(*) sous verrou, jamais sur ce compteur. Chaque inscription réécrit la réunion : son updated_at ne garde pas une édition au back-office.';

-- -----------------------------------------------------------------------------
-- 3. negotiation.meeting_registrations — la jauge et la liste d'attente (R4)
-- -----------------------------------------------------------------------------
ALTER TABLE negotiation.meeting_registrations ADD COLUMN IF NOT EXISTS waitlist_position integer;
ALTER TABLE negotiation.meeting_registrations ADD COLUMN IF NOT EXISTS client_ref        uuid;

ALTER TABLE negotiation.meeting_registrations DROP CONSTRAINT IF EXISTS ux_meeting_registrations_client_ref;
ALTER TABLE negotiation.meeting_registrations ADD CONSTRAINT ux_meeting_registrations_client_ref
    UNIQUE (person_id, client_ref);
ALTER TABLE negotiation.meeting_registrations DROP CONSTRAINT IF EXISTS ck_meeting_registrations_waitlist;
ALTER TABLE negotiation.meeting_registrations ADD CONSTRAINT ck_meeting_registrations_waitlist
    CHECK ((status = 'waitlisted') = (waitlist_position IS NOT NULL));

DROP TRIGGER IF EXISTS tg_meeting_registrations_audit ON negotiation.meeting_registrations;
CREATE TRIGGER tg_meeting_registrations_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.meeting_registrations
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON COLUMN negotiation.meeting_registrations.waitlist_position IS
    'Rang dans la liste d''attente, à partir de 1 ; posé si et seulement si la ligne est waitlisted (ck_meeting_registrations_waitlist). Recompacté par promote_meeting_waitlist.';
COMMENT ON COLUMN negotiation.meeting_registrations.client_ref IS
    'Référence du geste posée par le téléphone, neuve à chaque inscription ou réinscription : un envoi rejoué retrouve la ligne au lieu d''écrire deux fois (ux_meeting_registrations_client_ref).';

-- La jauge, tenue en base (specs/016-guide-nego-reunions, R4).
--
-- Les contrôles ne portent que sur une ENTRÉE : un INSERT, ou cancelled →
-- registered|waitlisted (réinscription). Se désinscrire n'est jamais contrôlé ;
-- waitlisted → registered (promotion) non plus : une place obtenue après la
-- fermeture des inscriptions reste obtenue.
--
-- La réunion est verrouillée (FOR UPDATE) et les places prises se comptent
-- par count(*) : registered_count n'est tenu qu'en fin de ligne par un trigger
-- AFTER, deux inscriptions simultanées y liraient la même valeur.
--
-- Les refus portent un nom de contrainte stable, que l'API traduit :
--   meeting_unavailable  réunion non publiée, annulée ou commencée
--   meeting_closed       inscription non demandée, ou hors de la fenêtre
--   meeting_full         complet, sans liste d'attente
CREATE OR REPLACE FUNCTION negotiation.tg_validate_meeting_registration()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_meeting negotiation.meetings%ROWTYPE;
    v_taken   integer;
BEGIN
    IF NEW.status = 'cancelled' THEN
        NEW.cancelled_at := COALESCE(NEW.cancelled_at, now());
        NEW.waitlist_position := NULL;
        RETURN NEW;
    END IF;

    IF TG_OP = 'UPDATE' AND OLD.status <> 'cancelled' THEN
        IF NEW.status = 'registered' THEN
            NEW.waitlist_position := NULL;
        END IF;
        RETURN NEW;
    END IF;

    SELECT * INTO v_meeting FROM negotiation.meetings WHERE id = NEW.meeting_id FOR UPDATE;

    IF v_meeting.status IS DISTINCT FROM 'scheduled' OR now() >= v_meeting.start_at THEN
        RAISE EXCEPTION 'meeting_unavailable : cette réunion n''accepte pas d''inscription (non publiée, annulée ou déjà commencée).'
            USING ERRCODE = 'restrict_violation', CONSTRAINT = 'meeting_unavailable';
    END IF;

    IF NOT v_meeting.requires_registration
       OR (v_meeting.registration_opens_at IS NOT NULL AND now() < v_meeting.registration_opens_at)
       OR (v_meeting.registration_closes_at IS NOT NULL AND now() >= v_meeting.registration_closes_at) THEN
        RAISE EXCEPTION 'meeting_closed : les inscriptions à cette réunion ne sont pas ouvertes.'
            USING ERRCODE = 'restrict_violation', CONSTRAINT = 'meeting_closed';
    END IF;

    SELECT count(*) INTO v_taken
      FROM negotiation.meeting_registrations r
     WHERE r.meeting_id = NEW.meeting_id AND r.status = 'registered' AND r.id <> NEW.id;

    -- Une liste d'attente non vide passe avant la nouvelle venue, même si une
    -- place s'est libérée sans promotion encore.
    IF (v_meeting.capacity IS NOT NULL AND v_taken >= v_meeting.capacity)
       OR EXISTS (SELECT 1 FROM negotiation.meeting_registrations r
                   WHERE r.meeting_id = NEW.meeting_id AND r.status = 'waitlisted' AND r.id <> NEW.id) THEN
        IF NOT v_meeting.waitlist_enabled THEN
            RAISE EXCEPTION 'meeting_full : cette réunion est complète.'
                USING ERRCODE = 'restrict_violation', CONSTRAINT = 'meeting_full';
        END IF;
        NEW.status := 'waitlisted';
        SELECT COALESCE(max(r.waitlist_position), 0) + 1 INTO NEW.waitlist_position
          FROM negotiation.meeting_registrations r
         WHERE r.meeting_id = NEW.meeting_id AND r.status = 'waitlisted';
    ELSE
        NEW.status := 'registered';
        NEW.waitlist_position := NULL;
    END IF;

    NEW.cancelled_at := NULL;
    IF TG_OP = 'UPDATE' THEN
        NEW.registered_at := now();
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_meeting_registrations_validate ON negotiation.meeting_registrations;
CREATE TRIGGER tg_meeting_registrations_validate
    BEFORE INSERT OR UPDATE OF status ON negotiation.meeting_registrations
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_validate_meeting_registration();

-- Appelée à chaque désinscription et quand l'administration relève ou retire
-- la capacité, dans la même transaction. Le paramètre `p_count` du patron de
-- programme disparaît : les places libres se calculent ici, sous le verrou.
CREATE OR REPLACE FUNCTION negotiation.promote_meeting_waitlist(p_meeting_id uuid)
RETURNS SETOF uuid
LANGUAGE plpgsql
AS $$
DECLARE
    v_capacity integer;
    v_free     integer;
BEGIN
    SELECT m.capacity INTO v_capacity FROM negotiation.meetings m WHERE m.id = p_meeting_id FOR UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    IF v_capacity IS NOT NULL THEN
        SELECT greatest(v_capacity - count(*), 0) INTO v_free
          FROM negotiation.meeting_registrations r
         WHERE r.meeting_id = p_meeting_id AND r.status = 'registered';
    END IF;

    -- LIMIT NULL : sans capacité, toute la liste passe.
    RETURN QUERY
    WITH suivantes AS (
        SELECT r.id
          FROM negotiation.meeting_registrations r
         WHERE r.meeting_id = p_meeting_id AND r.status = 'waitlisted'
         ORDER BY r.waitlist_position, r.registered_at
         LIMIT v_free
    )
    UPDATE negotiation.meeting_registrations r
       SET status = 'registered', waitlist_position = NULL
      FROM suivantes s
     WHERE r.id = s.id
    RETURNING r.person_id;

    WITH rangs AS (
        SELECT r.id, row_number() OVER (ORDER BY r.waitlist_position, r.registered_at)::integer AS rang
          FROM negotiation.meeting_registrations r
         WHERE r.meeting_id = p_meeting_id AND r.status = 'waitlisted'
    )
    UPDATE negotiation.meeting_registrations r
       SET waitlist_position = g.rang
      FROM rangs g
     WHERE r.id = g.id AND r.waitlist_position <> g.rang;
END;
$$;

COMMENT ON FUNCTION negotiation.promote_meeting_waitlist(uuid) IS
    'Promeut la liste d''attente dans l''ordre, autant que de places libres (capacity − count(registered) sous verrou ; toute la liste sans capacité), recompacte les rangs et rend les personnes promues — à prévenir.';

-- Qui prévenir d'un changement d'une réunion de la Francophonie : lue par
-- negotiation seul, comme change_recipients (R8).
CREATE OR REPLACE FUNCTION negotiation.meeting_audience(p_meeting_id uuid)
RETURNS SETOF uuid
LANGUAGE sql
STABLE
AS $$
    SELECT r.person_id
      FROM negotiation.meeting_registrations r
     WHERE r.meeting_id = p_meeting_id AND r.status IN ('registered', 'waitlisted');
$$;

COMMENT ON FUNCTION negotiation.meeting_audience(uuid) IS
    'Personnes à prévenir d''une annulation ou d''un changement de réunion : inscrites et en liste d''attente, désinscrites exclues.';

-- -----------------------------------------------------------------------------
-- 4. Semis — les types de notification (R8)
-- -----------------------------------------------------------------------------
INSERT INTO engagement.notification_types (code, module_code, label, default_channels, criticality, expected_variables) VALUES
    ('negotiation.francophone_meeting.changed', 'negotiation', '{"fr":"Réunion de la Francophonie modifiée","en":"Francophonie meeting changed"}', '{in_app}', 'normal', '{}'),
    ('negotiation.meeting_registration.promoted', 'negotiation', '{"fr":"Place obtenue à une réunion","en":"Place secured at a meeting"}', '{in_app}', 'normal', '{}')
ON CONFLICT (code) DO NOTHING;

COMMIT;
