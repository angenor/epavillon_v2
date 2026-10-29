-- Supprimer un envoi de courriels du back-office (29/10). Rejouable.
-- Modèle : docs/database/050_media.sql § 4 (media.tg_drop_owner_attachments),
-- docs/database/110_engagement.sql § 12 (tg_mailings_drop_attachments).
--
-- Additive, aucune donnée touchée : un envoi supprimé emporte ses rattachements
-- de pièces jointes (les objets restent, purgés plus tard s'ils sont orphelins).
-- Sans elle, l'API supprime quand même ; les rattachements resteraient orphelins.
-- À jouer en production avant de démarrer l'API qui supprime les envois partis.
BEGIN;

CREATE OR REPLACE FUNCTION media.tg_drop_owner_attachments()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    DELETE FROM media.attachments
     WHERE owner_schema = TG_TABLE_SCHEMA
       AND owner_table  = TG_TABLE_NAME
       AND owner_id     = OLD.id;
    RETURN NULL;
END;
$$;

COMMENT ON FUNCTION media.tg_drop_owner_attachments() IS
    'Déclencheur AFTER DELETE d''une table porteuse : retire les rattachements de la ligne supprimée, jamais les objets (purgés plus tard s''ils sont orphelins).';

DROP TRIGGER IF EXISTS tg_mailings_drop_attachments ON engagement.mailings;
CREATE TRIGGER tg_mailings_drop_attachments
    AFTER DELETE ON engagement.mailings
    FOR EACH ROW EXECUTE FUNCTION media.tg_drop_owner_attachments();

-- Les brouillons supprimés avant ce déclencheur ont laissé leurs rattachements.
DELETE FROM media.attachments x
 WHERE x.owner_schema = 'engagement' AND x.owner_table = 'mailings'
   AND NOT EXISTS (SELECT 1 FROM engagement.mailings m WHERE m.id = x.owner_id);

COMMIT;
