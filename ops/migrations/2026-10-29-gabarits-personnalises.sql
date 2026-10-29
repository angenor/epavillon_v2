-- Gabarits de courriel personnalisés (29/10). Rejouable.
-- Modèle : docs/database/110_engagement.sql § 12 (engagement.mail_templates,
-- engagement.mailings.mail_template_id, engagement.mailing_suggested_template(),
-- lignes de media.attachable_roles) et docs/database/050_media.sql (rôle
-- `background` de media.attachment_role).
--
-- Après 2026-10-29-logo-edition-et-gabarits. Déployer avec l'API et le worker :
-- l'API sert l'écran des gabarits, le worker fabrique la variante `courriel`
-- (6:1, 1200 x 200, JPEG compressé) des bandeaux d'édition déjà servis, mis en
-- file en fin de migration.

-- Hors transaction : une valeur ajoutée à une énumération ne s'emploie pas
-- dans la transaction qui l'ajoute.
ALTER TYPE media.attachment_role ADD VALUE IF NOT EXISTS 'background';

BEGIN;

CREATE TABLE IF NOT EXISTS engagement.mail_templates (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    name            text        NOT NULL,
    layout          text        NOT NULL,
    title           text,
    subtitle        text,
    -- Le voile sombre du bas vers le haut sous le texte posé sur l'image.
    gradient        boolean     NOT NULL DEFAULT true,
    -- Nul : commun à toutes les éditions (permission globale pour l'écrire).
    event_id        uuid        CONSTRAINT xmod_fk_mail_templates_event
                                REFERENCES event.events(id) ON DELETE RESTRICT,
    preselect_for_submitters boolean NOT NULL DEFAULT false,
    created_by      uuid        CONSTRAINT xmod_fk_mail_templates_author
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_mail_templates_name CHECK (length(btrim(name)) BETWEEN 1 AND 120),
    CONSTRAINT ck_mail_templates_layout CHECK (layout IN ('sobre', 'institutionnel', 'annonce',
                                                          'banniere', 'texte_sur_image',
                                                          'institutionnel_logo', 'edition')),
    CONSTRAINT ck_mail_templates_texts CHECK (coalesce(length(title), 0) <= 200
                                              AND coalesce(length(subtitle), 0) <= 200),
    -- Présélectionner « pour les déposants de cet événement » suppose un événement.
    CONSTRAINT ck_mail_templates_preselect CHECK (NOT preselect_for_submitters OR event_id IS NOT NULL)
);

-- Un seul gabarit présélectionné par édition : deux se disputeraient l'envoi.
CREATE UNIQUE INDEX IF NOT EXISTS ux_mail_templates_preselect
    ON engagement.mail_templates (event_id) WHERE preselect_for_submitters;
CREATE INDEX IF NOT EXISTS ix_mail_templates_event ON engagement.mail_templates (event_id);

DROP TRIGGER IF EXISTS tg_mail_templates_updated_at ON engagement.mail_templates;
CREATE TRIGGER tg_mail_templates_updated_at
    BEFORE UPDATE ON engagement.mail_templates
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

DROP TRIGGER IF EXISTS tg_mail_templates_audit ON engagement.mail_templates;
CREATE TRIGGER tg_mail_templates_audit
    AFTER INSERT OR UPDATE OR DELETE ON engagement.mail_templates
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE engagement.mail_templates IS
    'Gabarits de courriel personnalisés du back-office : mise en page de base, textes, images (media, rôles background et logo), édition de rattachement.';

-- Rejouée sur une base qui portait les six mises en page du premier passage.
ALTER TABLE engagement.mail_templates DROP CONSTRAINT IF EXISTS ck_mail_templates_layout;
ALTER TABLE engagement.mail_templates ADD CONSTRAINT ck_mail_templates_layout
    CHECK (layout IN ('sobre', 'institutionnel', 'annonce',
                      'banniere', 'texte_sur_image', 'institutionnel_logo', 'edition'));

ALTER TABLE engagement.mailings
    ADD COLUMN IF NOT EXISTS mail_template_id uuid
        REFERENCES engagement.mail_templates(id) ON DELETE SET NULL;

-- Les codes des mises en page de l'édition, renommés avant toute mise en ligne.
ALTER TABLE engagement.mailings DROP CONSTRAINT IF EXISTS ck_mailings_template;
UPDATE engagement.mailings SET template = 'banniere' WHERE template = 'banniere_edition';
UPDATE engagement.mailings SET template = 'institutionnel_logo' WHERE template = 'institutionnel_edition';
ALTER TABLE engagement.mailings ADD CONSTRAINT ck_mailings_template
    CHECK (template IN ('sobre', 'institutionnel', 'annonce',
                        'banniere', 'texte_sur_image', 'institutionnel_logo', 'edition'));

CREATE OR REPLACE FUNCTION engagement.mailing_suggested_template(p_mailing_id uuid)
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT t.id
    FROM engagement.mailings m
    JOIN engagement.mail_templates t
      ON t.event_id = m.event_id AND t.preselect_for_submitters
    WHERE m.id = p_mailing_id
      AND (EXISTS (SELECT 1 FROM jsonb_array_elements(
                       CASE WHEN jsonb_typeof(m.audience) = 'array' THEN m.audience ELSE '[]' END) c
                   WHERE c ->> 'kind' = 'submitters')
           OR EXISTS (SELECT 1
                      FROM unnest(m.extra_addresses) a
                      JOIN identity.people pe ON pe.primary_email = a
                      JOIN programme.proposals p
                        ON p.event_id = m.event_id AND p.deleted_at IS NULL AND p.status <> 'draft'
                       AND pe.id IN (p.submitted_by, p.contact_person_id)))
    LIMIT 1;
$$;

COMMENT ON FUNCTION engagement.mailing_suggested_template IS
    'Gabarit personnalisé à présélectionner pour un envoi qui écrit aux déposants de son édition ; NULL sinon.';

INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes,
     max_byte_size, expected_aspect_ratio) VALUES
    ('engagement', 'mail_templates', 'background', '{"fr":"Image de fond","en":"Background image"}', false,
     '{image/jpeg,image/png}', 2097152, 6.0000),
    ('engagement', 'mail_templates', 'logo', '{"fr":"Logo","en":"Logo"}', false,
     '{image/jpeg,image/png}', 2097152, NULL)
-- Rejouée sur une base qui portait le 3:1 du premier passage : la forme suit.
ON CONFLICT (owner_schema, owner_table, role)
    DO UPDATE SET expected_aspect_ratio = EXCLUDED.expected_aspect_ratio;

-- La variante `courriel` des bandeaux d'édition déjà servis : le worker ne
-- fabrique que ce qui manque, sans refaire l'analyse ni toucher l'objet.
INSERT INTO platform.jobs (queue, task, payload, idempotency_key, priority)
SELECT 'media', 'media.process_asset',
       jsonb_build_object('asset_id', a.id, 'bucket', a.bucket,
                          'object_key', a.object_key, 'mime_type', a.mime_type),
       'courriel:' || a.id, 50
FROM media.attachments t
JOIN media.assets a ON a.id = t.asset_id
WHERE t.owner_schema = 'event' AND t.owner_table = 'events' AND t.role = 'banner'
  AND a.status = 'ready' AND a.deleted_at IS NULL AND a.width > 1200
  AND NOT EXISTS (SELECT 1 FROM media.renditions r
                  WHERE r.asset_id = a.id AND r.variant_code = 'courriel')
ON CONFLICT DO NOTHING;

COMMIT;
