-- =============================================================================
-- ePavillon v2 — 900_seed.sql
-- Données d'amorçage : réglages de plateforme, drapeaux, pays, compte
-- d'administration initial.
--
-- Dépend de : tous les fichiers de schéma (000 à 130).
-- Ce fichier est IDEMPOTENT : il peut être rejoué sans effet de bord.
--
-- Les taxonomies (thèmes, catégories, types d'organisation, canaux
-- d'acquisition) sont amorcées dans 020_reference.sql, les rôles et permissions
-- dans 030_identity.sql. On ne les répète pas ici.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Réglages de plateforme
-- -----------------------------------------------------------------------------
INSERT INTO platform.settings (key, value, description, is_secret) VALUES
    ('platform.name',
     '{"fr":"ePavillon de la Francophonie","en":"Francophonie ePavilion"}',
     'Nom affiché de la plateforme.', false),
    ('platform.base_url',
     '"https://epavillonclimatique.francophonie.org"',
     'URL publique de référence, utilisée pour composer les liens des courriels.', false),
    ('platform.default_locale', '"fr"', 'Locale par défaut du frontend et des courriels.', false),
    ('platform.default_timezone', '"America/Montreal"', 'Fuseau par défaut du back-office (siège de l''IFDD).', false),
    -- `media.public_base_url` et `media.default_bucket` NE SONT PAS SEMÉS ICI.
    -- Ils l'étaient, avec d'autres valeurs que celles de 050_media.sql § 8 — et
    -- comme ce fichier est chargé après, son `ON CONFLICT DO NOTHING` les
    -- écartait en silence. Les valeurs actives ont toujours été celles de 050 ;
    -- ces deux lignes ne servaient qu'à faire croire le contraire à qui lit le
    -- fichier de semis pour connaître la configuration. Le module média déclare
    -- ses propres réglages, comme il déclare ses propres rôles d'attachement.
    ('email.from_address', '"no-reply@epavillonclimatique.francophonie.org"', 'Expéditeur des courriels transactionnels.', false),
    ('email.reply_to', '"epavillon@francophonie.org"', 'Adresse de réponse.', false),
    ('registration.reminder_offsets', '["2 days","1 day","1 hour","30 minutes"]',
     'Décalages de rappel appliqués par défaut avant une session.', false),
    ('organization.duplicate_block_threshold', '85',
     'Score de similarité au-delà duquel la création d''une organisation est bloquée au profit d''un rattachement.', false),
    ('organization.duplicate_warn_threshold', '55',
     'Score au-delà duquel des suggestions de rattachement sont proposées à l''utilisateur.', false),
    ('zoom.account_id', '"__secret__"', 'Identifiant de compte Zoom — la valeur réelle vit dans le coffre de secrets.', true),
    ('zoom.client_id', '"__secret__"', 'Client OAuth Zoom.', true),
    ('zoom.client_secret', '"__secret__"', 'Secret OAuth Zoom.', true)
ON CONFLICT (key) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 2. Drapeaux de fonctionnalités
--
-- Tout ce qui est en réflexion dans le cadrage démarre désactivé : le code peut
-- être livré sans être exposé, et l'ouverture se fait sans redéploiement.
--
-- Deux natures de drapeaux, à ne pas confondre :
--   `<module>.enabled` ferme l'interface d'un module entier — c'est lui que le
--   routage lit pour servir la page « En cours de maintenance » sans toucher
--   aux pages elles-mêmes ;
--   les drapeaux plus fins (`negotiation.channels`, `tools.ai_assistant`…)
--   commandent une fonctionnalité À L'INTÉRIEUR d'un module déjà ouvert. Ils ne
--   peuvent pas tenir lieu de drapeau de module.
-- -----------------------------------------------------------------------------
INSERT INTO platform.feature_flags (key, description, is_enabled, rollout_percent) VALUES
    ('publications.enabled',        'Espace Publications ouvert aux organisations.', false, 0),
    ('negotiation.enabled',         'Espace Négociations, réservé aux négociateurs.', false, 0),
    ('negotiation.channels',        'Canaux d''échange temps réel, à l''intérieur de l''espace Négociations.', false, 0),
    ('training.enabled',            'Espace Formations : catalogue, chapitres, quiz, attestations.', false, 0),
    ('messaging.enabled',           'Messagerie directe et mise en relation entre membres (tables du module engagement).', false, 0),
    ('tools.enabled',               'Espace Outils.', false, 0),
    ('tools.ai_assistant',          'Assistant IA et recherche documentaire (RAG), à l''intérieur des Outils.', false, 0),
    ('tools.surveys',               'Outil de sondages, à l''intérieur des Outils.', false, 0),
    ('calendar.external_sync',      'Synchronisation Google Agenda / Apple Calendar (phase ultérieure).', false, 0),
    ('newsletter.campaigns',        'Campagnes d''infolettre (hors périmètre du jalon 1).', false, 0),
    ('programme.waitlist',          'Liste d''attente sur les sessions à jauge limitée.', true, 100),
    ('org.auto_join_by_domain',     'Rattachement automatique à une organisation par domaine de courriel vérifié.', true, 100)
ON CONFLICT (key) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 3. Pays
--
-- ATTENTION — le référentiel complet ISO 3166-1 (249 entrées) est chargé par le
-- script d'import `scripts/seed_countries.rs`, qui lit un fichier de données
-- versionné. Ce fichier n'amorce que les entrées nécessaires au démarrage
-- (siège de l'IFDD et pays hôtes des prochaines éditions).
--
-- Le statut OIF (`oif_status`) DOIT être renseigné depuis la liste officielle
-- publiée par l'Organisation internationale de la Francophonie
-- (https://www.francophonie.org/88-etats-et-gouvernements-125) : cette liste
-- évolue à chaque Sommet et ne doit pas être devinée. Les entrées ci-dessous
-- sont laissées à 'none' lorsque le statut n'est pas certain.
-- -----------------------------------------------------------------------------
INSERT INTO reference.countries (iso2, iso3, name, continent, oif_status, default_timezone, flag_emoji) VALUES
    ('CA', 'CAN', '{"fr":"Canada","en":"Canada"}',                       'north_america', 'member', 'America/Toronto',   '🇨🇦'),
    ('FR', 'FRA', '{"fr":"France","en":"France"}',                       'europe',        'member', 'Europe/Paris',      '🇫🇷'),
    ('BE', 'BEL', '{"fr":"Belgique","en":"Belgium"}',                    'europe',        'member', 'Europe/Brussels',   '🇧🇪'),
    ('CH', 'CHE', '{"fr":"Suisse","en":"Switzerland"}',                  'europe',        'member', 'Europe/Zurich',     '🇨🇭'),
    ('SN', 'SEN', '{"fr":"Sénégal","en":"Senegal"}',                     'africa',        'member', 'Africa/Dakar',      '🇸🇳'),
    ('CI', 'CIV', '{"fr":"Côte d''Ivoire","en":"Côte d''Ivoire"}',       'africa',        'member', 'Africa/Abidjan',    '🇨🇮'),
    ('CM', 'CMR', '{"fr":"Cameroun","en":"Cameroon"}',                   'africa',        'member', 'Africa/Douala',     '🇨🇲'),
    ('BJ', 'BEN', '{"fr":"Bénin","en":"Benin"}',                         'africa',        'member', 'Africa/Porto-Novo', '🇧🇯'),
    ('BF', 'BFA', '{"fr":"Burkina Faso","en":"Burkina Faso"}',           'africa',        'member', 'Africa/Ouagadougou','🇧🇫'),
    ('TG', 'TGO', '{"fr":"Togo","en":"Togo"}',                           'africa',        'member', 'Africa/Lome',       '🇹🇬'),
    ('NE', 'NER', '{"fr":"Niger","en":"Niger"}',                         'africa',        'member', 'Africa/Niamey',     '🇳🇪'),
    ('ML', 'MLI', '{"fr":"Mali","en":"Mali"}',                           'africa',        'member', 'Africa/Bamako',     '🇲🇱'),
    ('TD', 'TCD', '{"fr":"Tchad","en":"Chad"}',                          'africa',        'member', 'Africa/Ndjamena',   '🇹🇩'),
    ('CD', 'COD', '{"fr":"République démocratique du Congo","en":"Democratic Republic of the Congo"}',
                                                                          'africa',        'member', 'Africa/Kinshasa',   '🇨🇩'),
    ('CG', 'COG', '{"fr":"Congo","en":"Congo"}',                         'africa',        'member', 'Africa/Brazzaville','🇨🇬'),
    ('GA', 'GAB', '{"fr":"Gabon","en":"Gabon"}',                         'africa',        'member', 'Africa/Libreville', '🇬🇦'),
    ('MG', 'MDG', '{"fr":"Madagascar","en":"Madagascar"}',               'africa',        'member', 'Indian/Antananarivo','🇲🇬'),
    ('MA', 'MAR', '{"fr":"Maroc","en":"Morocco"}',                       'africa',        'member', 'Africa/Casablanca', '🇲🇦'),
    ('TN', 'TUN', '{"fr":"Tunisie","en":"Tunisia"}',                     'africa',        'member', 'Africa/Tunis',      '🇹🇳'),
    ('HT', 'HTI', '{"fr":"Haïti","en":"Haiti"}',                         'north_america', 'member', 'America/Port-au-Prince','🇭🇹'),
    ('VN', 'VNM', '{"fr":"Viêt Nam","en":"Viet Nam"}',                   'asia',          'member', 'Asia/Ho_Chi_Minh',  '🇻🇳'),
    ('LB', 'LBN', '{"fr":"Liban","en":"Lebanon"}',                       'asia',          'member', 'Asia/Beirut',       '🇱🇧'),
    ('AZ', 'AZE', '{"fr":"Azerbaïdjan","en":"Azerbaijan"}',              'asia',          'none',   'Asia/Baku',         '🇦🇿'),
    ('AE', 'ARE', '{"fr":"Émirats arabes unis","en":"United Arab Emirates"}',
                                                                          'asia',          'none',   'Asia/Dubai',        '🇦🇪'),
    ('BR', 'BRA', '{"fr":"Brésil","en":"Brazil"}',                       'south_america', 'none',   'America/Belem',     '🇧🇷'),
    ('EG', 'EGY', '{"fr":"Égypte","en":"Egypt"}',                        'africa',        'member', 'Africa/Cairo',      '🇪🇬'),
    ('CO', 'COL', '{"fr":"Colombie","en":"Colombia"}',                   'south_america', 'none',   'America/Bogota',    '🇨🇴'),
    ('SA', 'SAU', '{"fr":"Arabie saoudite","en":"Saudi Arabia"}',        'asia',          'none',   'Asia/Riyadh',       '🇸🇦'),
    ('TR', 'TUR', '{"fr":"Türkiye","en":"Türkiye"}',                     'asia',          'none',   'Europe/Istanbul',   '🇹🇷')
ON CONFLICT (iso2) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 4. Séries d'événements
--
-- Les trois COP suivies par l'IFDD, plus le conteneur des rendez-vous ponctuels
-- (« section Autres » de la page programmation, activités uniques ou cycles
-- organisés directement par l'IFDD sur Zoom ou Teams).
-- -----------------------------------------------------------------------------
INSERT INTO event.event_series (code, kind, name, description, slug, track_code) VALUES
    ('cop_climate', 'cop_climate',
     '{"fr":"COP Climat (CCNUCC)","en":"Climate COP (UNFCCC)"}',
     '{"fr":"Conférence des Parties à la Convention-cadre des Nations unies sur les changements climatiques. L''OIF y tient un pavillon francophone.","en":"Conference of the Parties to the UNFCCC."}',
     'cop-climat', 'climate'),
    ('cop_biodiversity', 'cop_biodiversity',
     '{"fr":"COP Biodiversité (CDB)","en":"Biodiversity COP (CBD)"}',
     '{"fr":"Conférence des Parties à la Convention sur la diversité biologique. Le pavillon n''est pas systématique.","en":"Conference of the Parties to the CBD."}',
     'cop-biodiversite', 'biodiversity'),
    ('cop_desertification', 'cop_desertification',
     '{"fr":"COP Désertification (CNULCD)","en":"Desertification COP (UNCCD)"}',
     '{"fr":"Conférence des Parties à la Convention des Nations unies sur la lutte contre la désertification.","en":"Conference of the Parties to the UNCCD."}',
     'cop-desertification', 'desertification'),
    ('ifdd_webinars', 'webinar_series',
     '{"fr":"Rendez-vous de l''IFDD","en":"IFDD meetings"}',
     '{"fr":"Activités ponctuelles ou cycles thématiques organisés directement par l''IFDD, en ligne ou hybrides.","en":"Standalone or recurring activities organised by IFDD."}',
     'rendez-vous-ifdd', NULL)
ON CONFLICT (code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 4 bis. Canal de diffusion par défaut
--
-- Ligne sans événement : elle s'applique à toute session diffusée dont
-- l'événement n'a pas son propre canal. C'est elle qui rend effective la règle
-- « une seule activité en direct à la fois » dès la première programmation,
-- sans que personne ait à y penser.
-- -----------------------------------------------------------------------------
INSERT INTO event.broadcast_channels (event_id, code, name, provider, channel_ref, is_default)
VALUES (NULL, 'ifdd_principal',
        '{"fr":"Chaîne principale IFDD","en":"IFDD main channel"}',
        'youtube', '@ifddoif', true)
ON CONFLICT (event_id, code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 5. Organisation pivot et compte d'administration
--
-- Le premier super-administrateur est créé sans moyen d'authentification :
-- l'installation se termine par un envoi de lien d'activation
-- (identity.one_time_tokens, usage `invitation`). Aucun mot de passe par défaut
-- n'est écrit en base — c'est une faille classique des amorçages.
-- Remplacer l'adresse ci-dessous avant exécution en production.
-- -----------------------------------------------------------------------------
DO $$
DECLARE
    v_admin_email  text := 'admin@epavillonclimatique.francophonie.org';
    v_person_id    uuid;
    v_org_id       uuid;
    v_country_id   uuid;
BEGIN
    SELECT id INTO v_country_id FROM reference.countries WHERE iso2 = 'CA';

    -- Organisation pivot : l'IFDD lui-même.
    INSERT INTO org.organizations (legal_name, acronym, slug, organization_type_code,
                                   country_id, city, status, verified_at, website, trust_score)
    VALUES ('Institut de la Francophonie pour le développement durable', 'IFDD', 'ifdd',
            'international_organization', v_country_id, 'Québec', 'active', now(),
            'https://www.ifdd.francophonie.org', 100)
    ON CONFLICT (slug) DO UPDATE SET updated_at = now()
    RETURNING id INTO v_org_id;

    IF v_org_id IS NULL THEN
        SELECT id INTO v_org_id FROM org.organizations WHERE slug = 'ifdd';
    END IF;

    -- Dénominations complémentaires : couvrent les formulations rencontrées à
    -- l'inscription et évitent la création d'un doublon dès le premier jour.
    INSERT INTO org.organization_names (organization_id, name, kind, is_confirmed) VALUES
        (v_org_id, 'IFDD', 'acronym', true),
        (v_org_id, 'Institut de la Francophonie pour le Developpement Durable', 'misspelling', true),
        (v_org_id, 'Institute of the Francophonie for Sustainable Development', 'translation', true),
        (v_org_id, 'Institut de l''énergie et de l''environnement de la Francophonie', 'former', true),
        (v_org_id, 'IEPF', 'former', true)
    ON CONFLICT (organization_id, name_normalized, kind) DO NOTHING;

    INSERT INTO org.organization_domains (organization_id, domain, verified_at, verification_method, auto_join)
    VALUES (v_org_id, 'ifdd.francophonie.org', now(), 'manual', true),
           (v_org_id, 'francophonie.org', now(), 'manual', false)
    ON CONFLICT DO NOTHING;

    -- Compte d'administration.
    INSERT INTO identity.people (primary_email, first_name, last_name, country_id,
                                 preferred_locale, timezone, primary_organization_id, status)
    VALUES (v_admin_email::platform.email, 'Administration', 'ePavillon', v_country_id,
            'fr', 'America/Montreal', v_org_id, 'active')
    ON CONFLICT DO NOTHING
    RETURNING id INTO v_person_id;

    IF v_person_id IS NULL THEN
        SELECT id INTO v_person_id FROM identity.people WHERE primary_email = v_admin_email::platform.email;
    END IF;

    INSERT INTO identity.role_assignments (person_id, role_code, scope_type)
    VALUES (v_person_id, 'super_admin', 'global')
    ON CONFLICT DO NOTHING;

    INSERT INTO org.memberships (organization_id, person_id, role, status, is_primary, approved_at)
    VALUES (v_org_id, v_person_id, 'manager', 'active', true, now())
    ON CONFLICT (organization_id, person_id) DO NOTHING;

    RAISE NOTICE 'Compte d''administration initial : % (identifiant %). Envoyer un lien d''activation, aucun mot de passe n''a été créé.',
        v_admin_email, v_person_id;
END
$$;

-- -----------------------------------------------------------------------------
-- 6. Partitions initiales
--
-- Le worker crée les partitions du mois suivant chaque nuit ; on amorce les
-- deux premières pour que la première écriture ne tombe pas dans la partition
-- par défaut.
-- -----------------------------------------------------------------------------
SELECT platform.ensure_month_partition('platform', 'audit_log', current_date);
SELECT platform.ensure_month_partition('platform', 'audit_log', (current_date + interval '1 month')::date);

-- -----------------------------------------------------------------------------
-- 7. Contrôle de conformité des frontières de modules
--
-- Doit retourner zéro ligne. Toute FK inter-modules mal nommée est signalée ici
-- avant d'être découverte le jour de l'extraction d'un service.
-- -----------------------------------------------------------------------------
DO $$
DECLARE
    v_violations text;
BEGIN
    SELECT string_agg(format('%s.%s -> %s.%s (%s)', source_schema, source_table,
                             target_schema, target_table, constraint_name), E'\n')
      INTO v_violations
    FROM platform.cross_module_fk_report
    WHERE NOT is_compliant;

    IF v_violations IS NOT NULL THEN
        RAISE WARNING E'Clés étrangères inter-modules non conformes (préfixe xmod_fk_ attendu) :\n%', v_violations;
    ELSE
        RAISE NOTICE 'Frontières de modules conformes.';
    END IF;
END
$$;
