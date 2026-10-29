-- Le jour J d'une activité retenue (fiche d'un dossier, phase 2, 29/10). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql § 4 et 4 bis,
-- docs/database/075_programme_sessions.sql § 7 ter.
--
-- Additive, aucune donnée touchée :
--   - programme.proposal_focal_points : les personnes à joindre sur place ;
--   - programme.proposal_technical_needs : les besoins techniques, une ligne
--     par dossier, absente tant que rien n'est déclaré ;
--   - programme.proposal_social_posts : les liens des publications à relayer ;
--   - programme.proposal_documents.presented_by_speaker_id : qui présente un
--     support (facultatif) ;
--   - programme.proposal_needs_deadline() : dix jours avant la première séance.
-- Déployer l'API avec : elle sert GET /proposals/{id}/day-plan et ses quatre
-- écritures, et lit la nouvelle colonne des pièces. Jouer la migration AVANT de
-- démarrer la nouvelle API (ses requêtes nomment les nouvelles tables).
BEGIN;

ALTER TABLE programme.proposal_documents
    ADD COLUMN IF NOT EXISTS presented_by_speaker_id uuid
        REFERENCES programme.proposal_speakers(id) ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS programme.proposal_focal_points (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    proposal_id uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    speaker_id  uuid        REFERENCES programme.proposal_speakers(id) ON DELETE CASCADE,
    person_id   uuid        CONSTRAINT xmod_fk_proposal_focal_points_person
                            REFERENCES identity.people(id) ON DELETE SET NULL,
    full_name   text        NOT NULL,
    phone       text        NOT NULL DEFAULT '',
    whatsapp    text        NOT NULL DEFAULT '',
    sort_order  smallint    NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_proposal_focal_points_name CHECK (
        length(btrim(full_name)) > 0 AND length(full_name) <= 200),
    CONSTRAINT ck_proposal_focal_points_phones CHECK (
        length(phone) <= 50 AND length(whatsapp) <= 50)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_proposal_focal_points_speaker ON programme.proposal_focal_points (proposal_id, speaker_id)
    WHERE speaker_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ux_proposal_focal_points_person  ON programme.proposal_focal_points (proposal_id, person_id)
    WHERE person_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_proposal_focal_points_proposal ON programme.proposal_focal_points (proposal_id, sort_order);
CREATE INDEX IF NOT EXISTS ix_proposal_focal_points_person   ON programme.proposal_focal_points (person_id);

COMMENT ON TABLE programme.proposal_focal_points IS
    'Personnes à joindre le jour J d''une activité retenue. Au moins une une fois la liste enregistrée ; le déposant est proposé par défaut.';

CREATE TABLE IF NOT EXISTS programme.proposal_technical_needs (
    proposal_id           uuid        PRIMARY KEY REFERENCES programme.proposals(id) ON DELETE CASCADE,
    video                 boolean     NOT NULL DEFAULT false,
    remote                boolean     NOT NULL DEFAULT false,
    remote_details        text        NOT NULL DEFAULT '',
    accessibility         boolean     NOT NULL DEFAULT false,
    accessibility_details text        NOT NULL DEFAULT '',
    other                 text        NOT NULL DEFAULT '',
    updated_at            timestamptz NOT NULL DEFAULT now(),
    updated_by            uuid        CONSTRAINT xmod_fk_proposal_technical_needs_updater
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    CONSTRAINT ck_proposal_technical_needs_lengths CHECK (
        length(remote_details) <= 2000 AND length(accessibility_details) <= 2000
        AND length(other) <= 4000)
);

DROP TRIGGER IF EXISTS tg_proposal_technical_needs_updated_at ON programme.proposal_technical_needs;
CREATE TRIGGER tg_proposal_technical_needs_updated_at
    BEFORE UPDATE ON programme.proposal_technical_needs
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE programme.proposal_technical_needs IS
    'Besoins techniques d''une activité retenue, déclarés par l''organisation. Ligne absente : rien de déclaré.';

CREATE TABLE IF NOT EXISTS programme.proposal_social_posts (
    id          uuid         PRIMARY KEY DEFAULT platform.uuid_v7(),
    proposal_id uuid         NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    url         platform.url NOT NULL,
    created_by  uuid         CONSTRAINT xmod_fk_proposal_social_posts_creator
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at  timestamptz  NOT NULL DEFAULT now(),
    CONSTRAINT ux_proposal_social_posts UNIQUE (proposal_id, url)
);

COMMENT ON TABLE programme.proposal_social_posts IS
    'Liens des publications sur les réseaux sociaux relatives à une activité. La plateforme se déduit de l''adresse.';

CREATE OR REPLACE FUNCTION programme.proposal_needs_deadline(p_proposal_id uuid)
RETURNS timestamptz
LANGUAGE sql
STABLE
AS $$
    SELECT min(s.starts_at) - interval '10 days'
      FROM programme.sessions s
     WHERE s.proposal_id = p_proposal_id
       AND s.status <> 'cancelled';
$$;

COMMENT ON FUNCTION programme.proposal_needs_deadline IS
    'Dix jours avant la première séance non annulée du dossier ; NULL sans séance. Indicative.';

COMMIT;
