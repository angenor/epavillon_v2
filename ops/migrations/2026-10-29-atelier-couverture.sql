-- L'atelier de l'image de couverture (fiche d'un dossier, phase 3, 29/10). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql § 4 ter,
-- docs/database/050_media.sql § 1 (rôle) et la table blanche des rôles.
--
-- Additive, aucune donnée touchée :
--   - media.attachment_role gagne `cover_source` : la photo de départ de
--     l'atelier, gardée à part de l'image fabriquée ;
--   - media.attachable_roles : (programme, proposals, cover_source) ;
--   - programme.proposal_cover_designs : les choix de l'atelier, une ligne par
--     dossier, absente tant que l'atelier n'a jamais été enregistré.
-- Déployer l'API avec : elle sert GET /proposals/{id}/cover-studio et
-- PUT /proposals/{id}/cover-design. Jouer la migration AVANT de démarrer la
-- nouvelle API (ses requêtes nomment la nouvelle table et le nouveau rôle).

-- Hors de la transaction : une valeur ajoutée à une énumération ne peut servir
-- qu'une fois validée, et l'INSERT ci-dessous s'en sert.
ALTER TYPE media.attachment_role ADD VALUE IF NOT EXISTS 'cover_source';

BEGIN;

INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size, expected_aspect_ratio) VALUES
    ('programme', 'proposals', 'cover_source', '{"fr":"Photo de départ de la couverture","en":"Cover source photo"}', false, '{image/*}', 10485760, NULL)
ON CONFLICT DO NOTHING;

CREATE TABLE IF NOT EXISTS programme.proposal_cover_designs (
    proposal_id   uuid        PRIMARY KEY REFERENCES programme.proposals(id) ON DELETE CASCADE,
    template_code text        NOT NULL,
    accent        text        NOT NULL,
    options       jsonb       NOT NULL DEFAULT '{}',
    updated_at    timestamptz NOT NULL DEFAULT now(),
    updated_by    uuid        CONSTRAINT xmod_fk_proposal_cover_designs_updater
                              REFERENCES identity.people(id) ON DELETE SET NULL,
    CONSTRAINT ck_proposal_cover_designs_template CHECK (template_code ~ '^[a-z][a-z0-9-]{1,40}$'),
    CONSTRAINT ck_proposal_cover_designs_accent   CHECK (accent ~ '^[a-z][a-z0-9-]{1,40}$'),
    CONSTRAINT ck_proposal_cover_designs_options  CHECK (jsonb_typeof(options) = 'object')
);

DROP TRIGGER IF EXISTS tg_proposal_cover_designs_updated_at ON programme.proposal_cover_designs;
CREATE TRIGGER tg_proposal_cover_designs_updated_at
    BEFORE UPDATE ON programme.proposal_cover_designs
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE programme.proposal_cover_designs IS
    'Choix de l''atelier de l''image de couverture d''un dossier, pour le rouvrir tel qu''on l''a laissé. L''image produite est le rattachement `cover` ; modèles et couleurs vivent dans le code du site, la base n''en garde que les codes.';

COMMIT;
