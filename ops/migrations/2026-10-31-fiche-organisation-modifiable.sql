-- Modifier la fiche d'une organisation depuis le back-office (31/10). Rejouable.
-- Modèle : docs/database/040_organizations.sql (org.tg_sync_organization_names, org.retire_name).
--
-- Un nom légal ou un sigle remplacé devient un ancien nom (`former`), toujours
-- trouvable par la recherche. Reprise : les fiches dont le nom a déjà changé
-- (choix de fusion) portent deux noms légaux ; celui que la fiche ne porte plus
-- passe en `former`.

CREATE OR REPLACE FUNCTION org.tg_sync_organization_names()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        PERFORM org.retire_name(NEW.id, OLD.legal_name, NEW.legal_name, 'legal');
        IF OLD.acronym IS NOT NULL THEN
            PERFORM org.retire_name(NEW.id, OLD.acronym, NEW.acronym, 'acronym');
        END IF;
    END IF;

    INSERT INTO org.organization_names (organization_id, name, kind, is_confirmed)
    VALUES (NEW.id, NEW.legal_name, 'legal', true)
    ON CONFLICT (organization_id, name_normalized, kind) DO UPDATE SET name = EXCLUDED.name;

    IF NEW.acronym IS NOT NULL THEN
        INSERT INTO org.organization_names (organization_id, name, kind, is_confirmed)
        VALUES (NEW.id, NEW.acronym, 'acronym', true)
        ON CONFLICT (organization_id, name_normalized, kind) DO UPDATE SET name = EXCLUDED.name;
    END IF;

    RETURN NULL;
END;
$$;

-- Passe une dénomination dérivée (`legal` ou `acronym`) en `former` quand la
-- fiche ne la porte plus. Déjà connue comme ancien nom : la ligne dérivée part.
CREATE OR REPLACE FUNCTION org.retire_name(
    p_organization_id uuid, p_old text, p_new text, p_kind org.name_kind)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    IF platform.normalize_label(p_old) IS NOT DISTINCT FROM platform.normalize_label(p_new) THEN
        RETURN;
    END IF;

    UPDATE org.organization_names n
       SET kind = 'former', is_confirmed = true
     WHERE n.organization_id = p_organization_id
       AND n.kind = p_kind
       AND n.name_normalized = platform.normalize_label(p_old)
       AND NOT EXISTS (
           SELECT 1 FROM org.organization_names f
            WHERE f.organization_id = p_organization_id
              AND f.kind = 'former'
              AND f.name_normalized = n.name_normalized);

    DELETE FROM org.organization_names
     WHERE organization_id = p_organization_id
       AND kind = p_kind
       AND name_normalized = platform.normalize_label(p_old);
END;
$$;

DELETE FROM org.organization_names n
 USING org.organizations o
 WHERE o.id = n.organization_id
   AND n.kind IN ('legal', 'acronym')
   AND n.name_normalized IS DISTINCT FROM (CASE n.kind WHEN 'legal' THEN o.legal_name_normalized ELSE o.acronym_normalized END)
   AND EXISTS (SELECT 1 FROM org.organization_names f
                WHERE f.organization_id = n.organization_id AND f.kind = 'former'
                  AND f.name_normalized = n.name_normalized);

UPDATE org.organization_names n
   SET kind = 'former', is_confirmed = true
  FROM org.organizations o
 WHERE o.id = n.organization_id
   AND n.kind IN ('legal', 'acronym')
   AND n.name_normalized IS DISTINCT FROM (CASE n.kind WHEN 'legal' THEN o.legal_name_normalized ELSE o.acronym_normalized END);
