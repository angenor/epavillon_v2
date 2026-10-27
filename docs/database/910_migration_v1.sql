-- =============================================================================
-- ePavillon v2 : 910_migration_v1.sql
-- Reprise des données de la v1 (Supabase) : la table de correspondance des
-- identifiants, seule trace durable de la reprise dans le modèle.
--
-- Dépend de : 000_bootstrap (rien d'autre : aucune clé étrangère, la table
-- désigne des lignes de tous les modules).
--
-- -----------------------------------------------------------------------------
-- CE QUE CE FICHIER NE PORTE PLUS (27/10/2026)
--
-- La version précédente, écrite avant d'avoir vu la base réelle, portait aussi
-- l'outillage de la reprise : tables de correspondance des vocabulaires et des
-- rôles, résolution des organisations et des personnes, fonctions de
-- transformation, vue de réconciliation. Rien n'avait jamais tourné :
-- `migrate_activities` était un bouchon, `propose_organization_clusters`
-- écrivait des identifiants v1 dans `org.duplicate_candidates` (clé étrangère
-- vers `org.organizations`), `migrate_organizations` fusionnait en silence par
-- slug avec les fiches de production et forçait le type `ngo_association`,
-- `v_reconciliation` comparait `id_map` à lui-même. Les correspondances des
-- vocabulaires dataient d'avant la carte réelle (plan de reprise, § 1).
--
-- L'outillage vit désormais dans `ops/reprise-v1/`, scripts numérotés et
-- rejouables (docs/progression/reprise-v1/plan.md) : une règle de
-- correspondance se corrige en relançant, pas en migrant le modèle.
--
-- -----------------------------------------------------------------------------
-- LA ZONE DE TRANSIT
--
-- - `v1`, `v1_auth`, `v1_storage` : la v1 brute (schémas `public`, `auth`,
--   `storage` de Supabase), tables et données sans transformation. Créés par
--   `ops/reprise-v1/02-poser-v1-brute.sh`, absents d'une base chargée depuis
--   ce dossier.
-- - `legacy` : les tables de travail des scripts de transformation, qu'ils
--   créent eux-mêmes, et `id_map` ci-dessous, déclarée ici parce qu'elle
--   survit à la bascule.
--
-- Après la bascule et sa période d'observation (plan, étape 9) : suppression
-- de `v1`, `v1_auth`, `v1_storage` et de tout `legacy` hors `id_map`.
-- =============================================================================

CREATE SCHEMA IF NOT EXISTS legacy;
COMMENT ON SCHEMA legacy IS
    'Reprise des données de la v1 : table de correspondance des identifiants (durable) et tables de travail de l''outillage ops/reprise-v1 (supprimées après la bascule).';

-- -----------------------------------------------------------------------------
-- Correspondance des identifiants
--
-- Chaque ligne v1 rend des comptes (principe 3 du plan) : reprise, fusionnée
-- ou écartée, avec sa raison. La réconciliation compare, table v1 par table
-- v1, les lignes extraites aux lignes présentes ici.
--
-- Une ligne v1 a une seule cible : la ligne v2 qui la représente. Ce qu'elle
-- engendre d'autre se retrouve depuis cette cible (un compte Supabase donne une
-- personne, son compte v2 se retrouve par `person_id`).
--
-- Après la bascule, la table continue de servir : les anciennes adresses
-- (`/activities/<uuid>`) se redirigent par elle (D11), et le lien vers
-- l'identifiant Laravel des comptes importés en 2025 y attend la reprise des
-- années 2021 à 2024.
--
-- Pas de clé étrangère vers la cible : la table désigne des lignes de tous les
-- modules, et une ligne écartée n'en a pas.
-- -----------------------------------------------------------------------------
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
