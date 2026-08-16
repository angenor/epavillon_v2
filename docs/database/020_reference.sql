-- =============================================================================
-- ePavillon v2 — 020_reference.sql
-- Référentiels partagés : locales, pays et espace francophone, taxonomies.
--
-- Dépend de : 000_bootstrap.sql, 010_platform.sql
--
-- DÉCISION STRUCTURANTE — sortie des ENUM PostgreSQL
-- La v1 encodait les thématiques, catégories et secteurs en types ENUM
-- (`activity_theme` comptait 15 valeurs). Conséquences : ajouter un thème pour
-- la COP31 imposait une migration DDL, on ne pouvait ni renommer, ni retirer,
-- ni traduire, ni ordonner une valeur, et le libellé affiché vivait dans les
-- fichiers i18n du frontend.
-- Ici, les taxonomies ouvertes deviennent des données (reference.taxonomy_terms)
-- administrables depuis le back-office. Les ENUM restent réservés aux machines
-- à états fermées (statuts de workflow) : leur liste engage le code métier.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Locales
-- -----------------------------------------------------------------------------
CREATE TABLE reference.locales (
    code         text        PRIMARY KEY CHECK (code ~ '^[a-z]{2}(-[A-Z]{2})?$'),
    native_label text        NOT NULL,
    english_label text       NOT NULL,
    is_default   boolean     NOT NULL DEFAULT false,
    is_active    boolean     NOT NULL DEFAULT true,
    sort_order   smallint    NOT NULL DEFAULT 0,
    -- Configuration du dictionnaire de recherche plein texte associé.
    text_search_config regconfig NOT NULL DEFAULT 'simple'
);

-- Une seule locale par défaut, garantie par index unique partiel.
CREATE UNIQUE INDEX ux_locales_single_default ON reference.locales ((true)) WHERE is_default;

INSERT INTO reference.locales (code, native_label, english_label, is_default, sort_order, text_search_config) VALUES
    ('fr', 'Français', 'French',  true,  1, 'french'),
    ('en', 'English',  'English', false, 2, 'english')
ON CONFLICT (code) DO NOTHING;

COMMENT ON TABLE reference.locales IS
    'Langues supportées par la plateforme. Ajouter une locale ne nécessite aucune migration de schéma.';

-- -----------------------------------------------------------------------------
-- 2. Pays et espace francophone
--
-- Ajout par rapport à la v1 : le statut au sein de l'OIF. C'est une donnée
-- structurante pour l'IFDD (priorisation des propositions, quotas de
-- participation, statistiques par collège) qui était jusqu'ici traitée à la
-- main dans des fichiers Excel.
-- -----------------------------------------------------------------------------
CREATE TYPE reference.oif_membership AS ENUM ('member', 'associate', 'observer', 'none');

CREATE TABLE reference.countries (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    iso2           char(2)     NOT NULL UNIQUE,
    iso3           char(3)     NOT NULL UNIQUE,
    numeric_code   char(3),
    name           platform.i18n_text NOT NULL,
    official_name  platform.i18n_text,
    -- Nom canonique pour la recherche floue (« côte d'ivoire » = « cote divoire »).
    name_normalized text GENERATED ALWAYS AS (platform.normalize_label(name ->> 'fr')) STORED,
    region_code    text,               -- ex. 'africa-west', 'europe', 'caribbean'
    continent      text        CHECK (continent IN ('africa','europe','asia','oceania','north_america','south_america','antarctica')),
    oif_status     reference.oif_membership NOT NULL DEFAULT 'none',
    default_timezone platform.timezone_name,
    calling_code   text,
    flag_emoji     text,
    is_active      boolean     NOT NULL DEFAULT true,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_countries_name_trgm ON reference.countries USING gin (name_normalized gin_trgm_ops);
CREATE INDEX ix_countries_oif        ON reference.countries (oif_status) WHERE is_active;

CREATE TRIGGER tg_countries_updated_at
    BEFORE UPDATE ON reference.countries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON COLUMN reference.countries.oif_status IS
    'Statut au sein de l''Organisation internationale de la Francophonie : membre, membre associé, observateur.';

-- -----------------------------------------------------------------------------
-- 3. Taxonomies administrables
--
-- Un vocabulaire = une ligne dans `taxonomies` ; ses valeurs = des lignes dans
-- `taxonomy_terms`, éventuellement hiérarchiques (un thème « Adaptation » peut
-- porter des sous-thèmes). Chaque terme est traduit, ordonné, activable, et
-- conserve son code stable : le frontend et les exports statistiques
-- s'appuient sur `code`, jamais sur le libellé.
-- -----------------------------------------------------------------------------
CREATE TABLE reference.taxonomies (
    code            text        PRIMARY KEY CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    label           platform.i18n_text NOT NULL,
    description     platform.i18n_text,
    -- Un terme peut-il être choisi plusieurs fois sur une même entité ?
    is_multi_select boolean     NOT NULL DEFAULT true,
    is_hierarchical boolean     NOT NULL DEFAULT false,
    -- Verrouillage : certaines taxonomies (ex. types d'organisation) pilotent
    -- des règles métier ; leurs codes ne doivent pas être modifiés librement.
    is_system       boolean     NOT NULL DEFAULT false,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE reference.taxonomy_terms (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    taxonomy_code  text        NOT NULL REFERENCES reference.taxonomies(code) ON DELETE RESTRICT,
    parent_id      uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    code           text        NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    label          platform.i18n_text NOT NULL,
    description    platform.i18n_text,
    -- Identité visuelle du terme (pastilles du calendrier, filtres) : évite de
    -- disséminer des `switch` de couleurs dans le frontend.
    color_hex      text        CHECK (color_hex ~ '^#[0-9a-fA-F]{6}$'),
    icon           text,
    sort_order     smallint    NOT NULL DEFAULT 0,
    is_active      boolean     NOT NULL DEFAULT true,
    -- Terme remplacé : on ne supprime jamais un terme utilisé historiquement,
    -- on le déprécie et on pointe vers son successeur.
    superseded_by  uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE SET NULL,
    metadata       jsonb       NOT NULL DEFAULT '{}'::jsonb,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_taxonomy_terms_code UNIQUE (taxonomy_code, code),
    CONSTRAINT ck_taxonomy_terms_not_self CHECK (parent_id IS DISTINCT FROM id)
);

CREATE INDEX ix_taxonomy_terms_taxonomy ON reference.taxonomy_terms (taxonomy_code, sort_order) WHERE is_active;
CREATE INDEX ix_taxonomy_terms_parent   ON reference.taxonomy_terms (parent_id) WHERE parent_id IS NOT NULL;

CREATE TRIGGER tg_taxonomy_terms_updated_at
    BEFORE UPDATE ON reference.taxonomy_terms
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- Un terme doit appartenir à la même taxonomie que son parent.
CREATE OR REPLACE FUNCTION reference.tg_taxonomy_term_check_parent()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_parent_taxonomy text;
BEGIN
    IF NEW.parent_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT taxonomy_code INTO v_parent_taxonomy
    FROM reference.taxonomy_terms WHERE id = NEW.parent_id;

    IF v_parent_taxonomy IS DISTINCT FROM NEW.taxonomy_code THEN
        RAISE EXCEPTION 'Le terme parent appartient à la taxonomie % et non %',
            v_parent_taxonomy, NEW.taxonomy_code
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_taxonomy_terms_check_parent
    BEFORE INSERT OR UPDATE OF parent_id, taxonomy_code ON reference.taxonomy_terms
    FOR EACH ROW EXECUTE FUNCTION reference.tg_taxonomy_term_check_parent();

COMMENT ON TABLE reference.taxonomy_terms IS
    'Valeurs des vocabulaires métier (thèmes, catégories, secteurs...). Remplace les ENUM de la v1.';

-- -----------------------------------------------------------------------------
-- 4. Rattachement générique d'une entité à des termes
--
-- Une seule table pour « cette proposition traite d'adaptation et de finance »,
-- « cet article relève de la biodiversité », « cette formation cible
-- l'agriculture ». Les modules n'ont plus à créer leur propre table de liaison
-- ni à empiler des colonnes tableau (`main_themes activity_theme[]` en v1, qui
-- n'offrait aucune intégrité référentielle).
--
-- Le lien vers l'entité porteuse est volontairement souple (schéma + table +
-- id) : c'est le prix à payer pour rester transversal aux modules sans créer de
-- dépendances croisées. Le nettoyage est assuré par platform.purge_term_links().
-- -----------------------------------------------------------------------------
CREATE TABLE reference.entity_terms (
    entity_schema text     NOT NULL,
    entity_table  text     NOT NULL,
    entity_id     uuid     NOT NULL,
    term_id       uuid     NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    -- Rôle du rattachement : distingue « thème principal » de « thème secondaire »
    -- sur la même taxonomie.
    role          text     NOT NULL DEFAULT 'primary',
    sort_order    smallint NOT NULL DEFAULT 0,
    created_at    timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (entity_schema, entity_table, entity_id, term_id, role)
);

CREATE INDEX ix_entity_terms_term ON reference.entity_terms (term_id, entity_schema, entity_table);

COMMENT ON TABLE reference.entity_terms IS
    'Rattachement N-N entre une entité de n''importe quel module et des termes de taxonomie.';

-- Aide à la lecture : agrège les termes d'une entité sous forme de tableau de codes.
CREATE OR REPLACE FUNCTION reference.terms_of(
    p_schema text, p_table text, p_entity_id uuid, p_taxonomy text DEFAULT NULL
)
RETURNS text[]
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(array_agg(t.code ORDER BY et.sort_order, t.sort_order), '{}')
    FROM reference.entity_terms et
    JOIN reference.taxonomy_terms t ON t.id = et.term_id
    WHERE et.entity_schema = p_schema
      AND et.entity_table = p_table
      AND et.entity_id = p_entity_id
      AND (p_taxonomy IS NULL OR t.taxonomy_code = p_taxonomy);
$$;

-- -----------------------------------------------------------------------------
-- 5. Amorçage des vocabulaires
--
-- Les codes reprennent ceux de la v1 afin que la reprise de données soit une
-- simple correspondance, sans réécriture des contenus existants.
-- -----------------------------------------------------------------------------
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('activity_theme',    '{"fr":"Thématiques","en":"Themes"}',                    '{"fr":"Thématiques traitées par une activité ou une publication","en":"Themes covered by an activity or a publication"}', true,  true,  false),
    ('activity_category', '{"fr":"Catégories d''activité","en":"Activity categories"}', '{"fr":"Nature de l''activité proposée","en":"Nature of the proposed activity"}', true,  false, false),
    ('organization_type', '{"fr":"Types d''organisation","en":"Organization types"}',   '{"fr":"Nature juridique de l''organisation","en":"Legal nature of the organization"}', false, false, true),
    ('application_sector','{"fr":"Secteurs d''application","en":"Application sectors"}','{"fr":"Secteur économique visé","en":"Targeted economic sector"}', true,  false, false),
    ('document_type',     '{"fr":"Types de document","en":"Document types"}',           NULL, false, false, true),
    ('negotiation_track', '{"fr":"Filières de négociation","en":"Negotiation tracks"}',  '{"fr":"Convention de rattachement : climat, biodiversité, désertification","en":"Related convention"}', false, false, true),
    ('referral_source',   '{"fr":"Canaux d''acquisition","en":"Referral sources"}',      '{"fr":"Comment le participant a connu l''activité","en":"How the attendee heard about the activity"}', false, false, false),
    ('media_license',     '{"fr":"Licences des médias","en":"Media licences"}',          '{"fr":"Conditions de réutilisation d''un fichier téléversé","en":"Reuse terms of an uploaded file"}', false, false, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    -- Thématiques (reprise fidèle de l'ENUM activity_theme v1)
    ('activity_theme', 'mitigation',                        '{"fr":"Atténuation","en":"Mitigation"}', 10),
    ('activity_theme', 'adaptation',                        '{"fr":"Adaptation","en":"Adaptation"}', 20),
    ('activity_theme', 'climate_ambition_ndc',              '{"fr":"Ambition climatique et CDN","en":"Climate ambition and NDCs"}', 30),
    ('activity_theme', 'loss_and_damage',                   '{"fr":"Pertes et préjudices","en":"Loss and damage"}', 40),
    ('activity_theme', 'water_fisheries',                   '{"fr":"Eau et pêche","en":"Water and fisheries"}', 50),
    ('activity_theme', 'renewable_energy_land',             '{"fr":"Énergies renouvelables et territoires","en":"Renewable energy and land"}', 60),
    ('activity_theme', 'health_solidarity',                 '{"fr":"Santé et solidarité","en":"Health and solidarity"}', 70),
    ('activity_theme', 'industry_transition_and_technology','{"fr":"Transition industrielle et technologie","en":"Industrial transition and technology"}', 80),
    ('activity_theme', 'transport_urbanization',            '{"fr":"Transport et urbanisation","en":"Transport and urbanization"}', 90),
    ('activity_theme', 'climate_justice_indigenous',        '{"fr":"Justice climatique et peuples autochtones","en":"Climate justice and indigenous peoples"}', 100),
    ('activity_theme', 'agriculture_food',                  '{"fr":"Agriculture et alimentation","en":"Agriculture and food"}', 110),
    ('activity_theme', 'sustainable_livestock',             '{"fr":"Élevage durable","en":"Sustainable livestock"}', 120),
    ('activity_theme', 'climate_finance',                   '{"fr":"Finance climatique","en":"Climate finance"}', 130),
    ('activity_theme', 'gender',                            '{"fr":"Genre","en":"Gender"}', 140),
    ('activity_theme', 'transparency',                      '{"fr":"Transparence","en":"Transparency"}', 150),
    ('activity_theme', 'biodiversity',                      '{"fr":"Biodiversité","en":"Biodiversity"}', 160),
    ('activity_theme', 'desertification',                   '{"fr":"Désertification","en":"Desertification"}', 170),

    -- Catégories d'activité
    ('activity_category', 'capacity_building',       '{"fr":"Renforcement de capacités","en":"Capacity building"}', 10),
    ('activity_category', 'results_sharing',         '{"fr":"Partage de résultats","en":"Results sharing"}', 20),
    ('activity_category', 'technological_innovation','{"fr":"Innovation technologique","en":"Technological innovation"}', 30),
    ('activity_category', 'field_project',           '{"fr":"Projet de terrain","en":"Field project"}', 40),
    ('activity_category', 'best_practices',          '{"fr":"Bonnes pratiques","en":"Best practices"}', 50),
    ('activity_category', 'awareness',               '{"fr":"Sensibilisation","en":"Awareness"}', 60),
    ('activity_category', 'concertation',            '{"fr":"Concertation","en":"Consultation"}', 70),

    -- Types d'organisation
    ('organization_type', 'public_national_institution', '{"fr":"Institution publique nationale","en":"National public institution"}', 10),
    ('organization_type', 'international_organization',  '{"fr":"Organisation internationale","en":"International organization"}', 20),
    ('organization_type', 'regional_organization',       '{"fr":"Organisation régionale","en":"Regional organization"}', 30),
    ('organization_type', 'ngo_association',             '{"fr":"ONG / Association","en":"NGO / Association"}', 40),
    ('organization_type', 'private_sector',              '{"fr":"Secteur privé","en":"Private sector"}', 50),
    ('organization_type', 'academic_research',           '{"fr":"Université / Recherche","en":"Academia / Research"}', 60),
    ('organization_type', 'media',                       '{"fr":"Média","en":"Media"}', 70),

    -- Secteurs d'application
    ('application_sector', 'agriculture', '{"fr":"Agriculture","en":"Agriculture"}', 10),
    ('application_sector', 'livestock',   '{"fr":"Élevage","en":"Livestock"}', 20),
    ('application_sector', 'industry',    '{"fr":"Industrie","en":"Industry"}', 30),
    ('application_sector', 'energy',      '{"fr":"Énergie","en":"Energy"}', 40),
    ('application_sector', 'other',       '{"fr":"Autre","en":"Other"}', 90),

    -- Filières de négociation (ex-ENUM session_category_v2)
    ('negotiation_track', 'climate',         '{"fr":"Climat (CCNUCC)","en":"Climate (UNFCCC)"}', 10),
    ('negotiation_track', 'biodiversity',    '{"fr":"Biodiversité (CDB)","en":"Biodiversity (CBD)"}', 20),
    ('negotiation_track', 'desertification', '{"fr":"Désertification (CNULCD)","en":"Desertification (UNCCD)"}', 30),

    -- Types de document
    ('document_type', 'negotiation_guide',   '{"fr":"Guide de négociation","en":"Negotiation guide"}', 10),
    ('document_type', 'technical_note',      '{"fr":"Note technique","en":"Technical note"}', 20),
    ('document_type', 'relevant_document',   '{"fr":"Document de référence","en":"Reference document"}', 30),
    ('document_type', 'presentation',        '{"fr":"Présentation","en":"Presentation"}', 40),
    ('document_type', 'report',              '{"fr":"Rapport","en":"Report"}', 50),
    ('document_type', 'other',               '{"fr":"Autre","en":"Other"}', 90),

    -- Canaux d'acquisition (généralisation de la feature 006 de la v1)
    ('referral_source', 'ifdd_website',      '{"fr":"Site web de l''IFDD","en":"IFDD website"}', 10),
    ('referral_source', 'ifdd_linkedin',     '{"fr":"LinkedIn de l''IFDD","en":"IFDD LinkedIn"}', 20),
    ('referral_source', 'ifdd_facebook',     '{"fr":"Facebook de l''IFDD","en":"IFDD Facebook"}', 30),
    ('referral_source', 'ifdd_x',            '{"fr":"X (Twitter) de l''IFDD","en":"IFDD X (Twitter)"}', 40),
    ('referral_source', 'email_newsletter',  '{"fr":"Infolettre","en":"Newsletter"}', 50),
    ('referral_source', 'word_of_mouth',     '{"fr":"Bouche-à-oreille","en":"Word of mouth"}', 60),
    ('referral_source', 'other',             '{"fr":"Autre","en":"Other"}', 90),

    -- Licences des médias : condition de réutilisation d'un fichier téléversé.
    -- Renseignée à l'import, elle évite d'avoir à retrouver après coup si une
    -- photo de COP peut être republiée par un partenaire.
    ('media_license', 'all_rights_reserved', '{"fr":"Tous droits réservés","en":"All rights reserved"}', 10),
    ('media_license', 'cc_by',               '{"fr":"CC BY","en":"CC BY"}', 20),
    ('media_license', 'cc_by_sa',            '{"fr":"CC BY-SA","en":"CC BY-SA"}', 30),
    ('media_license', 'cc_by_nc',            '{"fr":"CC BY-NC","en":"CC BY-NC"}', 40),
    ('media_license', 'cc_by_nc_sa',         '{"fr":"CC BY-NC-SA","en":"CC BY-NC-SA"}', 50),
    ('media_license', 'public_domain',       '{"fr":"Domaine public","en":"Public domain"}', 60),
    ('media_license', 'ifdd_internal',       '{"fr":"Usage interne IFDD","en":"IFDD internal use"}', 70)
ON CONFLICT (taxonomy_code, code) DO NOTHING;
