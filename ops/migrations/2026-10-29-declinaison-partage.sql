-- Déclinaison `partage` des couvertures déjà servies (29/10). Rejouable.
--
-- Le worker fabrique désormais, pour toute image de plus de 1200 px, une
-- déclinaison `partage` : 1200 px, JPEG sous 100 Ko, l'aperçu d'un lien partagé
-- (`og:image`). Les couvertures déposées avant n'en ont pas : on remet leur
-- traitement en file ; un objet servable reçoit seulement ce qui lui manque,
-- sans nouvelle analyse ni changement d'état. Aucun changement de schéma.

BEGIN;

INSERT INTO platform.jobs (queue, task, payload, idempotency_key, priority)
SELECT DISTINCT ON (a.id)
       'media', 'media.process_asset',
       jsonb_build_object('asset_id', a.id, 'bucket', a.bucket,
                          'object_key', a.object_key, 'mime_type', a.mime_type),
       'partage:' || a.id, 50
FROM media.attachments t
JOIN media.assets a ON a.id = t.asset_id
WHERE t.role = 'cover'
  AND a.status = 'ready' AND a.deleted_at IS NULL AND a.width > 1200
  AND NOT EXISTS (SELECT 1 FROM media.renditions r
                  WHERE r.asset_id = a.id AND r.variant_code = 'partage')
ON CONFLICT DO NOTHING;

COMMIT;
