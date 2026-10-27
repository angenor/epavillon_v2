-- Lot 30 : organisations, dénominations, domaines, adhésions.
-- Carte : docs/progression/reprise-v1/carte-reelle-coeur.md § 4. Dépend du lot 20
-- (personnes : legacy.map('auth.users', …)) et du lot 10 (pays, s'il les consigne).
--
-- Doublons (D4, règle métier 1) : seul l'évident est fusionné ici, sans fiche
-- absorbée en v2 (la v1 n'a jamais exposé ces doublons) ; l'incertain part dans
-- legacy.organisations_groupes, exporté pour validation par le commanditaire.
-- Évident = même nom normalisé et même pays, ou même sigle, même pays et noms
-- à une faute près (trigramme >= 0,9). La cible est une fiche déjà en base si
-- elle existe (l'IFDD semée), sinon la fiche v1 vérifiée, puis la plus ancienne.

\set ON_ERROR_STOP on
BEGIN;
DO $$ BEGIN
    PERFORM set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true);
    PERFORM set_config('app.request_id', 'reprise-v1-30', true);
END $$;

-- Domaines qui ne prouvent aucune appartenance : la liste du modèle, plus ceux
-- relevés dans la v1 (variantes et faute de frappe de messageries, adresse générique).
CREATE TEMP TABLE domaines_publics ON COMMIT DROP AS
SELECT domain FROM org.public_email_domains
UNION SELECT unnest(ARRAY['ymail.com', 'gamil.com', 'organisation.org']);

CREATE TEMP TABLE o30 ON COMMIT DROP AS
WITH brut AS (
    SELECT o.*,
           btrim(o.name) AS nom,
           NULLIF(btrim(o.acronym), '') AS sigle_brut,
           -- Espaces après « :// » seulement (saisie « https:// www… ») ; ailleurs,
           -- l'adresse est illisible et passe à nul.
           regexp_replace(btrim(o.website), '^(https?://)\s+', '\1', 'i') AS site_brut,
           lower(btrim(o.email)) AS courriel
    FROM v1.organizations o
)
SELECT b.id AS v1_id,
       b.nom,
       platform.normalize_label(b.nom) AS nom_norm,
       CASE WHEN length(b.sigle_brut) BETWEEN 2 AND 32 THEN b.sigle_brut END AS sigle,
       b.sigle_brut IS NOT NULL AND length(b.sigle_brut) NOT BETWEEN 2 AND 32 AS sigle_refuse,
       platform.normalize_label(CASE WHEN length(b.sigle_brut) BETWEEN 2 AND 32 THEN b.sigle_brut END) AS sigle_norm,
       CASE WHEN b.site_brut ~ '^https?://[^\s]+$' AND length(b.site_brut) <= 2048 THEN b.site_brut END AS site,
       NULLIF(b.site_brut, '') IS NOT NULL AND b.site_brut !~ '^https?://[^\s]+$' AS site_refuse,
       CASE WHEN b.courriel ~ '^[^@\s]+@[^@\s]+\.[^@\s]{2,}$' THEN b.courriel END AS courriel,
       b.courriel !~ '^[^@\s]+@[^@\s]+\.[^@\s]{2,}$' AS courriel_refuse,
       split_part(b.courriel, '@', 2) AS domaine,
       b.organization_type::text AS type_code,
       COALESCE(legacy.map('public.countries', b.country_id::text),
                (SELECT r.id FROM v1.countries c JOIN reference.countries r ON r.iso2 = upper(c.code)
                  WHERE c.id = b.country_id)) AS pays_id,
       NULLIF(btrim(b.description), '') AS description,
       COALESCE(b.is_verified, false) AS verifiee,
       COALESCE(b.verified_at, b.updated_at) AS verifiee_le,
       legacy.map('auth.users', b.verified_by::text) AS verifiee_par,
       legacy.map('auth.users', b.created_by::text) AS creee_par,
       b.created_by AS v1_createur,
       b.created_at,
       b.updated_at,
       row_number() OVER (ORDER BY COALESCE(b.is_verified, false) DESC, b.created_at, b.id) AS rang
FROM brut b;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM o30 WHERE nom_norm IS NULL OR length(nom) < 2) THEN
        RAISE EXCEPTION 'Lot 30 : organisation v1 sans nom exploitable';
    END IF;
    IF EXISTS (SELECT 1 FROM o30 o WHERE NOT EXISTS (
            SELECT 1 FROM reference.taxonomy_terms t
            WHERE t.taxonomy_code = 'organization_type' AND t.code = o.type_code)) THEN
        RAISE EXCEPTION 'Lot 30 : type d''organisation v1 sans terme v2';
    END IF;
END $$;

-- Paires évidentes entre fiches v1.
CREATE TEMP TABLE paires_evidentes ON COMMIT DROP AS
SELECT a.v1_id AS a, b.v1_id AS b,
       CASE WHEN a.nom_norm = b.nom_norm THEN 'même nom normalisé, même pays'
            ELSE 'même sigle, même pays, noms à une faute près' END AS raison
FROM o30 a JOIN o30 b ON a.v1_id <> b.v1_id
 AND a.pays_id IS NOT DISTINCT FROM b.pays_id
 AND (a.nom_norm = b.nom_norm
      OR (a.sigle_norm = b.sigle_norm AND similarity(a.nom_norm, b.nom_norm) >= 0.9));

-- Composantes : chaque fiche prend le plus petit rang qu'elle atteint.
CREATE TEMP TABLE composantes ON COMMIT DROP AS
WITH RECURSIVE r(v1_id, rang) AS (
    SELECT v1_id, rang FROM o30
    UNION
    SELECT p.b, r.rang FROM r JOIN paires_evidentes p ON p.a = r.v1_id
)
SELECT v1_id, min(rang) AS racine FROM r GROUP BY v1_id;

-- Fiche déjà en base portant le même nom normalisé dans le même pays.
CREATE TEMP TABLE cibles_existantes ON COMMIT DROP AS
SELECT DISTINCT ON (c.racine) c.racine, x.id AS cible
FROM composantes c
JOIN o30 o ON o.v1_id = c.v1_id
JOIN org.organizations x ON x.status IN ('candidate', 'active')
 AND x.legal_name_normalized = o.nom_norm
 AND x.country_id IS NOT DISTINCT FROM o.pays_id
ORDER BY c.racine, x.created_at;

CREATE TEMP TABLE affectation ON COMMIT DROP AS
SELECT o.v1_id,
       o.rang = c.racine AND ce.cible IS NULL AS est_racine,
       c.racine,
       ce.cible AS cible_existante,
       CASE WHEN ce.cible IS NOT NULL THEN 'avec la fiche déjà en base : même nom normalisé, même pays'
            ELSE (SELECT p.raison FROM paires_evidentes p JOIN o30 r ON r.v1_id = p.b
                   WHERE p.a = o.v1_id ORDER BY r.rang LIMIT 1) END AS raison,
       CASE WHEN o.rang = c.racine AND ce.cible IS NULL THEN platform.uuid_v7() END AS nouvel_id
FROM o30 o
JOIN composantes c ON c.v1_id = o.v1_id
LEFT JOIN cibles_existantes ce ON ce.racine = c.racine;

ALTER TABLE affectation ADD COLUMN cible uuid;
UPDATE affectation a
   SET cible = COALESCE(a.cible_existante, (SELECT r.nouvel_id FROM affectation r
                                              WHERE r.racine = a.racine AND r.est_racine));

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM affectation WHERE cible IS NULL)
       OR EXISTS (SELECT 1 FROM affectation WHERE NOT est_racine AND raison IS NULL) THEN
        RAISE EXCEPTION 'Lot 30 : fusion sans cible ou sans raison';
    END IF;
END $$;

-- Adresse d'URL : celle du nom, suffixée en cas de collision (en base ou dans le lot).
CREATE TEMP TABLE slugs ON COMMIT DROP AS
WITH base AS (
    SELECT a.nouvel_id, rtrim(left(platform.slugify(o.nom), 150), '-') AS base, o.created_at
    FROM affectation a JOIN o30 o ON o.v1_id = a.v1_id
    WHERE a.est_racine
), rangs AS (
    SELECT b.*, row_number() OVER (PARTITION BY b.base ORDER BY b.created_at, b.nouvel_id)
              + CASE WHEN EXISTS (SELECT 1 FROM org.organizations x WHERE x.slug = b.base) THEN 1 ELSE 0 END AS n
    FROM base b
)
SELECT nouvel_id, CASE WHEN n = 1 THEN base ELSE base || '-' || n END AS slug FROM rangs;

INSERT INTO org.organizations
    (id, legal_name, acronym, slug, organization_type_code, country_id, description,
     website, contact_email, status, verified_at, verified_by, created_by, created_at, updated_at)
SELECT a.nouvel_id, o.nom, o.sigle, s.slug, o.type_code, o.pays_id,
       CASE WHEN o.description IS NOT NULL THEN jsonb_build_object('fr', o.description) END,
       o.site, o.courriel,
       CASE WHEN o.verifiee THEN 'active' ELSE 'candidate' END::org.organization_status,
       CASE WHEN o.verifiee THEN o.verifiee_le END,
       CASE WHEN o.verifiee THEN o.verifiee_par END,
       o.creee_par, o.created_at, o.updated_at
FROM affectation a
JOIN o30 o ON o.v1_id = a.v1_id
JOIN slugs s ON s.nouvel_id = a.nouvel_id
WHERE a.est_racine;

-- Nom légal et sigle des fiches fusionnées : dénominations de la cible, à confirmer
-- par un administrateur, pour qu'une recherche sur l'ancienne graphie la retrouve.
INSERT INTO org.organization_names (organization_id, name, kind, is_confirmed, created_at)
SELECT DISTINCT ON (a.cible, platform.normalize_label(v.nom), v.genre)
       a.cible, v.nom, v.genre, false, o.created_at
FROM affectation a
JOIN o30 o ON o.v1_id = a.v1_id
CROSS JOIN LATERAL (VALUES (o.nom, 'misspelling'::org.name_kind),
                           (o.sigle, 'acronym'::org.name_kind)) v(nom, genre)
WHERE NOT a.est_racine
  AND v.nom IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM org.organization_names n
                  WHERE n.organization_id = a.cible
                    AND n.name_normalized = platform.normalize_label(v.nom));

-- Domaine du courriel de contact, non vérifié : seulement s'il n'est pas générique
-- et désigne cette seule fiche, ou s'il est aussi celui de son site (un cabinet
-- dont l'adresse sert à quatre organisations n'en désigne aucune).
INSERT INTO org.organization_domains (organization_id, domain, created_at)
SELECT DISTINCT ON (a.cible, o.domaine) a.cible, o.domaine, o.created_at
FROM affectation a
JOIN o30 o ON o.v1_id = a.v1_id
WHERE o.courriel IS NOT NULL
  AND o.domaine ~ '^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$'
  AND o.domaine NOT IN (SELECT domain FROM domaines_publics)
  AND ((SELECT count(DISTINCT a2.cible) FROM affectation a2 JOIN o30 o2 ON o2.v1_id = a2.v1_id
         WHERE o2.domaine = o.domaine) = 1
       OR platform.extract_domain(o.site) = o.domaine)
  AND NOT EXISTS (SELECT 1 FROM org.organization_domains d
                  WHERE d.organization_id = a.cible AND d.domain = o.domaine)
ORDER BY a.cible, o.domaine, o.created_at;

-- Adhésions : users.organization_id. Le créateur de la fiche en devient référent.
-- Fonction exigée (ck_memberships_job_title) : celle de la personne (lot 20, tirée de
-- l'import Laravel sans les civilités), à défaut « Non renseignée » (D5).
CREATE TEMP TABLE adhesions ON COMMIT DROP AS
SELECT u.id AS v1_user,
       legacy.map('auth.users', u.id::text) AS personne,
       a.cible AS organisation,
       CASE WHEN o.v1_createur = u.id THEN 'manager' ELSE 'member' END::org.membership_role AS role,
       COALESCE((SELECT NULLIF(btrim(p.job_title), '') FROM identity.people p
                  WHERE p.id = legacy.map('auth.users', u.id::text)),
                'Non renseignée') AS fonction,
       GREATEST(u.created_at, o.created_at) AS depuis
FROM v1.users u
JOIN affectation a ON a.v1_id = u.organization_id
JOIN o30 o ON o.v1_id = u.organization_id;

CREATE TEMP TABLE adhesions_creees (organisation uuid, personne uuid) ON COMMIT DROP;
WITH creees AS (
    INSERT INTO org.memberships (organization_id, person_id, role, status, job_title, created_at, updated_at)
    SELECT DISTINCT ON (d.organisation, d.personne)
           d.organisation, d.personne, d.role, 'active', d.fonction, d.depuis, d.depuis
    FROM adhesions d
    WHERE d.personne IS NOT NULL
      -- Adhésion déjà en base (le commanditaire, gardé par la purge) : laissée telle quelle.
      AND NOT EXISTS (SELECT 1 FROM org.memberships m
                      WHERE m.organization_id = d.organisation AND m.person_id = d.personne)
    ORDER BY d.organisation, d.personne, d.role, d.depuis
    RETURNING organization_id, person_id
)
INSERT INTO adhesions_creees SELECT * FROM creees;

-- Score de confiance : recalculé par le worker (org.trust_score.recompute), comme
-- après tout geste de l'application.
INSERT INTO platform.jobs (task, payload, idempotency_key, run_at)
SELECT 'org.trust_score.recompute',
       jsonb_build_object('organization_id', c.cible),
       'org.trust_score.recompute:' || c.cible,
       now()
FROM (SELECT DISTINCT cible FROM affectation) c;

-- Correspondances.
SELECT count(*) AS consignees FROM (
SELECT legacy.remember(
    'public.organizations', a.v1_id::text, 'org', 'organizations', a.cible,
    CASE WHEN a.est_racine THEN 'migrated' ELSE 'merged' END,
    NULLIF(concat_ws(' ; ',
        CASE WHEN NOT a.est_racine THEN 'fusion évidente (' || a.raison || ')' END,
        CASE WHEN o.sigle_refuse THEN 'sigle d''un caractère mis à nul (2 à 32 exigés)' END,
        CASE WHEN o.site_refuse THEN 'site illisible mis à nul' END,
        CASE WHEN o.courriel_refuse THEN 'courriel de contact invalide mis à nul' END), ''))
FROM affectation a JOIN o30 o ON o.v1_id = a.v1_id
) x;

SELECT count(*) AS consignees FROM (
SELECT legacy.remember(
    'public.organization_validations', v.id::text, 'org', 'organization_endorsements', NULL,
    'skipped', 'attestation par des pairs, sans table v2 : phase 2, extension du modèle')
FROM v1.organization_validations v
) x;

-- Groupes probables mais incertains (D4), pour validation : même domaine non
-- générique, même sigle, ou noms proches, entre fiches que le lot n'a pas réunies,
-- fiches déjà en base comprises.
CREATE TABLE legacy.organisations_groupes (
    groupe        integer NOT NULL,
    fiche_v2      uuid    NOT NULL,
    fiches_v1     text,
    nom           text    NOT NULL,
    sigle         text,
    pays          text,
    verifiee      boolean NOT NULL,
    membres       integer NOT NULL,
    criteres      text    NOT NULL
);
COMMENT ON TABLE legacy.organisations_groupes IS
    'Reprise v1, lot 30 : doublons probables non fusionnés, à valider par le commanditaire (D4). Supprimée après la bascule.';

WITH fiches AS (
    SELECT x.id, x.legal_name_normalized AS nom_norm, x.acronym_normalized AS sigle_norm, x.country_id,
           x.website, x.legal_name, x.acronym, x.verified_at IS NOT NULL AS verifiee
    FROM org.organizations x
    WHERE x.status IN ('candidate', 'active')
), domaines AS (
    SELECT DISTINCT a.cible AS id, o.domaine
    FROM affectation a JOIN o30 o ON o.v1_id = a.v1_id
    WHERE o.courriel IS NOT NULL AND o.domaine NOT IN (SELECT domain FROM domaines_publics)
    UNION
    SELECT organization_id, domain FROM org.organization_domains
), paires AS (
    SELECT a.id AS a, b.id AS b,
           concat_ws(', ',
               CASE WHEN EXISTS (SELECT 1 FROM domaines da JOIN domaines db ON db.domaine = da.domaine
                                  WHERE da.id = a.id AND db.id = b.id) THEN 'même domaine' END,
               CASE WHEN a.sigle_norm = b.sigle_norm THEN 'même sigle' END,
               CASE WHEN similarity(a.nom_norm, b.nom_norm) >= 0.6
                    THEN 'noms proches (' || round(similarity(a.nom_norm, b.nom_norm)::numeric, 2) || ')' END,
               CASE WHEN a.country_id = b.country_id THEN 'même pays' END) AS criteres
    FROM fiches a JOIN fiches b ON a.id < b.id
    WHERE (a.id IN (SELECT cible FROM affectation) OR b.id IN (SELECT cible FROM affectation))
      AND ((a.sigle_norm = b.sigle_norm AND a.country_id = b.country_id)
           OR (similarity(a.nom_norm, b.nom_norm) >= 0.6 AND a.country_id = b.country_id)
           OR similarity(a.nom_norm, b.nom_norm) >= 0.85
           OR EXISTS (SELECT 1 FROM domaines da JOIN domaines db ON db.domaine = da.domaine
                       WHERE da.id = a.id AND db.id = b.id))
), sym AS (
    SELECT a, b FROM paires UNION SELECT b, a FROM paires
), comp AS (
    WITH RECURSIVE r(id, racine) AS (
        SELECT a, a FROM sym
        UNION
        SELECT s.b, r.racine FROM r JOIN sym s ON s.a = r.id
    )
    SELECT id, min(racine::text) AS racine FROM r GROUP BY id
)
INSERT INTO legacy.organisations_groupes
SELECT dense_rank() OVER (ORDER BY c.racine),
       f.id,
       (SELECT string_agg(a.v1_id::text, ' ' ORDER BY a.v1_id) FROM affectation a WHERE a.cible = f.id),
       f.legal_name, f.acronym,
       (SELECT r.iso2 FROM reference.countries r WHERE r.id = f.country_id),
       f.verifiee,
       (SELECT count(*) FROM org.memberships m WHERE m.organization_id = f.id AND m.status = 'active'),
       (SELECT string_agg(DISTINCT p.criteres, ' | ') FROM paires p WHERE p.a = f.id OR p.b = f.id)
FROM comp c JOIN fiches f ON f.id = c.id;

-- Compte rendu.
DO $$
DECLARE
    v_table text;
    v_extraites bigint;
    v_rendues bigint;
    v_adh_v1 bigint;
    v_adh_creees bigint;
    v_adh_deja bigint;
    v_adh_sans_personne bigint;
BEGIN
    FOREACH v_table IN ARRAY ARRAY['organizations', 'organization_validations'] LOOP
        EXECUTE format('SELECT count(*) FROM v1.%I', v_table) INTO v_extraites;
        SELECT count(*) INTO v_rendues FROM legacy.id_map WHERE legacy_table = 'public.' || v_table;
        IF v_extraites <> v_rendues THEN
            RAISE EXCEPTION 'Lot 30 : public.% extraites %, consignées %', v_table, v_extraites, v_rendues;
        END IF;
        RAISE NOTICE 'public.% : % extraites, % reprises, % fusionnées, % écartées', v_table, v_extraites,
            (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'public.' || v_table AND disposition = 'migrated'),
            (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'public.' || v_table AND disposition = 'merged'),
            (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'public.' || v_table AND disposition = 'skipped');
    END LOOP;

    SELECT count(*) INTO v_adh_v1 FROM adhesions;
    SELECT count(*) INTO v_adh_sans_personne FROM adhesions WHERE personne IS NULL;
    SELECT count(*) INTO v_adh_creees FROM adhesions_creees;
    SELECT count(*) - v_adh_creees INTO v_adh_deja
      FROM (SELECT DISTINCT organisation, personne FROM adhesions WHERE personne IS NOT NULL) d;
    IF EXISTS (SELECT 1 FROM adhesions d WHERE d.personne IS NOT NULL AND NOT EXISTS (
                   SELECT 1 FROM org.memberships m
                   WHERE m.organization_id = d.organisation AND m.person_id = d.personne)) THEN
        RAISE EXCEPTION 'Lot 30 : adhésions non rendues';
    END IF;
    RAISE NOTICE 'adhésions v1 : %, créées %, déjà en base %, personne non reprise %, doublons de personne %',
        v_adh_v1, v_adh_creees, v_adh_deja, v_adh_sans_personne,
        v_adh_v1 - v_adh_sans_personne - v_adh_creees - v_adh_deja;
    RAISE NOTICE 'dont fonction « Non renseignée » : %',
        (SELECT count(*) FROM adhesions WHERE personne IS NOT NULL AND fonction = 'Non renseignée');
    RAISE NOTICE 'domaines posés : %, dénominations ajoutées par fusion : %, groupes incertains : % (% fiches)',
        (SELECT count(*) FROM org.organization_domains d WHERE d.organization_id IN (SELECT cible FROM affectation)
            AND d.verified_at IS NULL),
        (SELECT count(*) FROM org.organization_names n WHERE n.organization_id IN (SELECT cible FROM affectation)
            AND NOT n.is_confirmed),
        (SELECT count(DISTINCT groupe) FROM legacy.organisations_groupes),
        (SELECT count(*) FROM legacy.organisations_groupes);
END $$;

COMMIT;
