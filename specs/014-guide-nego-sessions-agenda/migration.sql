-- =============================================================================
-- ePavillon v2 — migration de l'étape 3a de Guide Négo : les sessions de
-- négociation, leur import et « Mon agenda »
--
-- Du schéma en service après l'étape 1b (specs/012-guide-nego-lecteur-pdf/
-- migration.sql) vers celui de docs/database/ après l'étape 3a.
--
-- POURQUOI CE FICHIER EXISTE
--   `docs/database/` n'est chargé qu'à la création du volume : une base qui
--   porte des comptes se migre, selon le § 13 de docs/DEPLOIEMENT.md. Il vaut
--   AUSSI en local. **`down -v` n'est jamais la procédure.**
--
-- REJOUABLE SANS DÉGÂT
--   Tables, index et colonnes sous IF NOT EXISTS ; triggers retirés puis
--   reposés ; les CHECK retouchés de negotiation.meetings retirés puis reposés,
--   à l'identique. Les textes — commentaires compris — sont ceux du modèle,
--   MOT POUR MOT : un mot qui diffère ressortirait à la comparaison.
--
-- ORDRE
--   1. reference : les vocabulaires des types de réunion et des groupes
--   2. negotiation : les points de l'ordre du jour (les réunions y renvoient)
--   3. negotiation.meetings : colonnes de la source, contraintes retouchées
--   4. negotiation : l'import, son journal, les écarts, les traductions,
--      « Mon groupe » et « Mon agenda »
--   5. semis : le modèle de rédaction, l'import de la COP31 éteint
--
-- CONTRÔLE APRÈS COUP (§ 13, étape 6)
--   pg_dump --schema-only de cette base, comparé à celui d'une base chargée
--   depuis docs/database/. Comparer après tri des lignes.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- 1. reference — les vocabulaires des sessions de négociation
--
-- Deux vocabulaires lus par l'import de la source officielle
-- (specs/014-guide-nego-sessions-agenda, R4). Le tri et le rattachement des
-- réunions de la CCNUCC se font par leur `metadata`, jamais par une liste
-- écrite dans le code :
--
--   negotiation_meeting_type
--     source_categories    catégories de la source (`typeofevent`, lues dans le
--                          calendrier de la COP30) qui admettent ce type. Une
--                          réunion dont la catégorie n'est admise par aucun type
--                          est écartée : événements parallèles, conférences de
--                          presse, la plupart des événements de la présidence.
--     denominations        fragments de titre qui désignent ce type.
--     requires_title_match le type n'admet que les titres qui le nomment, même
--                          dans une catégorie qui l'accepte.
--     default_for          catégories dont c'est le type quand aucun titre ne
--                          nomme un autre type.
--   negotiation_group
--     denominations        noms et sigles sous lesquels la source nomme le
--                          groupe, relevés dans les titres réels (COP29, COP30).
--
-- Chaînes comparées après normalisation — minuscules, sans accents ni
-- ponctuation — et EN MOTS ENTIERS : « EIG » est dans « Sovereign » et « LDC »
-- dans « LLDCs », qui ne sont ni l'un ni l'autre. L'ordre de `sort_order` est
-- aussi l'ordre de résolution des types : le plus précis d'abord (« informal
-- informal consultations » contient « informal consultations »).
--
-- `label.en` des types est le terme passé au lexique. Libellés fr/en : des
-- données, jamais un fichier i18n. is_system : aucun écran ne les modifie.
-- -----------------------------------------------------------------------------
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('negotiation_meeting_type', '{"fr":"Types de réunion de négociation","en":"Negotiation meeting types"}', '{"fr":"Nature d''une session officielle : plénière, groupe de contact, consultations informelles…","en":"Nature of an official session: plenary, contact group, informal consultations…"}', false, false, true),
    ('negotiation_group',        '{"fr":"Groupes de négociation","en":"Negotiating groups"}',             '{"fr":"Groupes de Parties qui se coordonnent pendant une COP : Groupe africain, PMA, G77 et Chine…","en":"Party groupings that coordinate during a COP: African Group, LDCs, G77 and China…"}', true, false, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, description, sort_order, metadata) VALUES
    ('negotiation_meeting_type', 'plenary', '{"fr":"Plénière","en":"Plenary"}', NULL, 10,
     '{"source_categories":["Plenary"],"denominations":["plenary","plenaries"],"requires_title_match":false,"default_for":["Plenary"]}'),
    ('negotiation_meeting_type', 'heads_of_delegation', '{"fr":"Chefs de délégation","en":"Heads of delegation"}', NULL, 20,
     '{"source_categories":["Negotiations"],"denominations":["hods","hod","heads of delegation","head of delegation"],"requires_title_match":false,"default_for":[]}'),
    ('negotiation_meeting_type', 'presidency_consultation', '{"fr":"Consultation de la présidence","en":"Presidency consultation"}', NULL, 30,
     '{"source_categories":["Negotiations","Presidency event"],"denominations":["presidency consultation","presidency consultations"],"requires_title_match":true,"default_for":[]}'),
    ('negotiation_meeting_type', 'informal_informals', '{"fr":"Aparté","en":"Informal informals"}', NULL, 40,
     '{"source_categories":["Negotiations"],"denominations":["informal informal","informal informals","informal-informal","informal-informals"],"requires_title_match":false,"default_for":[]}'),
    ('negotiation_meeting_type', 'informal_consultations', '{"fr":"Consultations informelles","en":"Informal consultations"}', NULL, 50,
     '{"source_categories":["Negotiations"],"denominations":["informal consultation","informal consultations"],"requires_title_match":false,"default_for":[]}'),
    ('negotiation_meeting_type', 'contact_group', '{"fr":"Groupe de contact","en":"Contact group"}', NULL, 60,
     '{"source_categories":["Negotiations"],"denominations":["contact group","contact groups"],"requires_title_match":false,"default_for":[]}'),
    ('negotiation_meeting_type', 'mandated_event', '{"fr":"Événement mandaté","en":"Mandated event"}', NULL, 70,
     '{"source_categories":["Mandated events"],"denominations":["mandated event","mandated events"],"requires_title_match":false,"default_for":["Mandated events"]}'),
    ('negotiation_meeting_type', 'group_coordination', '{"fr":"Coordination de groupe","en":"Group coordination"}', NULL, 80,
     '{"source_categories":["Coordination meetings"],"denominations":[],"requires_title_match":false,"default_for":["Coordination meetings"]}'),
    ('negotiation_meeting_type', 'negotiation_other', '{"fr":"Autre réunion de négociation","en":"Other negotiation meeting"}', NULL, 90,
     '{"source_categories":["Negotiations"],"denominations":[],"requires_title_match":false,"default_for":["Negotiations"]}'),

    ('negotiation_group', 'african_group', '{"fr":"Groupe africain","en":"African Group"}', '{"fr":"Groupe africain des négociateurs (AGN)","en":"African Group of Negotiators (AGN)"}', 10,
     '{"denominations":["African Group","African Group of Negotiators","AGN"]}'),
    ('negotiation_group', 'ldc', '{"fr":"PMA","en":"LDCs"}', '{"fr":"Groupe des pays les moins avancés","en":"Least Developed Countries Group"}', 20,
     '{"denominations":["LDC","LDCs","Least Developed Countries"]}'),
    ('negotiation_group', 'g77_china', '{"fr":"G77 et Chine","en":"G77 and China"}', '{"fr":"Groupe des 77 et la Chine","en":"Group of 77 and China"}', 30,
     '{"denominations":["G77 & China","G77 and China","G77","Group of 77"]}'),
    ('negotiation_group', 'aosis', '{"fr":"AOSIS","en":"AOSIS"}', '{"fr":"Alliance des petits États insulaires","en":"Alliance of Small Island States"}', 40,
     '{"denominations":["AOSIS","Alliance of Small Island States","SIDS","SIDs","Small Island Developing States"]}'),
    ('negotiation_group', 'arab_group', '{"fr":"Groupe arabe","en":"Arab Group"}', NULL, 50,
     '{"denominations":["Arab Group","Arab Group of Negotiators"]}'),
    ('negotiation_group', 'lmdc', '{"fr":"LMDC","en":"LMDC"}', '{"fr":"Pays en développement animés du même esprit","en":"Like-Minded Developing Countries"}', 60,
     '{"denominations":["LMDC","LMDCs","Like-Minded Developing Countries"]}'),
    ('negotiation_group', 'ailac', '{"fr":"AILAC","en":"AILAC"}', '{"fr":"Association indépendante de l''Amérique latine et des Caraïbes","en":"Independent Association of Latin America and the Caribbean"}', 70,
     '{"denominations":["AILAC"]}'),
    ('negotiation_group', 'eu', '{"fr":"Union européenne","en":"European Union"}', NULL, 80,
     '{"denominations":["EU","European Union"]}'),
    ('negotiation_group', 'eig', '{"fr":"GIE","en":"EIG"}', '{"fr":"Groupe de l''intégrité environnementale","en":"Environmental Integrity Group"}', 90,
     '{"denominations":["EIG","Environmental Integrity Group"]}'),
    ('negotiation_group', 'basic', '{"fr":"BASIC","en":"BASIC"}', '{"fr":"Brésil, Afrique du Sud, Inde et Chine","en":"Brazil, South Africa, India and China"}', 100,
     '{"denominations":["BASIC"]}'),
    ('negotiation_group', 'umbrella_group', '{"fr":"Groupe de l''Ombrelle","en":"Umbrella Group"}', NULL, 110,
     '{"denominations":["Umbrella Group"]}'),
    ('negotiation_group', 'alba', '{"fr":"ALBA","en":"ALBA"}', '{"fr":"Alliance bolivarienne pour les peuples de notre Amérique","en":"Bolivarian Alliance for the Peoples of Our America"}', 120,
     '{"denominations":["ALBA"]}'),
    ('negotiation_group', 'grulac', '{"fr":"GRULAC","en":"GRULAC"}', '{"fr":"Groupe des États d''Amérique latine et des Caraïbes","en":"Group of Latin American and Caribbean States"}', 130,
     '{"denominations":["GRULAC"]}')
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 2. negotiation — Les points de l'ordre du jour d'une COP (Guide Négo, étape 3a)
--
-- La source officielle ne les publie pas à part : l'import les extrait du titre
-- des sessions (« SBI 12 (a) … - Informal consultation ») et les crée à la
-- première lecture qui les cite. Leur rattachement à une thématique est un
-- travail de l'IFDD, au back-office : un point n'est donc jamais supprimé par
-- l'import, même quand plus aucune session ne le cite.
-- -----------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS negotiation.agenda_items (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id        uuid        NOT NULL CONSTRAINT xmod_fk_agenda_items_event
                                REFERENCES event.events(id) ON DELETE RESTRICT,
    code            text        NOT NULL,
    title           text        NOT NULL,
    theme_term_id   uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    theme_set_by    uuid        CONSTRAINT xmod_fk_agenda_items_theme_setter
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    theme_set_at    timestamptz,
    first_read_at   timestamptz NOT NULL,
    CONSTRAINT ux_agenda_items_code UNIQUE (event_id, code)
);

CREATE INDEX IF NOT EXISTS ix_agenda_items_theme ON negotiation.agenda_items (theme_term_id) WHERE theme_term_id IS NOT NULL;

DROP TRIGGER IF EXISTS tg_agenda_items_audit ON negotiation.agenda_items;
CREATE TRIGGER tg_agenda_items_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.agenda_items
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
DROP TRIGGER IF EXISTS tg_agenda_items_check_theme ON negotiation.agenda_items;
CREATE TRIGGER tg_agenda_items_check_theme
    BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.agenda_items
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'theme_term_id', 'negotiation_theme');

COMMENT ON TABLE negotiation.agenda_items IS
    'Points de l''ordre du jour d''une COP, extraits du titre des sessions par l''import. Jamais supprimés par lui : leur thématique est posée à la main par l''IFDD.';
COMMENT ON COLUMN negotiation.agenda_items.code IS
    'Code tel que lu dans le titre de la session : « SBI 12 (a) », « CMA 8 ». Unique par édition.';
COMMENT ON COLUMN negotiation.agenda_items.title IS
    'Intitulé anglais, extrait du titre de la première session qui cite le point.';
COMMENT ON COLUMN negotiation.agenda_items.theme_term_id IS
    'Thématique (negotiation_theme), posée au back-office. Les sessions du point en héritent : c''est ce qui les montre en premier à qui suit la thématique.';

-- -----------------------------------------------------------------------------
-- 3. negotiation.meetings — la session importée
-- -----------------------------------------------------------------------------

ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS source_key            text;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS source_url            platform.url;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS title_original        text;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS first_read_at         timestamptz;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS last_read_at          timestamptz;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS meeting_type_term_id  uuid
    REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS group_term_id         uuid
    REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS agenda_item_id        uuid
    REFERENCES negotiation.agenda_items(id) ON DELETE RESTRICT;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS is_open_access        boolean;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS absent_reads          smallint NOT NULL DEFAULT 0;
ALTER TABLE negotiation.meetings ADD COLUMN IF NOT EXISTS cancelled_at          timestamptz;

ALTER TABLE negotiation.meetings ALTER COLUMN end_at DROP NOT NULL;

-- Un CHECK retouché se retire et se repose : rejoué, il revient identique.
ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_period;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_period
    CHECK (end_at IS NULL OR end_at > start_at);
ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_imported_end;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_imported_end
    CHECK (source_key IS NOT NULL OR end_at IS NOT NULL);
ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_source_complete;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_source_complete
    CHECK (source_key IS NULL OR (event_id IS NOT NULL AND source_url IS NOT NULL
        AND title_original IS NOT NULL AND first_read_at IS NOT NULL AND last_read_at IS NOT NULL));
ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_onsite_venue;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_onsite_venue
    CHECK (status = 'draft' OR format = 'online' OR venue_label IS NOT NULL OR source_key IS NOT NULL);
ALTER TABLE negotiation.meetings DROP CONSTRAINT IF EXISTS ck_meetings_import_cancellation;
ALTER TABLE negotiation.meetings ADD CONSTRAINT ck_meetings_import_cancellation
    CHECK (source_key IS NULL OR cancellation_reason IS NULL
        OR cancellation_reason IN ('source', 'postponed', 'removed'));

CREATE UNIQUE INDEX IF NOT EXISTS ux_meetings_source ON negotiation.meetings (event_id, source_key)
    WHERE source_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_meetings_event_day ON negotiation.meetings (event_id, start_at)
    WHERE kind = 'negotiation_session';

DROP TRIGGER IF EXISTS tg_meetings_check_type ON negotiation.meetings;
CREATE TRIGGER tg_meetings_check_type
    BEFORE INSERT OR UPDATE OF meeting_type_term_id ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'meeting_type_term_id', 'negotiation_meeting_type');
DROP TRIGGER IF EXISTS tg_meetings_check_group ON negotiation.meetings;
CREATE TRIGGER tg_meetings_check_group
    BEFORE INSERT OR UPDATE OF group_term_id ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'group_term_id', 'negotiation_group');

COMMENT ON COLUMN negotiation.meetings.end_at IS
    'Nulle seulement pour une session importée sans fin annoncée à la source ; toute autre réunion en a une (ck_meetings_imported_end).';
COMMENT ON COLUMN negotiation.meetings.source_key IS
    'Identifiant de la session à la source officielle ; nul pour une réunion saisie à la main. Unique par édition (ux_meetings_source).';
COMMENT ON COLUMN negotiation.meetings.source_url IS
    'La fiche de la session à la source : « Voir l''original ».';
COMMENT ON COLUMN negotiation.meetings.title_original IS
    'Titre anglais tel que lu, préfixe « CANCELLED » ou « POSTPONED » retiré : il FAIT FOI. `title` en reçoit la copie sous "fr" et "en" — le domaine i18n_text exige "fr", et aucun texte humain ne le traduit ; la traduction automatique vit dans title_translations.';
COMMENT ON COLUMN negotiation.meetings.last_read_at IS
    'Dernière lecture réussie où la session figurait à la source.';
COMMENT ON COLUMN negotiation.meetings.meeting_type_term_id IS
    'Type de réunion (negotiation_meeting_type), résolu à l''import par les metadata du vocabulaire.';
COMMENT ON COLUMN negotiation.meetings.group_term_id IS
    'Groupe de négociation (negotiation_group) d''une coordination, re-résolu à chaque lecture ; nul si le titre n''en nomme aucun.';
COMMENT ON COLUMN negotiation.meetings.agenda_item_id IS
    'Point de l''ordre du jour ; nul = « Hors ordre du jour officiel ». La session hérite de sa thématique.';
COMMENT ON COLUMN negotiation.meetings.is_open_access IS
    'Ouverte (vrai) ou à accès limité (faux), selon la source.';
COMMENT ON COLUMN negotiation.meetings.absent_reads IS
    'Lectures réussies de suite où la session manquait à la source. À 2, elle passe cancelled (motif removed) ; reparue, le compteur retombe à 0.';
COMMENT ON COLUMN negotiation.meetings.cancelled_at IS
    'Heure du constat de l''annulation par l''import.';

CREATE OR REPLACE FUNCTION negotiation.tg_meeting_status_event()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    -- Une session importée n'émet rien : ses « inscrits » n'existent pas, et
    -- l'avis de changement viendra des signalements (étape 3b).
    IF NEW.source_key IS NOT NULL THEN
        RETURN NULL;
    END IF;
    IF NEW.status IS DISTINCT FROM OLD.status AND NEW.status IN ('scheduled', 'cancelled') THEN
        PERFORM platform.emit_event(
            'negotiation', 'meeting', NEW.id,
            'negotiation.meeting.' || CASE WHEN NEW.status = 'cancelled' THEN 'cancelled' ELSE 'published' END,
            jsonb_build_object('space_id', NEW.space_id, 'kind', NEW.kind,
                               'start_at', NEW.start_at, 'reason', NEW.cancellation_reason)
        );
    END IF;
    RETURN NULL;
END;
$$;

-- -----------------------------------------------------------------------------
-- 4 bis. L'import de la source officielle — Guide Négo, étape 3a
--
-- Les sessions de négociation d'une COP ne se saisissent pas : un travail lit le
-- calendrier de conférence de la CCNUCC, compare à ce qu'il a déjà lu et
-- n'écrit que les écarts (specs/014-guide-nego-sessions-agenda, R2, R5, R7).
-- La source fait foi ; ce qui en vient porte son origine et son heure de
-- lecture, dans negotiation.meetings.
-- -----------------------------------------------------------------------------

-- L'import d'une édition : son interrupteur, son lecteur et son état de santé.
CREATE TABLE IF NOT EXISTS negotiation.official_imports (
    id                      uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id                uuid        NOT NULL CONSTRAINT xmod_fk_official_imports_event
                                        REFERENCES event.events(id) ON DELETE RESTRICT,
    is_enabled              boolean     NOT NULL DEFAULT false,
    reader                  text        NOT NULL DEFAULT 'archive',
    archive_name            text,
    archive_first_day       date,
    live_url                platform.url,
    time_correction_minutes smallint    NOT NULL DEFAULT 60,
    official_programme_url  platform.url NOT NULL,
    interval_seconds        integer     NOT NULL DEFAULT 300,
    missed_threshold        smallint    NOT NULL DEFAULT 3,
    missed_reads            smallint    NOT NULL DEFAULT 0,
    last_success_at         timestamptz,
    last_attempt_at         timestamptz,
    last_error              text,
    last_change_count       integer,
    failing_since           timestamptz,
    updated_by              uuid        CONSTRAINT xmod_fk_official_imports_updater
                                        REFERENCES identity.people(id) ON DELETE SET NULL,
    updated_at              timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_official_imports_event UNIQUE (event_id),
    CONSTRAINT ck_official_imports_reader    CHECK (reader IN ('archive', 'live')),
    CONSTRAINT ck_official_imports_interval  CHECK (interval_seconds >= 60),
    CONSTRAINT ck_official_imports_threshold CHECK (missed_threshold >= 1),
    CONSTRAINT ck_official_imports_missed    CHECK (missed_reads >= 0)
);

DROP TRIGGER IF EXISTS tg_official_imports_updated_at ON negotiation.official_imports;
CREATE TRIGGER tg_official_imports_updated_at BEFORE UPDATE ON negotiation.official_imports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_official_imports_audit ON negotiation.official_imports;
CREATE TRIGGER tg_official_imports_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.official_imports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.official_imports IS
    'L''import des sessions officielles d''une édition : interrupteur, lecteur, cadence, et santé de la source. Une ligne par édition.';
COMMENT ON COLUMN negotiation.official_imports.reader IS
    'archive = un jeu archivé embarqué (recette, et tant que l''accès à la source manque) ; live = le JSON du calendrier de conférence, par HTTP.';
COMMENT ON COLUMN negotiation.official_imports.archive_name IS
    'Le fichier du jeu archivé que lit le lecteur archive.';
COMMENT ON COLUMN negotiation.official_imports.archive_first_day IS
    'Pose le premier jour de l''archive sur ce jour, heure murale gardée, dans le fuseau de l''édition : l''archive se rejoue sur une autre COP, ou sur aujourd''hui pour la recette.';
COMMENT ON COLUMN negotiation.official_imports.time_correction_minutes IS
    'Minutes ajoutées aux heures naïves de la source avant de les lire dans le fuseau de l''édition : la CCNUCC publie l''heure locale moins une heure (constaté sur la COP29 et la COP30).';
COMMENT ON COLUMN negotiation.official_imports.official_programme_url IS
    'Le renvoi « Programme officiel de la CCNUCC », offert quand l''affichage est coupé.';
COMMENT ON COLUMN negotiation.official_imports.missed_threshold IS
    'Lectures manquées d''affilée au-delà desquelles l''affichage se coupe (negotiation.import_is_serving).';
COMMENT ON COLUMN negotiation.official_imports.missed_reads IS
    'Lectures manquées d''affilée : source injoignable, délai dépassé, contenu illisible, ou aucune réunion retenue. Retombe à zéro à la première réussite.';
COMMENT ON COLUMN negotiation.official_imports.failing_since IS
    'Heure de la première lecture manquée de la série en cours : « la source n''a pas répondu depuis 06:40 ». Nulle dès qu''une lecture réussit.';
COMMENT ON COLUMN negotiation.official_imports.last_change_count IS
    'Sessions touchées par la dernière lecture réussie.';

-- Le journal des lectures, purgé au-delà de 30 jours par le travail d'import.
CREATE TABLE IF NOT EXISTS negotiation.import_runs (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    import_id       uuid        NOT NULL REFERENCES negotiation.official_imports(id) ON DELETE CASCADE,
    started_at      timestamptz NOT NULL DEFAULT now(),
    finished_at     timestamptz,
    outcome         text        NOT NULL,
    error           text,
    session_count   integer,
    change_count    integer,
    is_manual       boolean     NOT NULL DEFAULT false,
    CONSTRAINT ck_import_runs_outcome CHECK (outcome IN ('success', 'failure'))
);

CREATE INDEX IF NOT EXISTS ix_import_runs_import ON negotiation.import_runs (import_id, started_at DESC);

COMMENT ON TABLE negotiation.import_runs IS
    'Journal des lectures de la source officielle, une ligne par lecture. Purgé au-delà de 30 jours par le travail d''import ; pas d''audit, c''est déjà un journal.';
COMMENT ON COLUMN negotiation.import_runs.change_count IS
    'Sessions touchées par la lecture — apparues, changées, absentes, reparues —, et non lignes de meeting_changes.';
COMMENT ON COLUMN negotiation.import_runs.is_manual IS
    'Lecture demandée par « Lire maintenant » : elle ne replanifie pas la chaîne.';

-- L'historique des écarts constatés entre deux lectures.
CREATE TABLE IF NOT EXISTS negotiation.meeting_changes (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    meeting_id      uuid        NOT NULL REFERENCES negotiation.meetings(id) ON DELETE CASCADE,
    field           text        NOT NULL,
    old_value       jsonb,
    new_value       jsonb,
    detected_at     timestamptz NOT NULL,
    import_run_id   uuid        REFERENCES negotiation.import_runs(id) ON DELETE SET NULL,
    CONSTRAINT ck_meeting_changes_field
        CHECK (field IN ('start', 'end', 'venue', 'title', 'type', 'access', 'agenda_item', 'status'))
);

CREATE INDEX IF NOT EXISTS ix_meeting_changes_meeting ON negotiation.meeting_changes (meeting_id, detected_at DESC);

COMMENT ON TABLE negotiation.meeting_changes IS
    'Écarts constatés par l''import, champ par champ. « Déplacée » s''en déduit (un changement start, end ou venue) : aucun état stocké ne se désynchronise de l''heure. Pas d''audit, c''est déjà un journal.';
COMMENT ON COLUMN negotiation.meeting_changes.field IS
    'Liste close du code de l''import, pas un vocabulaire : un champ de plus est une évolution du comparateur.';
COMMENT ON COLUMN negotiation.meeting_changes.detected_at IS
    'Heure de la lecture qui a vu l''écart.';
COMMENT ON COLUMN negotiation.meeting_changes.import_run_id IS
    'La lecture qui a vu l''écart. Mise à nul quand le journal est purgé : l''écart, lui, reste.';

-- Une traduction par titre anglais, partagée par les sessions de même titre.
CREATE TABLE IF NOT EXISTS negotiation.title_translations (
    source_text     text        PRIMARY KEY,
    text_fr         text        NOT NULL,
    model           text        NOT NULL,
    translated_at   timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE negotiation.title_translations IS
    'Traduction automatique des titres de sessions officielles, une par titre anglais. ÉCART AU PRINCIPE XII, décidé par le commanditaire le 25/09 : publiée SANS relecture humaine, bornée aux titres de sessions officielles (une COP en compte des centaines par jour), toujours affichée sous le titre anglais — qui fait foi — et marquée « Traduction automatique ». Pas d''audit : c''est le travail qui écrit, et le modèle est noté.';
COMMENT ON COLUMN negotiation.title_translations.source_text IS
    'Le titre anglais exact, tel que gardé dans meetings.title_original : un titre changé à la source appelle une nouvelle traduction.';
COMMENT ON COLUMN negotiation.title_translations.model IS
    'Le modèle d''OpenRouter qui l''a produite (réglage ai.drafting_model au moment de la traduction).';

-- La règle de coupure, en un seul endroit : la route publique et le back-office
-- la lisent, aucun code ne la réécrit.
CREATE OR REPLACE FUNCTION negotiation.import_is_serving(p_event_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT coalesce((
        SELECT i.is_enabled
           AND i.last_success_at IS NOT NULL
           AND i.missed_reads < i.missed_threshold
           AND now() - i.last_success_at
               < make_interval(secs => i.missed_threshold * i.interval_seconds + 60)
          FROM negotiation.official_imports i
         WHERE i.event_id = p_event_id
    ), false);
$$;

COMMENT ON FUNCTION negotiation.import_is_serving(uuid) IS
    'Vrai si les sessions importées de l''édition peuvent s''afficher : import allumé, au moins une lecture réussie, moins de missed_threshold lectures manquées d''affilée, ET dernière réussite plus récente que missed_threshold × interval_seconds + 60 s — un worker arrêté ne manque aucune lecture, et ne doit pas laisser servir une liste périmée. Faux sans ligne d''import.';

-- -----------------------------------------------------------------------------
-- 4 ter. « Mon groupe » et « Mon agenda » — Guide Négo, étape 3a
-- -----------------------------------------------------------------------------

-- Les groupes de négociation suivis : même patron que theme_subscriptions.
-- Aucun groupe suivi : toutes les coordinations passent le filtre.
CREATE TABLE IF NOT EXISTS negotiation.group_subscriptions (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_group_subscriptions_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    group_term_id   uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    followed_at     timestamptz NOT NULL DEFAULT now(),
    left_at         timestamptz,
    CONSTRAINT ck_group_subscriptions_period CHECK (left_at IS NULL OR left_at >= followed_at)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_group_subscriptions_active
    ON negotiation.group_subscriptions (person_id, group_term_id) WHERE left_at IS NULL;
CREATE INDEX IF NOT EXISTS ix_group_subscriptions_group
    ON negotiation.group_subscriptions (group_term_id, followed_at DESC) WHERE left_at IS NULL;

DROP TRIGGER IF EXISTS tg_group_subscriptions_audit ON negotiation.group_subscriptions;
CREATE TRIGGER tg_group_subscriptions_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.group_subscriptions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
DROP TRIGGER IF EXISTS tg_group_subscriptions_check_group ON negotiation.group_subscriptions;
CREATE TRIGGER tg_group_subscriptions_check_group
    BEFORE INSERT OR UPDATE OF group_term_id ON negotiation.group_subscriptions
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'group_term_id', 'negotiation_group');

COMMENT ON TABLE negotiation.group_subscriptions IS
    'Groupes de négociation qu''une personne suit (« Mon groupe »). N''ouvre aucun droit : commande les coordinations qu''on lui montre. Même patron que theme_subscriptions.';
COMMENT ON COLUMN negotiation.group_subscriptions.left_at IS
    'Un suivi se ferme, il ne se supprime pas ; l''index unique ne porte que sur le suivi vivant.';

-- « Mon agenda » : les sessions officielles qu'une personne garde. Pas une
-- inscription (meeting_registrations) : suivre une session officielle n'engage
-- aucune place, sans capacité ni liste d'attente. Aucune contrainte de
-- chevauchement : deux sessions à la même heure se gardent (règle n° 2).
CREATE TABLE IF NOT EXISTS negotiation.agenda_entries (
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_agenda_entries_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    meeting_id      uuid        NOT NULL REFERENCES negotiation.meetings(id) ON DELETE CASCADE,
    remind_before   interval,
    added_at        timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, meeting_id),
    CONSTRAINT ck_agenda_entries_remind
        CHECK (remind_before IS NULL OR remind_before = interval '15 minutes')
);

CREATE INDEX IF NOT EXISTS ix_agenda_entries_meeting ON negotiation.agenda_entries (meeting_id);

DROP TRIGGER IF EXISTS tg_agenda_entries_updated_at ON negotiation.agenda_entries;
CREATE TRIGGER tg_agenda_entries_updated_at BEFORE UPDATE ON negotiation.agenda_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_agenda_entries_audit ON negotiation.agenda_entries;
CREATE TRIGGER tg_agenda_entries_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.agenda_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('meeting_id');

COMMENT ON TABLE negotiation.agenda_entries IS
    'Sessions officielles gardées dans « Mon agenda ». Le retrait supprime la ligne. Pas une inscription : aucune place engagée, aucun refus pour chevauchement.';
COMMENT ON COLUMN negotiation.agenda_entries.remind_before IS
    'Rappel dans l''application ouverte : nul = aucun, sinon 15 minutes, seule valeur offerte. Sans effet sur une session annulée.';

-- -----------------------------------------------------------------------------
-- 6 ter. Guide Négo — le modèle de rédaction et l'import de la COP31 (étape 3a)
--
-- Le modèle qui rédige est un réglage (ADR-005) : le changer tient en une ligne,
-- sans redéploiement. Il est lu à chaque traduction de titres.
--
-- L'import de la COP31 est semé ÉTEINT, sur le lecteur archivé : l'allumer est
-- un geste du back-office. Sur une base sans cette édition, rien n'est posé.
-- -----------------------------------------------------------------------------
INSERT INTO platform.settings (key, value, description, is_secret) VALUES
    ('ai.drafting_model', '"google/gemini-2.5-flash"',
     'Modèle d''OpenRouter qui rédige — traduction des titres de sessions officielles, et plus tard l''assistant (ADR-005). La clé, elle, vit dans l''environnement (OPENROUTER_API_KEY), jamais ici.', false)
ON CONFLICT (key) DO NOTHING;

INSERT INTO negotiation.official_imports (event_id, is_enabled, reader, official_programme_url)
SELECT e.id, false, 'archive', 'https://unfccc.int/cop31/schedule'
  FROM event.events e
 WHERE e.slug = 'cop31'
ON CONFLICT (event_id) DO NOTHING;

COMMIT;
