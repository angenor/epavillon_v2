-- Purge des données d'essai de la production v2 (D2, plan § 5) : tout ce que
-- l'usage a créé, sauf la personne du commanditaire, son compte et ses rôles.
-- Reste ce que le schéma a semé ou que l'administration règle.
--
-- Liste fermée, établie le 27/10 en comparant la copie de production du jour à
-- une base neuve chargée depuis docs/database/ : chaque table non vide est
-- classée ici (purgée, partielle ou gardée), sinon le script s'arrête avant
-- d'effacer quoi que ce soit. Une table vide n'a pas besoin d'être classée.
--
-- Les objets de Garage ne s'effacent pas en SQL : leurs clés sont relevées dans
-- legacy.garage_a_purger avant la purge, pour l'étape à part de la bascule.
--
-- Les déclencheurs sont coupés pendant la purge (session_replication_role) :
-- ni audit, ni outbox, ni notification. Les cascades de clés étrangères le sont
-- aussi, d'où les suppressions explicites et le contrôle des orphelins à la fin.

\set ON_ERROR_STOP on
BEGIN;
DO $$ BEGIN PERFORM set_config('app.request_id', 'reprise-v1-purge', true); END $$;

CREATE TEMP TABLE classement (
    nom    text PRIMARY KEY,
    sort   text NOT NULL CHECK (sort IN ('purgee', 'partielle', 'gardee')),
    raison text NOT NULL
) ON COMMIT DROP;

INSERT INTO classement (nom, sort, raison) VALUES
    -- Créé par l'usage : effacé en entier.
    ('analytics.refresh_log',               'purgee', 'journal des rafraîchissements de projections'),
    ('content.highlights',                  'purgee', 'contenus mis en avant d''essai'),
    ('engagement.email_messages',           'purgee', 'historique des courriels'),
    ('engagement.notifications',            'purgee', 'notifications'),
    ('event.calls_for_proposals',           'purgee', 'appel de l''édition d''essai'),
    ('event.event_days',                    'purgee', 'journées de l''édition d''essai'),
    ('event.events',                        'purgee', 'édition COP31 d''essai (la v1 fait foi)'),
    ('event.rooms',                         'purgee', 'salle de l''édition d''essai'),
    ('event.venues',                        'purgee', 'lieu de l''édition d''essai'),
    ('identity.sessions',                   'purgee', 'sessions de connexion, commanditaire compris : il se reconnecte'),
    ('media.assets',                        'purgee', 'médias d''essai (objets Garage relevés à part)'),
    ('media.attachments',                   'purgee', 'rattachements des médias d''essai'),
    ('media.renditions',                    'purgee', 'déclinaisons des médias d''essai'),
    ('negotiation.document_pages',          'purgee', 'Guide Négo : pages du document d''exemple'),
    ('negotiation.document_renditions',     'purgee', 'Guide Négo : rendus du document d''exemple'),
    ('negotiation.documents',               'purgee', 'Guide Négo : document d''exemple'),
    ('negotiation.faq_entries',             'purgee', 'Guide Négo : FAQ d''exemple'),
    ('negotiation.faq_feedback',            'purgee', 'Guide Négo : avis sur la FAQ'),
    ('negotiation.faq_reads',               'purgee', 'Guide Négo : lectures de la FAQ'),
    ('negotiation.glossary_entries',        'purgee', 'Guide Négo : glossaire d''exemple'),
    ('negotiation.invitation_code_attempts','purgee', 'Guide Négo : essais de code'),
    ('negotiation.invitation_code_uses',    'purgee', 'Guide Négo : usages de code'),
    ('negotiation.invitation_codes',        'purgee', 'Guide Négo : code d''invitation d''essai'),
    ('negotiation.knowledge_sources',       'purgee', 'Guide Négo : sources d''exemple'),
    ('negotiation.network_memberships',     'purgee', 'Guide Négo : adhésion au réseau d''essai'),
    ('negotiation.pathway_checks',          'purgee', 'Guide Négo : parcours d''exemple'),
    ('negotiation.pathway_groups',          'purgee', 'Guide Négo : parcours d''exemple'),
    ('negotiation.pathway_steps',           'purgee', 'Guide Négo : parcours d''exemple'),
    ('org.duplicate_candidates',            'purgee', 'doublons proposés entre fiches d''essai'),
    ('platform.audit_log',                  'purgee', 'historique des actions sur les données d''essai'),
    ('platform.inbox_events',               'purgee', 'historique des travaux'),
    ('platform.jobs',                       'purgee', 'historique des travaux'),
    ('platform.outbox_events',              'purgee', 'historique de l''outbox'),
    ('programme.proposal_organizations',    'purgee', 'propositions d''essai'),
    ('programme.proposal_reads',            'purgee', 'propositions d''essai'),
    ('programme.proposal_speakers',         'purgee', 'propositions d''essai'),
    ('programme.proposal_transitions',      'purgee', 'propositions d''essai'),
    ('programme.proposals',                 'purgee', 'propositions d''essai'),
    ('programme.registrations',             'purgee', 'inscription d''essai'),
    ('programme.session_organizations',     'purgee', 'séances d''essai'),
    ('programme.session_questions',         'purgee', 'séances d''essai'),
    ('programme.session_speakers',          'purgee', 'séances d''essai'),
    ('programme.sessions',                  'purgee', 'séances d''essai'),

    -- Mêlé : on garde le semé et le commanditaire.
    ('identity.people',                     'partielle', 'gardés : le commanditaire et le compte d''administration semé par 900_seed'),
    ('identity.accounts',                   'partielle', 'gardé : celui du commanditaire'),
    ('identity.role_assignments',           'partielle', 'gardés : ceux des personnes gardées, dont la portée existe encore'),
    ('org.organizations',                   'partielle', 'gardée : l''IFDD, semée'),
    ('org.organization_names',              'partielle', 'gardées : celles de l''IFDD, semées'),
    ('org.memberships',                     'partielle', 'gardées : celles des personnes gardées à l''IFDD'),
    ('media.storage_quotas',                'partielle', 'gardé : le quota par défaut, semé ; ceux des organisations naissent du premier dépôt'),
    ('reference.entity_terms',              'partielle', 'gardés : ceux dont l''entité existe encore'),

    -- Semé par le schéma ou réglé par l'administration : intact.
    ('engagement.commentable_subjects',     'gardee', 'semé'),
    ('engagement.notification_types',       'gardee', 'semé'),
    ('event.broadcast_channels',            'gardee', 'semé : canal principal de l''IFDD, sans événement'),
    ('event.event_series',                  'gardee', 'séries semées'),
    ('identity.permissions',                'gardee', 'permissions'),
    ('identity.role_permissions',           'gardee', 'permissions des rôles'),
    ('identity.roles',                      'gardee', 'rôles'),
    ('media.asset_references',              'gardee', 'registre semé'),
    ('media.attachable_roles',              'gardee', 'registre semé'),
    ('negotiation.spaces',                  'gardee', 'les trois espaces, semés'),
    ('org.organization_domains',            'gardee', 'domaines de l''IFDD, semés'),
    ('org.organization_references',         'gardee', 'registre semé'),
    ('org.public_email_domains',            'gardee', 'référentiel semé'),
    ('platform.feature_flags',              'gardee', 'drapeaux, réglés par l''administration'),
    ('platform.modules',                    'gardee', 'semé'),
    ('platform.settings',                   'gardee', 'réglages de l''administration'),
    ('programme.proposal_transitions_allowed','gardee', 'machine à états, semée'),
    ('programme.registration_form_fields',  'gardee', 'formulaire par défaut, semé'),
    ('programme.registration_forms',        'gardee', 'formulaire par défaut, semé'),
    ('publication.publishing_policies',     'gardee', 'semé'),
    ('reference.countries',                 'gardee', 'référentiel'),
    ('reference.locales',                   'gardee', 'référentiel'),
    ('reference.taxonomies',                'gardee', 'taxonomies'),
    ('reference.taxonomy_terms',            'gardee', 'termes, réglés par l''administration'),
    ('tool.assistants',                     'gardee', 'semé'),
    ('tool.usage_quotas',                   'gardee', 'quota par défaut, semé');

-- 1. Garde : aucune table non vide hors classement, aucune table classée disparue.
DO $$
DECLARE
    t record;
    n bigint;
    hors_liste text[] := '{}';
    absentes text;
BEGIN
    FOR t IN
        SELECT ns.nspname || '.' || c.relname AS nom
        FROM pg_class c JOIN pg_namespace ns ON ns.oid = c.relnamespace
        WHERE c.relkind IN ('r', 'p') AND NOT c.relispartition
          AND ns.nspname NOT IN ('public', 'legacy', 'v1', 'v1_auth', 'v1_storage', 'information_schema')
          AND ns.nspname NOT LIKE 'pg\_%'
    LOOP
        CONTINUE WHEN EXISTS (SELECT 1 FROM classement WHERE nom = t.nom);
        EXECUTE format('SELECT count(*) FROM %s', t.nom) INTO n;
        IF n > 0 THEN
            hors_liste := hors_liste || format('%s (%s lignes)', t.nom, n);
        END IF;
    END LOOP;
    IF cardinality(hors_liste) > 0 THEN
        RAISE EXCEPTION 'Tables non vides non classées, à purger ou garder avec leur raison : %',
            array_to_string(hors_liste, ', ');
    END IF;

    SELECT string_agg(nom, ', ') INTO absentes
    FROM classement WHERE to_regclass(nom) IS NULL;
    IF absentes IS NOT NULL THEN
        RAISE EXCEPTION 'Tables classées absentes du schéma : %', absentes;
    END IF;
END $$;

-- 2. Ce qu'on garde.
CREATE TEMP TABLE personnes_gardees ON COMMIT DROP AS
SELECT id FROM identity.people
WHERE primary_email IN ('angenor99@gmail.com', 'admin@epavillonclimatique.francophonie.org');

DO $$
BEGIN
    IF (SELECT count(*) FROM identity.people WHERE primary_email = 'angenor99@gmail.com') <> 1 THEN
        RAISE EXCEPTION 'Personne du commanditaire introuvable ou en double : purge arrêtée.';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM org.organizations WHERE slug = 'ifdd') THEN
        RAISE EXCEPTION 'Fiche de l''IFDD introuvable : purge arrêtée.';
    END IF;
END $$;

-- 3. Objets de Garage à effacer à la bascule.
SET LOCAL client_min_messages = warning;
DROP TABLE IF EXISTS legacy.garage_a_purger;
CREATE TABLE legacy.garage_a_purger AS
SELECT a.bucket, a.object_key, 'media.assets'::text AS source FROM media.assets a
UNION ALL
SELECT a.bucket, r.object_key, 'media.renditions' FROM media.renditions r JOIN media.assets a ON a.id = r.asset_id
UNION ALL
SELECT a.bucket, p.image_key, 'negotiation.document_pages'
FROM negotiation.document_pages p
JOIN negotiation.documents d ON d.id = p.document_id
JOIN media.assets a ON a.id = d.asset_id
WHERE p.image_key IS NOT NULL;
COMMENT ON TABLE legacy.garage_a_purger IS
    'Objets Garage des médias d''essai effacés par ops/reprise-v1/03-purger-essais.sql : à supprimer du stockage à la bascule.';

-- 4. Purge, déclencheurs coupés.
SET LOCAL session_replication_role = replica;

DO $$
DECLARE t record;
BEGIN
    FOR t IN SELECT nom FROM classement WHERE sort = 'purgee' LOOP
        EXECUTE format('DELETE FROM %s', t.nom);
    END LOOP;
END $$;

DELETE FROM identity.accounts a
WHERE a.person_id NOT IN (SELECT id FROM identity.people WHERE primary_email = 'angenor99@gmail.com');

DELETE FROM org.memberships m
WHERE m.person_id NOT IN (SELECT id FROM personnes_gardees)
   OR m.organization_id NOT IN (SELECT id FROM org.organizations WHERE slug = 'ifdd');

DELETE FROM org.organization_names n
WHERE n.organization_id NOT IN (SELECT id FROM org.organizations WHERE slug = 'ifdd');

DELETE FROM media.storage_quotas WHERE organization_id IS NOT NULL;

DELETE FROM org.organizations WHERE slug <> 'ifdd';

DELETE FROM identity.role_assignments r
WHERE r.person_id NOT IN (SELECT id FROM personnes_gardees)
   OR (r.scope_type = 'organization' AND r.scope_id NOT IN (SELECT id FROM org.organizations))
   OR (r.scope_type = 'event' AND r.scope_id NOT IN (SELECT id FROM event.events))
   OR (r.scope_type = 'negotiation_space' AND r.scope_id NOT IN (SELECT id FROM negotiation.spaces));

DELETE FROM identity.people WHERE id NOT IN (SELECT id FROM personnes_gardees);

DO $$
DECLARE e record;
BEGIN
    FOR e IN SELECT DISTINCT entity_schema, entity_table FROM reference.entity_terms LOOP
        EXECUTE format(
            'DELETE FROM reference.entity_terms t WHERE t.entity_schema = %L AND t.entity_table = %L
               AND NOT EXISTS (SELECT 1 FROM %I.%I x WHERE x.id = t.entity_id)',
            e.entity_schema, e.entity_table, e.entity_schema, e.entity_table);
    END LOOP;
END $$;

-- Références des lignes gardées vers des lignes effacées.
UPDATE identity.people SET primary_organization_id = NULL
WHERE primary_organization_id IS NOT NULL
  AND primary_organization_id NOT IN (SELECT id FROM org.organizations);

SET LOCAL session_replication_role = origin;

-- 5. Contrôles : tables purgées vides, aucun orphelin de clé étrangère.
DO $$
DECLARE
    t record;
    k record;
    n bigint;
    orphelins text[] := '{}';
BEGIN
    FOR t IN SELECT nom FROM classement WHERE sort = 'purgee' LOOP
        EXECUTE format('SELECT count(*) FROM %s', t.nom) INTO n;
        IF n > 0 THEN
            RAISE EXCEPTION '% n''est pas vide après la purge (% lignes)', t.nom, n;
        END IF;
    END LOOP;

    FOR k IN
        SELECT c.conname, c.conrelid::regclass AS enfant, c.confrelid::regclass AS parent,
               (SELECT string_agg(format('e.%I', a.attname), ', ' ORDER BY u.ord)
                FROM unnest(c.conkey) WITH ORDINALITY u(att, ord)
                JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = u.att) AS cols_enfant,
               (SELECT string_agg(format('p.%I', a.attname), ', ' ORDER BY u.ord)
                FROM unnest(c.confkey) WITH ORDINALITY u(att, ord)
                JOIN pg_attribute a ON a.attrelid = c.confrelid AND a.attnum = u.att) AS cols_parent
        FROM pg_constraint c JOIN pg_namespace ns ON ns.oid = c.connamespace
        WHERE c.contype = 'f' AND c.conparentid = 0
          AND ns.nspname NOT IN ('v1', 'v1_auth', 'v1_storage')
    LOOP
        EXECUTE format(
            'SELECT count(*) FROM %s e WHERE ROW(%s) IS NOT NULL
               AND NOT EXISTS (SELECT 1 FROM %s p WHERE ROW(%s) = ROW(%s))',
            k.enfant, k.cols_enfant, k.parent, k.cols_parent, k.cols_enfant) INTO n;
        IF n > 0 THEN
            orphelins := orphelins || format('%s (%s : %s)', k.conname, k.enfant, n);
        END IF;
    END LOOP;
    IF cardinality(orphelins) > 0 THEN
        RAISE EXCEPTION 'Clés étrangères orphelines après la purge : %', array_to_string(orphelins, ', ');
    END IF;
END $$;

-- Sinon les dossiers repris commencent au numéro qui suit les 13 essais.
ALTER SEQUENCE programme.proposal_reference_seq RESTART WITH 1;

SELECT format('Purge faite : %s personnes, %s compte, %s rôles, %s organisation gardés ; %s objets Garage à effacer à la bascule.',
              (SELECT count(*) FROM identity.people),
              (SELECT count(*) FROM identity.accounts),
              (SELECT count(*) FROM identity.role_assignments),
              (SELECT count(*) FROM org.organizations),
              (SELECT count(*) FROM legacy.garage_a_purger)) AS bilan;

COMMIT;
