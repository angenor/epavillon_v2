-- Lot 20 : personnes, comptes, rôles.
--
-- Sources : auth.users + public.users (mêmes identifiants), public.migration_logs
-- (comptes importés de Laravel), auth.identities, public.user_roles.
-- Règles : carte du cœur §§ 1 et 2, D1 révisé, vérification Laravel (ecarts.md).
--
-- - Compte `password` sans empreinte, `password_reset_required_at` posé (D1).
-- - Vérification : auth.users.email_confirmed_at, sauf import Laravel (is_migrate),
--   daté par old_user_data->>'email_verified_at' et non par l'import.
-- - Personne déjà en production (le commanditaire) : `merged` par adresse, rien de
--   créé ni modifié, son mot de passe compris.
-- - Fonction, civilité, téléphone, biographie : public.users, à défaut l'import
--   Laravel ; son `titre` mêle civilités et fonctions, d'où le tri ci-dessous.
-- - Rôles : admin, super_admin, revisionniste (reviewer), negotiator, en portée
--   globale ; paco jamais (D9) ; désactivés ou échus écartés.
-- - Phase 2 (pas de place dans le modèle) : address, display_mode. Ailleurs :
--   photo (lot 60), organisation et primary_organization_id (lot 30),
--   notification_preferences (annexes).
--
-- Variable `paco` : comptes nés du formulaire PACO (raw_user_meta_data.source).
--   tous     (défaut, commanditaire 27/10) : les 639 repris, vérifiés ou non, sans rôle
--   verifies : jamais vérifiés écartés, ils restent dans Supabase
--   aucun    : tous écartés
--   psql -v paco=verifies -f 20-personnes-comptes-roles.sql
--
-- Relecture des rôles repris (rôles v1 falsifiables jusqu'au 27/10), après le lot :
--   SELECT ra.role_code, p.display_name, p.primary_email, ra.granted_at, m.disposition
--   FROM legacy.id_map m JOIN identity.role_assignments ra ON ra.id = m.target_id
--   JOIN identity.people p ON p.id = ra.person_id
--   WHERE m.legacy_table = 'public.user_roles' ORDER BY 1, 2;
-- vers sauvegardes/reprise-v1/roles-a-relire.txt (hors dépôt).

\set ON_ERROR_STOP on
\if :{?paco}
\else
\set paco tous
\endif

BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-20', true),
       set_config('reprise.paco', :'paco', true) \gset lot20_

DO $$
BEGIN
    IF current_setting('reprise.paco') NOT IN ('tous', 'verifies', 'aucun') THEN
        RAISE EXCEPTION 'lot 20 : paco = « % », attendu tous, verifies ou aucun', current_setting('reprise.paco');
    END IF;
    IF EXISTS (SELECT 1 FROM v1_auth.users a FULL JOIN v1.users u ON u.id = a.id
               WHERE a.id IS NULL OR u.id IS NULL) THEN
        RAISE EXCEPTION 'lot 20 : auth.users et public.users ne se correspondent plus un pour un';
    END IF;
    -- Relevés à zéro le 27/10 : un cas nouveau demande une règle écrite.
    IF EXISTS (SELECT 1 FROM v1.users WHERE is_blocked OR is_suspended)
       OR EXISTS (SELECT 1 FROM v1_auth.users WHERE deleted_at IS NOT NULL OR banned_until IS NOT NULL) THEN
        RAISE EXCEPTION 'lot 20 : compte bloqué, suspendu ou supprimé dans la v1, règle à écrire';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM legacy.id_map WHERE legacy_table = 'public.countries') THEN
        RAISE EXCEPTION 'lot 20 : le lot 10 (pays) n''a pas été joué';
    END IF;
END $$;

-- Personnes ------------------------------------------------------------------

CREATE TEMP TABLE lot20_personnes ON COMMIT DROP AS
SELECT s.*,
       CASE s.disposition WHEN 'merged' THEN s.existante
                          WHEN 'migrated' THEN platform.uuid_v7() END AS person_id
FROM (
    SELECT a.id AS v1_id,
           a.email_confirmed_at,
           a.last_sign_in_at,
           a.created_at AS compte_cree_le,
           a.updated_at AS compte_modifie_le,
           u.is_migrate AS est_laravel,
           coalesce(a.raw_user_meta_data ->> 'source' = 'paco', false) AS est_paco,
           ml.id AS journal_id,
           ml.old_user_id AS laravel_id,
           ml.old_user_data AS laravel,
           p.id AS existante,
           CASE
               WHEN p.id IS NOT NULL THEN 'merged'
               WHEN coalesce(a.raw_user_meta_data ->> 'source' = 'paco', false)
                    AND (current_setting('reprise.paco') = 'aucun'
                         OR (current_setting('reprise.paco') = 'verifies' AND a.email_confirmed_at IS NULL))
                   THEN 'skipped'
               ELSE 'migrated'
           END AS disposition
    FROM v1_auth.users a
    JOIN v1.users u ON u.id = a.id
    LEFT JOIN v1.migration_logs ml ON ml.new_user_id = a.id
    LEFT JOIN identity.people p
           ON p.primary_email = lower(btrim(a.email))::platform.email AND p.status <> 'anonymized'
) AS s;

ALTER TABLE lot20_personnes ADD COLUMN note text;
UPDATE lot20_personnes SET note = CASE
    WHEN disposition = 'merged'
        THEN 'personne déjà en production, rapprochée par adresse : rien de créé ni modifié'
    WHEN disposition = 'skipped' AND current_setting('reprise.paco') = 'aucun'
        THEN 'compte né du formulaire PACO, non repris (D9)'
    WHEN disposition = 'skipped'
        THEN 'compte PACO jamais vérifié, reste dans Supabase (D9)'
    WHEN est_laravel AND laravel IS NOT NULL
        THEN 'import Laravel : vérification datée de Laravel, pas de l''import'
    WHEN laravel IS NOT NULL
        THEN 'compte Supabase complété par l''import Laravel : vérification Supabase'
    WHEN est_paco AND email_confirmed_at IS NULL
        THEN 'compte né du formulaire PACO, jamais vérifié, repris sans rôle'
    WHEN est_paco
        THEN 'compte né du formulaire PACO, repris sans rôle'
END;

INSERT INTO identity.people (id, primary_email, email_verified_at, first_name, last_name,
                             civility, phone, job_title, biography, country_id,
                             is_directory_visible, status, created_at, updated_at)
SELECT l.person_id,
       lower(btrim(a.email))::platform.email,
       CASE WHEN l.est_laravel AND l.laravel IS NOT NULL
            THEN (l.laravel ->> 'email_verified_at')::timestamptz
            ELSE l.email_confirmed_at END,
       btrim(u.first_name),
       btrim(u.last_name),
       t.civilite,
       coalesce(nullif(btrim(u.phone), ''), nullif(btrim(l.laravel ->> 'telephone'), '')),
       CASE WHEN t.civilite IS NULL AND t.titre ~ '[[:alpha:]].*[[:alpha:]]' THEN t.titre END,
       CASE WHEN b.bio IS NOT NULL THEN jsonb_build_object('fr', b.bio)::platform.i18n_text END,
       legacy.map('public.countries', u.country_id::text),
       coalesce(u.networking_visibility, true),
       'active',
       coalesce(u.created_at, a.created_at),
       coalesce(u.updated_at, u.created_at, a.created_at)
FROM lot20_personnes l
JOIN v1_auth.users a ON a.id = l.v1_id
JOIN v1.users u ON u.id = l.v1_id
CROSS JOIN LATERAL (SELECT nullif(btrim(l.laravel ->> 'titre'), '') AS titre) t0
CROSS JOIN LATERAL (
    SELECT t0.titre,
           CASE lower(rtrim(t0.titre, '.'))
               WHEN 'm' THEN 'm' WHEN 'mr' THEN 'm' WHEN 'monsieur' THEN 'm' WHEN 'mister' THEN 'm'
               WHEN 'mme' THEN 'mme' WHEN 'madame' THEN 'mme' WHEN 'mlle' THEN 'mme'
               WHEN 'mademoiselle' THEN 'mme' WHEN 'ms' THEN 'mme' WHEN 'mrs' THEN 'mme' WHEN 'miss' THEN 'mme'
               WHEN 'dr' THEN 'dr' WHEN 'docteur' THEN 'dr' WHEN 'dra' THEN 'dr' WHEN 'drs' THEN 'dr'
               WHEN 'pr' THEN 'pr' WHEN 'prof' THEN 'pr' WHEN 'professeur' THEN 'pr'
           END AS civilite
) t
CROSS JOIN LATERAL (
    SELECT coalesce(nullif(btrim(u.biography), ''), nullif(btrim(l.laravel ->> 'biographie'), '')) AS bio
) b
WHERE l.disposition = 'migrated';

-- Adresse professionnelle distincte : adresse secondaire, servant au rapprochement.
INSERT INTO identity.person_emails (person_id, email, label, created_at)
SELECT l.person_id, lower(btrim(u.email_pro))::platform.email, 'professional',
       coalesce(u.created_at, now())
FROM lot20_personnes l
JOIN v1.users u ON u.id = l.v1_id
WHERE l.disposition = 'migrated'
  AND nullif(btrim(u.email_pro), '') IS NOT NULL
  AND lower(btrim(u.email_pro)) <> lower(btrim(u.email))
  AND btrim(u.email_pro) ~ '^[^@\s]+@[^@\s]+\.[^@\s]{2,}$';

-- Comptes --------------------------------------------------------------------

INSERT INTO identity.accounts (person_id, provider, password_hash, password_reset_required_at,
                               last_login_at, created_at, updated_at)
SELECT l.person_id, 'password', NULL, now(),
       l.last_sign_in_at, l.compte_cree_le, coalesce(l.compte_modifie_le, l.compte_cree_le)
FROM lot20_personnes l
WHERE l.disposition = 'migrated';

-- Correspondance des comptes et profils ----------------------------------------

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'auth.users', l.v1_id::text, 'identity', 'people', l.person_id, l.disposition, l.note
FROM lot20_personnes l;

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'public.users', l.v1_id::text, 'identity', 'people', l.person_id,
       CASE WHEN l.disposition = 'skipped' THEN 'skipped' ELSE 'merged' END,
       CASE WHEN l.disposition = 'skipped' THEN l.note
            ELSE 'profil v1, même personne que auth.users' END
FROM lot20_personnes l;

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'auth.identities', i.id::text, 'identity',
       CASE WHEN l.disposition = 'merged' AND ac.id IS NULL THEN 'people' ELSE 'accounts' END,
       CASE WHEN l.disposition = 'skipped' THEN NULL
            WHEN l.disposition = 'merged' AND ac.id IS NULL THEN l.person_id
            ELSE ac.id END,
       l.disposition,
       CASE l.disposition
           WHEN 'skipped' THEN l.note
           WHEN 'merged'  THEN 'compte de production conservé, mot de passe inchangé'
       END
FROM v1_auth.identities i
JOIN lot20_personnes l ON l.v1_id = i.user_id
LEFT JOIN identity.accounts ac ON ac.person_id = l.person_id AND ac.provider = 'password';

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'public.migration_logs', ml.id::text, 'identity', 'people', l.person_id,
       CASE WHEN l.person_id IS NULL THEN 'skipped' ELSE 'merged' END,
       CASE WHEN l.person_id IS NULL THEN coalesce(l.note, 'compte importé absent de auth.users')
            ELSE 'journal de l''import Laravel (' || ml.action || ')' END
FROM v1.migration_logs ml
LEFT JOIN lot20_personnes l ON l.v1_id = ml.new_user_id;

-- Lien vers l'identifiant Laravel, pour la reprise des années 2021 à 2024.
INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'laravel.users', ml.old_user_id::text, 'identity', 'people', l.person_id,
       CASE WHEN l.person_id IS NULL THEN 'skipped' ELSE 'merged' END,
       CASE WHEN l.person_id IS NULL THEN coalesce(l.note, 'compte importé absent de auth.users')
            ELSE 'compte Laravel importé dans Supabase en 2025' END
FROM v1.migration_logs ml
LEFT JOIN lot20_personnes l ON l.v1_id = ml.new_user_id;

-- Rôles ----------------------------------------------------------------------

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM v1.user_roles
               WHERE role::text NOT IN ('standard', 'unfccc_focal_point', 'negotiator', 'trainer',
                                        'admin', 'super_admin', 'paco', 'revisionniste')) THEN
        RAISE EXCEPTION 'lot 20 : rôle v1 inconnu, correspondance à écrire';
    END IF;
END $$;

CREATE TEMP TABLE lot20_roles ON COMMIT DROP AS
SELECT r.id AS v1_id, r.role::text AS role_v1, r.assigned_at, r.valid_until, r.assigned_by,
       l.person_id,
       CASE r.role::text
           WHEN 'revisionniste' THEN 'reviewer' WHEN 'admin' THEN 'admin'
           WHEN 'super_admin' THEN 'super_admin' WHEN 'negotiator' THEN 'negotiator'
           WHEN 'trainer' THEN 'trainer'
       END AS role_v2,
       CASE
           WHEN l.person_id IS NULL THEN 'personne non reprise : ' || coalesce(l.note, 'absente de auth.users')
           WHEN r.role::text = 'paco' THEN 'rôle PACO non repris (D9)'
           WHEN r.role::text = 'standard' THEN 'rôle implicite en v2, aucune attribution'
           WHEN r.role::text = 'unfccc_focal_point' THEN 'point focal CCNUCC : profil négociateur, phase 2'
           WHEN NOT coalesce(r.is_active, true)
               THEN 'rôle désactivé dans la v1' || coalesce(', échu le ' || to_char(r.valid_until AT TIME ZONE 'UTC', 'YYYY-MM-DD'), '')
           WHEN r.valid_until <= now()
               THEN 'rôle échu le ' || to_char(r.valid_until AT TIME ZONE 'UTC', 'YYYY-MM-DD')
       END AS ecart
FROM v1.user_roles r
LEFT JOIN lot20_personnes l ON l.v1_id = r.user_id;

ALTER TABLE lot20_roles ADD COLUMN existante uuid, ADD COLUMN rang bigint, ADD COLUMN cible uuid;

UPDATE lot20_roles x SET existante = ra.id
FROM identity.role_assignments ra
WHERE x.ecart IS NULL AND ra.person_id = x.person_id AND ra.role_code = x.role_v2
  AND ra.scope_type = 'global' AND ra.revoked_at IS NULL;

UPDATE lot20_roles x SET rang = o.rang
FROM (SELECT v1_id, row_number() OVER (PARTITION BY person_id, role_v2 ORDER BY assigned_at, v1_id) AS rang
      FROM lot20_roles WHERE ecart IS NULL AND existante IS NULL) o
WHERE o.v1_id = x.v1_id;

UPDATE lot20_roles SET cible = platform.uuid_v7() WHERE rang = 1;
UPDATE lot20_roles x SET cible = premier.cible
FROM lot20_roles premier
WHERE x.rang > 1 AND premier.rang = 1
  AND premier.person_id = x.person_id AND premier.role_v2 = x.role_v2;

INSERT INTO identity.role_assignments (id, person_id, role_code, scope_type, scope_id,
                                       granted_by, granted_at, valid_from, valid_until, note)
SELECT x.cible, x.person_id, x.role_v2, 'global', NULL,
       legacy.map('auth.users', x.assigned_by::text),
       coalesce(x.assigned_at, now()), coalesce(x.assigned_at, now()), x.valid_until,
       'Repris de la v1 (rôle « ' || x.role_v1 || ' »)'
FROM lot20_roles x
WHERE x.rang = 1;

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table,
                           target_id, disposition, note)
SELECT 'public.user_roles', x.v1_id::text, 'identity', 'role_assignments',
       coalesce(x.existante, x.cible),
       CASE WHEN x.ecart IS NOT NULL THEN 'skipped'
            WHEN x.existante IS NOT NULL OR x.rang > 1 THEN 'merged'
            ELSE 'migrated' END,
       CASE WHEN x.ecart IS NOT NULL THEN x.ecart
            WHEN x.existante IS NOT NULL THEN 'même rôle déjà attribué en production'
            WHEN x.rang > 1 THEN 'même rôle attribué deux fois dans la v1'
            WHEN x.role_v1 = 'revisionniste' THEN 'revisionniste v1 = reviewer v2, portée globale'
       END
FROM lot20_roles x;

-- Compte rendu ---------------------------------------------------------------

DO $$
DECLARE
    r record;
BEGIN
    FOR r IN
        SELECT s.t, s.extraites,
               count(m.*) FILTER (WHERE m.disposition = 'migrated') AS migrated,
               count(m.*) FILTER (WHERE m.disposition = 'merged')   AS merged,
               count(m.*) FILTER (WHERE m.disposition = 'skipped')  AS skipped
        FROM (VALUES
            ('auth.users',            (SELECT count(*) FROM v1_auth.users)),
            ('auth.identities',       (SELECT count(*) FROM v1_auth.identities)),
            ('public.users',          (SELECT count(*) FROM v1.users)),
            ('public.migration_logs', (SELECT count(*) FROM v1.migration_logs)),
            ('laravel.users',         (SELECT count(DISTINCT old_user_id) FROM v1.migration_logs)),
            ('public.user_roles',     (SELECT count(*) FROM v1.user_roles))
        ) AS s(t, extraites)
        LEFT JOIN legacy.id_map m ON m.legacy_table = s.t
        GROUP BY s.t, s.extraites
        ORDER BY s.t
    LOOP
        RAISE NOTICE 'lot 20 : % extraites %, migrated %, merged %, skipped %',
            r.t, r.extraites, r.migrated, r.merged, r.skipped;
        IF r.extraites <> r.migrated + r.merged + r.skipped THEN
            RAISE EXCEPTION 'lot 20 : % ne rend pas compte de toutes ses lignes', r.t;
        END IF;
    END LOOP;

    IF (SELECT count(*) FROM identity.people p JOIN lot20_personnes l ON l.person_id = p.id
        WHERE l.disposition = 'migrated')
       <> (SELECT count(*) FROM lot20_personnes WHERE disposition = 'migrated')
       OR (SELECT count(*) FROM identity.accounts ac JOIN lot20_personnes l ON l.person_id = ac.person_id
           WHERE l.disposition = 'migrated' AND ac.password_hash IS NULL
             AND ac.password_reset_required_at IS NOT NULL)
       <> (SELECT count(*) FROM lot20_personnes WHERE disposition = 'migrated') THEN
        RAISE EXCEPTION 'lot 20 : personnes ou comptes créés en nombre inattendu';
    END IF;

    RAISE NOTICE 'lot 20 : adresses pro écartées (forme invalide) %',
        (SELECT count(*) FROM lot20_personnes l JOIN v1.users u ON u.id = l.v1_id
          WHERE l.disposition = 'migrated' AND nullif(btrim(u.email_pro), '') IS NOT NULL
            AND lower(btrim(u.email_pro)) <> lower(btrim(u.email))
            AND btrim(u.email_pro) !~ '^[^@\s]+@[^@\s]+\.[^@\s]{2,}$');
    RAISE NOTICE 'lot 20 : personnes créées %, dont vérifiées % (Laravel %), PACO reprises %, adresses pro %, rôles attribués %',
        (SELECT count(*) FROM lot20_personnes WHERE disposition = 'migrated'),
        (SELECT count(*) FROM identity.people p JOIN lot20_personnes l ON l.person_id = p.id
          WHERE l.disposition = 'migrated' AND p.email_verified_at IS NOT NULL),
        (SELECT count(*) FROM lot20_personnes WHERE disposition = 'migrated' AND est_laravel AND laravel IS NOT NULL),
        (SELECT count(*) FROM lot20_personnes WHERE disposition = 'migrated' AND est_paco),
        (SELECT count(*) FROM identity.person_emails e JOIN lot20_personnes l ON l.person_id = e.person_id
          WHERE l.disposition = 'migrated'),
        (SELECT count(*) FROM lot20_roles WHERE rang = 1);
END $$;

COMMIT;
