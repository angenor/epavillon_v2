-- Reprise v1 : remplace l'ancien 910 (outillage écrit avant d'avoir vu Supabase,
-- jamais joué, en partie faux) par la seule table de correspondance des
-- identifiants. L'outillage vit dans ops/reprise-v1/. Rejouable.
-- Modèle : docs/database/910_migration_v1.sql.
--
-- Refuse de rien effacer si l'ancien `legacy` porte des lignes de reprise : en
-- production il est vide (ses deux tables de vocabulaire ne portent que ce que
-- l'ancien 910 y semait).
BEGIN;

DO $$
BEGIN
    IF to_regclass('legacy.enum_mapping') IS NULL THEN
        RETURN;
    END IF;
    IF EXISTS (SELECT 1 FROM legacy.id_map)
       OR EXISTS (SELECT 1 FROM legacy.organization_resolution)
       OR EXISTS (SELECT 1 FROM legacy.person_resolution) THEN
        RAISE EXCEPTION 'legacy porte déjà des lignes de reprise : migration arrêtée, rien n''est effacé.';
    END IF;
    DROP SCHEMA legacy CASCADE;
END $$;

CREATE SCHEMA IF NOT EXISTS legacy;
COMMENT ON SCHEMA legacy IS
    'Reprise des données de la v1 : table de correspondance des identifiants (durable) et tables de travail de l''outillage ops/reprise-v1 (supprimées après la bascule).';

CREATE TABLE IF NOT EXISTS legacy.id_map (
    legacy_table  text        NOT NULL,
    legacy_id     text        NOT NULL,
    target_schema text        NOT NULL,
    target_table  text        NOT NULL,
    target_id     uuid,
    disposition   text        NOT NULL DEFAULT 'migrated',
    note          text,
    migrated_at   timestamptz NOT NULL DEFAULT now(),

    PRIMARY KEY (legacy_table, legacy_id),
    CONSTRAINT ck_id_map_legacy_table
        CHECK (legacy_table ~ '^[a-z0-9_]+\.[a-z0-9_-]+$'),
    CONSTRAINT ck_id_map_disposition
        CHECK (disposition IN ('migrated', 'merged', 'skipped')),
    CONSTRAINT ck_id_map_target_shape
        CHECK ((disposition = 'skipped') = (target_id IS NULL)),
    CONSTRAINT ck_id_map_note
        CHECK (disposition = 'migrated' OR length(btrim(note)) > 0)
);

CREATE INDEX IF NOT EXISTS ix_id_map_target
    ON legacy.id_map (target_schema, target_table, target_id);

COMMENT ON TABLE legacy.id_map IS
    'Ce que chaque ligne de la v1 est devenue en v2 : reprise, fusionnée ou écartée, avec sa raison. Survit à la bascule (redirections des anciennes adresses).';
COMMENT ON COLUMN legacy.id_map.legacy_table IS
    'Table d''origine, sous son nom Supabase qualifié : public.activities, auth.users, storage.objects. Hors Supabase : laravel.users (identifiant Laravel des comptes importés), code.<fichier> (journées spéciales relevées dans le code v1).';
COMMENT ON COLUMN legacy.id_map.legacy_id IS
    'Identifiant d''origine, en texte : la v1 mêle uuid et entiers.';
COMMENT ON COLUMN legacy.id_map.target_id IS
    'Ligne v2 obtenue. Nulle si et seulement si la ligne v1 est écartée.';
COMMENT ON COLUMN legacy.id_map.disposition IS
    'migrated : reprise en une ligne v2 qui lui est propre. merged : rattachée à une ligne v2 déjà créée par une autre ligne v1 ou déjà en production (doublon, invité devenu membre). skipped : écartée, target_table dit où elle serait allée.';
COMMENT ON COLUMN legacy.id_map.note IS
    'La raison, obligatoire pour une fusion ou un écart : « doublon de … validé le … », « inscription PACO (D9) », « import Laravel en double ». Un écart sans raison bloque la bascule.';

-- Cible d'une ligne v1, nulle si elle est écartée ou pas encore reprise.
CREATE OR REPLACE FUNCTION legacy.map(p_table text, p_legacy_id text)
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT target_id FROM legacy.id_map
    WHERE legacy_table = p_table AND legacy_id = p_legacy_id;
$$;

-- Écrit la correspondance. Sans ON CONFLICT : une ligne v1 consignée deux fois
-- est une erreur du lot, pas une mise à jour.
CREATE OR REPLACE FUNCTION legacy.remember(
    p_table text, p_legacy_id text,
    p_target_schema text, p_target_table text, p_target_id uuid,
    p_disposition text DEFAULT 'migrated', p_note text DEFAULT NULL
)
RETURNS uuid
LANGUAGE sql
AS $$
    INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                               target_id, disposition, note)
    VALUES (p_table, p_legacy_id, p_target_schema, p_target_table, p_target_id,
            p_disposition, p_note)
    RETURNING target_id;
$$;

COMMIT;
