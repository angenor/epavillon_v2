-- Photo d'un intervenant sans adresse (D16, suite, 27/10). Rejouable.
-- Modèle : docs/database/050_media.sql (media.attachable_roles).
--
-- Sans personne, pas de photo de profil : la photo se rattache à la ligne
-- programme.proposal_speakers, et le détail public d'une séance l'y lit.
-- Déployer l'API avec : elle garde la ligne d'un intervenant sans adresse d'un
-- enregistrement à l'autre (sa photo avec) et sert cette photo.
BEGIN;

INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size, expected_aspect_ratio)
VALUES
    ('programme', 'proposal_speakers', 'avatar', '{"fr":"Photo de l''intervenant","en":"Speaker picture"}',
     false, '{image/*}', 5242880, NULL)
ON CONFLICT DO NOTHING;

COMMIT;
