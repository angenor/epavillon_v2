-- Logo d'une édition et gabarits de courriel de l'édition (29/10). Rejouable.
-- Modèle : docs/database/050_media.sql (media.attachable_roles) et
-- docs/database/110_engagement.sql § 12 (engagement.mailings.template).
--
-- Une édition porte un logo (rôle `logo`, une image, sans forme imposée), posé
-- au back-office à côté du bandeau, de la couverture et de la vignette. Les
-- trois mises en page de courriel s'y ajoutent : 'banniere', 'texte_sur_image' et
-- 'institutionnel_logo', qui lisent la bannière et le logo de l'édition de
-- l'envoi.
-- Déployer avec l'API et le worker, qui rendent les gabarits. Ensuite
-- seulement, le lot 101 (ops/reprise-v1/lots/101-visuels-editions.sh).
BEGIN;

INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size, expected_aspect_ratio)
VALUES
    ('event', 'events', 'logo', '{"fr":"Logo de l''édition","en":"Edition logo"}', false, '{image/*}', 5242880, NULL)
ON CONFLICT DO NOTHING;

ALTER TABLE engagement.mailings DROP CONSTRAINT IF EXISTS ck_mailings_template;
ALTER TABLE engagement.mailings ADD CONSTRAINT ck_mailings_template
    CHECK (template IN ('sobre', 'institutionnel', 'annonce',
                        'banniere', 'texte_sur_image', 'institutionnel_logo'));

COMMIT;
