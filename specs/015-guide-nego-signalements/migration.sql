-- =============================================================================
-- ePavillon v2 — migration de l'étape 3b de Guide Négo : les signalements du
-- réseau, les réunions non annoncées et les notifications
--
-- Du schéma en service après l'étape 3a (specs/014-guide-nego-sessions-agenda/
-- migration.sql) vers celui de docs/database/ après l'étape 3b.
--
-- POURQUOI CE FICHIER EXISTE
--   `docs/database/` n'est chargé qu'à la création du volume : une base qui
--   porte des comptes se migre, selon le § 13 de docs/DEPLOIEMENT.md. Il vaut
--   AUSSI en local. **`down -v` n'est jamais la procédure.**
--
-- REJOUABLE SANS DÉGÂT
--   Type, tables, index et colonne sous condition ; triggers retirés puis
--   reposés ; fonctions remplacées ; semis sous ON CONFLICT. Les textes —
--   commentaires compris — sont ceux du modèle, MOT POUR MOT.
--
-- ORDRE
--   1. negotiation.theme_subscriptions : le réglage « prévenir des changements »
--   2. negotiation : signalements, réunions non annoncées, leur agenda, et les
--      deux fonctions qui disent qui prévenir
--   3. semis : la permission de valider, les quatre types de notification
--
-- CONTRÔLE APRÈS COUP (§ 13, étape 6)
--   pg_dump --schema-only de cette base, comparé à celui d'une base chargée
--   depuis docs/database/. Comparer après tri des lignes.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- 1. negotiation.theme_subscriptions — prévenir des changements (R11)
-- -----------------------------------------------------------------------------
ALTER TABLE negotiation.theme_subscriptions
    ADD COLUMN IF NOT EXISTS notify_changes boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN negotiation.theme_subscriptions.notify_changes IS
    'Prévenir des changements des sessions de la thématique (Guide Négo, étape 3b). Éteint par défaut : suivre une thématique ordonne ce qu''on montre, pas ce dont on avertit. Quitter la thématique l''éteint avec elle.';

-- -----------------------------------------------------------------------------
-- 2. Signalements et réunions non annoncées — Guide Négo, étape 3b
--
-- Un signalement ne touche JAMAIS la session officielle (ADR-010) : il vit dans
-- sa table et se pose par-dessus, une fois validé ET publié. `published_at` est
-- la seule porte du public : l'encart, la réunion non annoncée, « Validé » chez
-- l'autrice, les avis et les courriels n'existent qu'une fois posé. Entre la
-- validation et la publication, trente secondes où l'annulation gagne toujours
-- (specs/015-guide-nego-signalements, R3).
--
-- Aucun déclencheur d'outbox sur ces tables : les événements naissent du travail
-- de publication et du refus, qui calculent les destinataires par
-- change_recipients() et network_recipients() ci-dessous (R8).
-- -----------------------------------------------------------------------------
DO $$
BEGIN
    CREATE TYPE negotiation.report_status AS ENUM ('submitted', 'validated', 'rejected');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;

CREATE TABLE IF NOT EXISTS negotiation.session_reports (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    author_id           uuid        NOT NULL CONSTRAINT xmod_fk_session_reports_author
                                    REFERENCES identity.people(id) ON DELETE CASCADE,
    client_ref          uuid        NOT NULL,
    event_id            uuid        NOT NULL CONSTRAINT xmod_fk_session_reports_event
                                    REFERENCES event.events(id) ON DELETE RESTRICT,
    meeting_id          uuid        REFERENCES negotiation.meetings(id) ON DELETE CASCADE,
    reason              text        NOT NULL,
    proposed_start      timestamptz,
    proposed_venue      text,
    what                text,
    proposed_day        date,
    theme_term_id       uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    detail              text,
    status              negotiation.report_status NOT NULL DEFAULT 'submitted',
    submitted_at        timestamptz NOT NULL DEFAULT now(),
    decided_by          uuid        CONSTRAINT xmod_fk_session_reports_decider
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    decided_at          timestamptz,
    reject_reason       text,
    reject_detail       text,
    source_snapshot     jsonb,
    published_at        timestamptz,
    withdrawn_at        timestamptz,
    withdrawal          text,
    network_meeting_id  uuid,
    CONSTRAINT ux_session_reports_client_ref UNIQUE (author_id, client_ref),
    CONSTRAINT ck_session_reports_reason
        CHECK (reason IN ('cancelled', 'time', 'venue', 'other', 'unannounced')),
    CONSTRAINT ck_session_reports_target
        CHECK ((meeting_id IS NULL) = (reason = 'unannounced')),
    CONSTRAINT ck_session_reports_unannounced
        CHECK (CASE WHEN reason = 'unannounced'
                    THEN what IS NOT NULL AND proposed_day IS NOT NULL
                    ELSE what IS NULL AND proposed_day IS NULL AND theme_term_id IS NULL
                         AND network_meeting_id IS NULL END),
    CONSTRAINT ck_session_reports_detail CHECK (char_length(detail) <= 600),
    CONSTRAINT ck_session_reports_reject_reason
        CHECK (reject_reason IN ('source_maintains', 'already_known', 'not_precise')),
    CONSTRAINT ck_session_reports_reject_detail CHECK (char_length(reject_detail) <= 600),
    CONSTRAINT ck_session_reports_decision
        CHECK (CASE status
                    WHEN 'submitted' THEN decided_by IS NULL AND decided_at IS NULL
                         AND source_snapshot IS NULL AND published_at IS NULL
                    ELSE decided_at IS NOT NULL END
               AND (status = 'rejected') = (reject_reason IS NOT NULL)
               AND (reject_detail IS NULL OR reject_reason IS NOT NULL)
               AND (published_at IS NULL OR status = 'validated')),
    CONSTRAINT ck_session_reports_withdrawal
        CHECK ((withdrawal IN ('caught_up', 'admin') AND withdrawn_at IS NOT NULL AND status = 'validated')
               OR (withdrawal IS NULL AND withdrawn_at IS NULL))
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_session_reports_pending ON negotiation.session_reports (author_id, meeting_id)
    WHERE status = 'submitted' AND meeting_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_session_reports_queue ON negotiation.session_reports (submitted_at)
    WHERE status = 'submitted';
CREATE INDEX IF NOT EXISTS ix_session_reports_meeting ON negotiation.session_reports (meeting_id)
    WHERE published_at IS NOT NULL AND withdrawn_at IS NULL;

DROP TRIGGER IF EXISTS tg_session_reports_audit ON negotiation.session_reports;
CREATE TRIGGER tg_session_reports_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.session_reports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
DROP TRIGGER IF EXISTS tg_session_reports_check_theme ON negotiation.session_reports;
CREATE TRIGGER tg_session_reports_check_theme
    BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.session_reports
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'theme_term_id', 'negotiation_theme');

COMMENT ON TABLE negotiation.session_reports IS
    'Signalements du réseau sur les sessions officielles, et réunions non annoncées (motif unannounced, sans session). Ne modifie jamais la session : se pose par-dessus, une fois validé et publié (ADR-010). Aucun déclencheur d''outbox : les événements naissent du travail de publication et du refus.';
COMMENT ON COLUMN negotiation.session_reports.client_ref IS
    'Référence posée par le téléphone à la création : un envoi rejoué par la file retrouve la ligne au lieu d''en créer une seconde (ux_session_reports_client_ref).';
COMMENT ON COLUMN negotiation.session_reports.event_id IS
    'La COP du signalement, posée aussi pour une réunion non annoncée, qui n''a pas de session.';
COMMENT ON COLUMN negotiation.session_reports.meeting_id IS
    'Session officielle signalée ; nulle si et seulement si le motif est unannounced (ck_session_reports_target).';
COMMENT ON COLUMN negotiation.session_reports.reason IS
    'Motif, code stable : cancelled (annulée), time (heure changée), venue (salle changée), other, unannounced (réunion non annoncée).';
COMMENT ON COLUMN negotiation.session_reports.proposed_start IS
    'Nouvelle heure de début (motif time), ou début d''une réunion non annoncée — nul = sans heure.';
COMMENT ON COLUMN negotiation.session_reports.proposed_venue IS
    'Nouvelle salle (motif venue), ou « Où » d''une réunion non annoncée.';
COMMENT ON COLUMN negotiation.session_reports.what IS
    '« Quoi » d''une réunion non annoncée ; requis pour ce motif, nul pour les autres (ck_session_reports_unannounced).';
COMMENT ON COLUMN negotiation.session_reports.proposed_day IS
    'Jour de la liste où la réunion non annoncée s''affiche, dans le fuseau de la COP ; requis pour ce motif.';
COMMENT ON COLUMN negotiation.session_reports.theme_term_id IS
    'Thématique (negotiation_theme) d''une réunion non annoncée ; une session signalée tient la sienne de son point de l''ordre du jour.';
COMMENT ON COLUMN negotiation.session_reports.detail IS
    'Précision libre de l''autrice, 600 caractères au plus. Jamais affichée avec son nom.';
COMMENT ON COLUMN negotiation.session_reports.status IS
    'submitted → validated | rejected ; validated → submitted tant que published_at est nul (annulation). « Validé » ne se montre à l''autrice qu''une fois publié.';
COMMENT ON COLUMN negotiation.session_reports.decided_by IS
    'Administratrice ou administrateur qui a validé ou refusé ; remis à nul par l''annulation.';
COMMENT ON COLUMN negotiation.session_reports.reject_reason IS
    'Motif du refus, code stable : source_maintains (la source maintient), already_known (déjà connu), not_precise (pas assez précis). Posé si et seulement si refusé.';
COMMENT ON COLUMN negotiation.session_reports.source_snapshot IS
    'Ce que disait la source officielle au moment de la décision (ADR-010) : en validant, l''administration voit la source à cet instant.';
COMMENT ON COLUMN negotiation.session_reports.published_at IS
    'Seule porte du public, posée par le travail de publication trente secondes après la validation, jamais avant. Tant qu''elle est nulle, rien n''est montré ni envoyé, et l''annulation gagne.';
COMMENT ON COLUMN negotiation.session_reports.withdrawn_at IS
    'Retrait de l''affichage : la source a rattrapé le signalement (caught_up), ou l''administration l''a retiré (admin). La fin de session n''écrit rien, elle se calcule.';
COMMENT ON COLUMN negotiation.session_reports.network_meeting_id IS
    'Réunion non annoncée née de ce signalement, posée par le travail de publication.';

-- La réunion non annoncée n'est PAS une ligne de meetings : chaque requête de 3a
-- filtre les sessions importées par source_key, et un oubli ferait passer une
-- réunion du réseau pour officielle. Deux tables, deux sens (R2).
CREATE TABLE IF NOT EXISTS negotiation.network_meetings (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id        uuid        NOT NULL CONSTRAINT xmod_fk_network_meetings_event
                                REFERENCES event.events(id) ON DELETE RESTRICT,
    report_id       uuid        NOT NULL REFERENCES negotiation.session_reports(id) ON DELETE CASCADE,
    title           text        NOT NULL,
    venue           text,
    start_at        timestamptz,
    day             date        NOT NULL,
    theme_term_id   uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    validated_at    timestamptz NOT NULL,
    withdrawn_at    timestamptz,
    CONSTRAINT ux_network_meetings_report UNIQUE (report_id)
);

CREATE INDEX IF NOT EXISTS ix_network_meetings_event_day ON negotiation.network_meetings (event_id, day)
    WHERE withdrawn_at IS NULL;

DROP TRIGGER IF EXISTS tg_network_meetings_audit ON negotiation.network_meetings;
CREATE TRIGGER tg_network_meetings_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.network_meetings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
DROP TRIGGER IF EXISTS tg_network_meetings_check_theme ON negotiation.network_meetings;
CREATE TRIGGER tg_network_meetings_check_theme
    BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.network_meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'theme_term_id', 'negotiation_theme');

DO $$
BEGIN
    ALTER TABLE negotiation.session_reports ADD CONSTRAINT fk_session_reports_network_meeting
        FOREIGN KEY (network_meeting_id) REFERENCES negotiation.network_meetings(id) ON DELETE SET NULL;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;

COMMENT ON TABLE negotiation.network_meetings IS
    'Réunions non annoncées, signalées par le réseau et validées. Créées par le travail de publication, jamais avant. Aucune colonne de source : ce n''est pas une session officielle, et elle ne se montre jamais comme telle.';
COMMENT ON COLUMN negotiation.network_meetings.title IS
    '« Quoi », tel que signalé.';
COMMENT ON COLUMN negotiation.network_meetings.start_at IS
    'Heure de début ; nulle = sans heure.';
COMMENT ON COLUMN negotiation.network_meetings.day IS
    'Jour de la liste, dans le fuseau de la COP. La réunion quitte l''affichage à la fin de ce jour, sans rien écrire.';
COMMENT ON COLUMN negotiation.network_meetings.withdrawn_at IS
    'Retrait par l''administration.';

-- « Mon agenda » pour une réunion non annoncée : table sœur d'agenda_entries,
-- que 3a garde intacte.
CREATE TABLE IF NOT EXISTS negotiation.network_agenda_entries (
    person_id           uuid        NOT NULL CONSTRAINT xmod_fk_network_agenda_entries_person
                                    REFERENCES identity.people(id) ON DELETE CASCADE,
    network_meeting_id  uuid        NOT NULL REFERENCES negotiation.network_meetings(id) ON DELETE CASCADE,
    remind_before       interval,
    added_at            timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, network_meeting_id),
    CONSTRAINT ck_network_agenda_entries_remind
        CHECK (remind_before IS NULL OR remind_before = interval '15 minutes')
);

CREATE INDEX IF NOT EXISTS ix_network_agenda_entries_meeting ON negotiation.network_agenda_entries (network_meeting_id);

DROP TRIGGER IF EXISTS tg_network_agenda_entries_updated_at ON negotiation.network_agenda_entries;
CREATE TRIGGER tg_network_agenda_entries_updated_at BEFORE UPDATE ON negotiation.network_agenda_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_network_agenda_entries_audit ON negotiation.network_agenda_entries;
CREATE TRIGGER tg_network_agenda_entries_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.network_agenda_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('network_meeting_id');

COMMENT ON TABLE negotiation.network_agenda_entries IS
    'Réunions non annoncées gardées dans « Mon agenda ». Même règle qu''agenda_entries : le retrait supprime la ligne, aucune place engagée.';
COMMENT ON COLUMN negotiation.network_agenda_entries.remind_before IS
    'Rappel dans l''application ouverte : nul = aucun, sinon 15 minutes, seule valeur offerte.';

-- Qui prévenir : une seule règle, lue par negotiation seul — l'import, le
-- travail de publication, le travail de courriel. engagement reçoit la liste
-- dans l'événement et ne lit rien ici (R8).
CREATE OR REPLACE FUNCTION negotiation.change_recipients(p_meeting_id uuid)
RETURNS SETOF uuid
LANGUAGE sql
STABLE
AS $$
    SELECT ae.person_id
      FROM negotiation.agenda_entries ae
     WHERE ae.meeting_id = p_meeting_id
    UNION
    SELECT ts.person_id
      FROM negotiation.meetings m
      JOIN negotiation.agenda_items ai ON ai.id = m.agenda_item_id
      JOIN negotiation.theme_subscriptions ts
        ON ts.theme_term_id = ai.theme_term_id AND ts.left_at IS NULL AND ts.notify_changes
     WHERE m.id = p_meeting_id;
$$;

COMMENT ON FUNCTION negotiation.change_recipients(uuid) IS
    'Personnes à prévenir d''un changement de session : celles qui l''ont dans « Mon agenda », et celles qui suivent sa thématique (celle de son point de l''ordre du jour) avec notify_changes. Sans doublon.';

CREATE OR REPLACE FUNCTION negotiation.network_recipients(p_network_meeting_id uuid)
RETURNS SETOF uuid
LANGUAGE sql
STABLE
AS $$
    SELECT na.person_id
      FROM negotiation.network_agenda_entries na
     WHERE na.network_meeting_id = p_network_meeting_id
    UNION
    SELECT ts.person_id
      FROM negotiation.network_meetings nm
      JOIN negotiation.theme_subscriptions ts
        ON ts.theme_term_id = nm.theme_term_id AND ts.left_at IS NULL AND ts.notify_changes
     WHERE nm.id = p_network_meeting_id;
$$;

COMMENT ON FUNCTION negotiation.network_recipients(uuid) IS
    'Personnes à prévenir d''une réunion non annoncée : celles qui l''ont dans « Mon agenda », et celles qui suivent sa thématique avec notify_changes. Sans doublon.';

-- -----------------------------------------------------------------------------
-- 3. Semis — la permission de valider (R6) et les types de notification (R8)
-- -----------------------------------------------------------------------------
INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('negotiation.report.validate', '{"fr":"Valider les signalements du réseau","en":"Validate network reports"}',    'negotiation')
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin',      'negotiation.report.validate')
ON CONFLICT DO NOTHING;

INSERT INTO engagement.notification_types (code, module_code, label, default_channels, criticality, expected_variables) VALUES
    ('negotiation.meeting.changed',     'negotiation', '{"fr":"Session de négociation modifiée","en":"Negotiation session changed"}', '{in_app}',   'normal',    '{}'),
    ('negotiation.report.published',    'negotiation', '{"fr":"Changement signalé par le réseau","en":"Change reported by the network"}', '{in_app}', 'normal',  '{}'),
    ('negotiation.network_meeting.published', 'negotiation', '{"fr":"Réunion non annoncée","en":"Unannounced meeting"}',    '{in_app}',    'normal',    '{}'),
    ('negotiation.report.decided',      'negotiation', '{"fr":"Décision sur votre signalement","en":"Decision on your report"}', '{in_app}',     'normal',    '{}')
ON CONFLICT (code) DO NOTHING;

COMMIT;
