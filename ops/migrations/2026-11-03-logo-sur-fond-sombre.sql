-- Logo d'une édition sur fond sombre (03/11/2026). Rejouable.
-- Modèle : docs/database/050_media.sql (media.attachment_role,
-- media.attachable_roles). Un emplacement « Logo sur fond sombre » parmi les
-- images d'une édition au back-office, lu par le bandeau de la programmation.
-- À jouer AVANT la mise en ligne : l'API rend ce rôle.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier

-- Hors transaction : une valeur ajoutée à un ENUM ne sert qu'une fois validée.
ALTER TYPE media.attachment_role ADD VALUE IF NOT EXISTS 'logo_inverse';

BEGIN;
INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size, expected_aspect_ratio)
VALUES
    ('event', 'events', 'logo_inverse', '{"fr":"Logo sur fond sombre","en":"Logo for dark backgrounds"}', false, '{image/*}', 5242880, NULL)
ON CONFLICT DO NOTHING;
COMMIT;
