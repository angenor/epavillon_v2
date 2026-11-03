-- Courriels programmés : textes d'un type et d'un envoi (04/11/2026). Rejouable.
-- Additive, modèle seul (aucune donnée) : une table neuve, rien d'existant ne change.
-- Modèle : docs/database/110_engagement.sql (engagement.scheduled_email_texts).
--
-- L'onglet « Programmés » de Courriels réécrit les rappels déjà en file : le
-- texte d'un type pour l'édition, ou celui d'un seul envoi. Le worker lit ces
-- textes à l'envoi, avant son texte par défaut.
--
-- Jouer AVANT de déployer l'API et le worker : les rappels lisent cette table.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-04-courriels-programmes', true);

CREATE TABLE IF NOT EXISTS engagement.scheduled_email_texts (
    id               uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id         uuid        NOT NULL CONSTRAINT xmod_fk_scheduled_email_texts_event
                                 REFERENCES event.events(id) ON DELETE CASCADE,
    kind             text        NOT NULL,
    offset_minutes   integer,
    session_id       uuid        CONSTRAINT xmod_fk_scheduled_email_texts_session
                                 REFERENCES programme.sessions(id) ON DELETE CASCADE,
    track_id         uuid        CONSTRAINT xmod_fk_scheduled_email_texts_track
                                 REFERENCES event.programme_tracks(id) ON DELETE CASCADE,
    target_starts_at timestamptz,
    locale           text        NOT NULL,
    subject          text        NOT NULL,
    body             text        NOT NULL,
    updated_by       uuid        CONSTRAINT xmod_fk_scheduled_email_texts_author
                                 REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_scheduled_email_texts_kind CHECK (kind IN ('activity', 'day')),
    CONSTRAINT ck_scheduled_email_texts_locale CHECK (locale IN ('fr', 'en')),
    CONSTRAINT ck_scheduled_email_texts_offset CHECK (offset_minutes IS NULL OR offset_minutes > 0),
    CONSTRAINT ck_scheduled_email_texts_target CHECK (
        (kind = 'activity' AND offset_minutes IS NOT NULL
             AND track_id IS NULL AND target_starts_at IS NULL)
        OR (kind = 'day' AND session_id IS NULL
             AND ((track_id IS NULL AND offset_minutes IS NULL AND target_starts_at IS NULL)
               OR (track_id IS NOT NULL AND offset_minutes IS NOT NULL AND target_starts_at IS NOT NULL)))),
    CONSTRAINT ck_scheduled_email_texts_subject CHECK (length(btrim(subject)) BETWEEN 1 AND 250),
    CONSTRAINT ck_scheduled_email_texts_body CHECK (length(btrim(body)) BETWEEN 1 AND 10000),
    CONSTRAINT ux_scheduled_email_texts UNIQUE NULLS NOT DISTINCT
        (event_id, kind, offset_minutes, session_id, track_id, target_starts_at, locale)
);

CREATE INDEX IF NOT EXISTS ix_scheduled_email_texts_session
    ON engagement.scheduled_email_texts (session_id) WHERE session_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_scheduled_email_texts_track
    ON engagement.scheduled_email_texts (track_id) WHERE track_id IS NOT NULL;

DROP TRIGGER IF EXISTS tg_scheduled_email_texts_updated_at ON engagement.scheduled_email_texts;
CREATE TRIGGER tg_scheduled_email_texts_updated_at
    BEFORE UPDATE ON engagement.scheduled_email_texts
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

DROP TRIGGER IF EXISTS tg_scheduled_email_texts_audit ON engagement.scheduled_email_texts;
CREATE TRIGGER tg_scheduled_email_texts_audit
    AFTER INSERT OR UPDATE OR DELETE ON engagement.scheduled_email_texts
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE engagement.scheduled_email_texts IS
    'Textes des courriels programmés (rappels d''activité et de journée) : texte d''un type pour l''édition, ou d''un envoi (cible, décalage, début visé), par langue. Lus à l''envoi, avant le texte par défaut du code.';
COMMENT ON COLUMN engagement.scheduled_email_texts.offset_minutes IS
    'Décalage avant le début, en minutes. Nul pour le texte du type « rappel de journée », qui couvre tous ses décalages.';
COMMENT ON COLUMN engagement.scheduled_email_texts.body IS
    'Texte brut ; une ligne vide sépare deux paragraphes. Les valeurs des variables sont échappées dans le HTML à l''envoi.';

COMMIT;
