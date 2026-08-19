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
    ('directory.enabled',           'Annuaire des organisations et des personnes, et profils publics — l''espace Communauté.', false, 0),
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
-- 3 bis. Le reste de la norme ISO 3166-1 — les 220 autres entités
--
-- POURQUOI LA LISTE COMPLÈTE, ET PAS SEULEMENT L'ESPACE FRANCOPHONE
-- Le pays d'une édition est choisi dans un sélecteur du back-office (A10), et une
-- conférence des Nations unies se tient où elle se tient : la COP29 à Bakou, la
-- COP28 à Dubaï, la COP30 et la COP31 à Belém. Une liste réduite aux membres de
-- l'OIF obligerait à modifier le semis avant chaque édition hors zone — ce qui
-- s'est produit trois fois en trois ans sur la v1.
--
-- PROVENANCE DES DONNÉES, ET CE QU'ELLE VAUT
--   · `iso2`, `iso3`, `numeric_code` : la norme ISO 3166-1. Vérifiés mécaniquement
--     — 249 entités, trois codes uniques chacune, et les 29 lignes curées
--     ci-dessus concordent.
--   · `name` : les données CLDR de l'exécution (`Intl.DisplayNames`), en français
--     et en anglais. Elles ne sont pas toujours la forme protocolaire — CLDR écrit
--     « Congo-Kinshasa » là où le bloc curé ci-dessus écrit « République
--     démocratique du Congo ». C'est voulu : le bloc curé PRIME (`ON CONFLICT
--     (iso2) DO NOTHING` sur un INSERT unique, les lignes curées étant listées en
--     premier), et l'IFDD affine les autres au fil de l'eau depuis le back-office.
--   · `default_timezone` : le fuseau de la capitale ou le fuseau principal. Chacun
--     vérifié comme identifiant IANA valide.
--
-- `oif_status` VAUT 'none' SUR TOUTE CETTE SECTION, ET CE N'EST PAS UNE AFFIRMATION
-- L'OIF compte 88 États et gouvernements, dont une trentaine ne figure pas dans le
-- bloc curé ci-dessus. Leur statut n'est PAS deviné ici : la liste évolue à chaque
-- Sommet, elle fait autorité chez l'OIF, et un statut inventé serait une donnée
-- fausse dans un référentiel dont l'IFDD se sert pour ses rapports. Il reste à
-- reprendre depuis la liste officielle — consigné dans
-- docs/progression/ecrans/a10-evenements.md.
-- -----------------------------------------------------------------------------
INSERT INTO reference.countries (iso2, iso3, numeric_code, name, continent, oif_status, default_timezone, flag_emoji) VALUES
    ('AD', 'AND', '020', '{"fr":"Andorre","en":"Andorra"}', 'europe', 'none', 'Europe/Andorra', '🇦🇩'),
    ('AF', 'AFG', '004', '{"fr":"Afghanistan","en":"Afghanistan"}', 'asia', 'none', 'Asia/Kabul', '🇦🇫'),
    ('AG', 'ATG', '028', '{"fr":"Antigua-et-Barbuda","en":"Antigua & Barbuda"}', 'north_america', 'none', 'America/Antigua', '🇦🇬'),
    ('AI', 'AIA', '660', '{"fr":"Anguilla","en":"Anguilla"}', 'north_america', 'none', 'America/Anguilla', '🇦🇮'),
    ('AL', 'ALB', '008', '{"fr":"Albanie","en":"Albania"}', 'europe', 'none', 'Europe/Tirane', '🇦🇱'),
    ('AM', 'ARM', '051', '{"fr":"Arménie","en":"Armenia"}', 'asia', 'none', 'Asia/Yerevan', '🇦🇲'),
    ('AO', 'AGO', '024', '{"fr":"Angola","en":"Angola"}', 'africa', 'none', 'Africa/Luanda', '🇦🇴'),
    ('AQ', 'ATA', '010', '{"fr":"Antarctique","en":"Antarctica"}', 'antarctica', 'none', 'Antarctica/Casey', '🇦🇶'),
    ('AR', 'ARG', '032', '{"fr":"Argentine","en":"Argentina"}', 'south_america', 'none', 'America/Argentina/Buenos_Aires', '🇦🇷'),
    ('AS', 'ASM', '016', '{"fr":"Samoa américaines","en":"American Samoa"}', 'oceania', 'none', 'Pacific/Pago_Pago', '🇦🇸'),
    ('AT', 'AUT', '040', '{"fr":"Autriche","en":"Austria"}', 'europe', 'none', 'Europe/Vienna', '🇦🇹'),
    ('AU', 'AUS', '036', '{"fr":"Australie","en":"Australia"}', 'oceania', 'none', 'Australia/Sydney', '🇦🇺'),
    ('AW', 'ABW', '533', '{"fr":"Aruba","en":"Aruba"}', 'north_america', 'none', 'America/Aruba', '🇦🇼'),
    ('AX', 'ALA', '248', '{"fr":"Îles Åland","en":"Åland Islands"}', 'europe', 'none', 'Europe/Mariehamn', '🇦🇽'),
    ('BA', 'BIH', '070', '{"fr":"Bosnie-Herzégovine","en":"Bosnia & Herzegovina"}', 'europe', 'none', 'Europe/Sarajevo', '🇧🇦'),
    ('BB', 'BRB', '052', '{"fr":"Barbade","en":"Barbados"}', 'north_america', 'none', 'America/Barbados', '🇧🇧'),
    ('BD', 'BGD', '050', '{"fr":"Bangladesh","en":"Bangladesh"}', 'asia', 'none', 'Asia/Dhaka', '🇧🇩'),
    ('BG', 'BGR', '100', '{"fr":"Bulgarie","en":"Bulgaria"}', 'europe', 'none', 'Europe/Sofia', '🇧🇬'),
    ('BH', 'BHR', '048', '{"fr":"Bahreïn","en":"Bahrain"}', 'asia', 'none', 'Asia/Bahrain', '🇧🇭'),
    ('BI', 'BDI', '108', '{"fr":"Burundi","en":"Burundi"}', 'africa', 'none', 'Africa/Bujumbura', '🇧🇮'),
    ('BL', 'BLM', '652', '{"fr":"Saint-Barthélemy","en":"St. Barthélemy"}', 'north_america', 'none', 'America/St_Barthelemy', '🇧🇱'),
    ('BM', 'BMU', '060', '{"fr":"Bermudes","en":"Bermuda"}', 'north_america', 'none', 'Atlantic/Bermuda', '🇧🇲'),
    ('BN', 'BRN', '096', '{"fr":"Brunei","en":"Brunei"}', 'asia', 'none', 'Asia/Brunei', '🇧🇳'),
    ('BO', 'BOL', '068', '{"fr":"Bolivie","en":"Bolivia"}', 'south_america', 'none', 'America/La_Paz', '🇧🇴'),
    ('BQ', 'BES', '535', '{"fr":"Pays-Bas caribéens","en":"Caribbean Netherlands"}', 'north_america', 'none', 'America/Kralendijk', '🇧🇶'),
    ('BS', 'BHS', '044', '{"fr":"Bahamas","en":"Bahamas"}', 'north_america', 'none', 'America/Nassau', '🇧🇸'),
    ('BT', 'BTN', '064', '{"fr":"Bhoutan","en":"Bhutan"}', 'asia', 'none', 'Asia/Thimphu', '🇧🇹'),
    ('BV', 'BVT', '074', '{"fr":"Île Bouvet","en":"Bouvet Island"}', 'antarctica', 'none', 'UTC', '🇧🇻'),
    ('BW', 'BWA', '072', '{"fr":"Botswana","en":"Botswana"}', 'africa', 'none', 'Africa/Gaborone', '🇧🇼'),
    ('BY', 'BLR', '112', '{"fr":"Biélorussie","en":"Belarus"}', 'europe', 'none', 'Europe/Minsk', '🇧🇾'),
    ('BZ', 'BLZ', '084', '{"fr":"Belize","en":"Belize"}', 'north_america', 'none', 'America/Belize', '🇧🇿'),
    ('CC', 'CCK', '166', '{"fr":"Îles Cocos","en":"Cocos (Keeling) Islands"}', 'asia', 'none', 'Indian/Cocos', '🇨🇨'),
    ('CF', 'CAF', '140', '{"fr":"République centrafricaine","en":"Central African Republic"}', 'africa', 'none', 'Africa/Bangui', '🇨🇫'),
    ('CK', 'COK', '184', '{"fr":"Îles Cook","en":"Cook Islands"}', 'oceania', 'none', 'Pacific/Rarotonga', '🇨🇰'),
    ('CL', 'CHL', '152', '{"fr":"Chili","en":"Chile"}', 'south_america', 'none', 'America/Santiago', '🇨🇱'),
    ('CN', 'CHN', '156', '{"fr":"Chine","en":"China"}', 'asia', 'none', 'Asia/Shanghai', '🇨🇳'),
    ('CR', 'CRI', '188', '{"fr":"Costa Rica","en":"Costa Rica"}', 'north_america', 'none', 'America/Costa_Rica', '🇨🇷'),
    ('CU', 'CUB', '192', '{"fr":"Cuba","en":"Cuba"}', 'north_america', 'none', 'America/Havana', '🇨🇺'),
    ('CV', 'CPV', '132', '{"fr":"Cap-Vert","en":"Cape Verde"}', 'africa', 'none', 'Atlantic/Cape_Verde', '🇨🇻'),
    ('CW', 'CUW', '531', '{"fr":"Curaçao","en":"Curaçao"}', 'north_america', 'none', 'America/Curacao', '🇨🇼'),
    ('CX', 'CXR', '162', '{"fr":"Île Christmas","en":"Christmas Island"}', 'asia', 'none', 'Indian/Christmas', '🇨🇽'),
    ('CY', 'CYP', '196', '{"fr":"Chypre","en":"Cyprus"}', 'asia', 'none', 'Asia/Nicosia', '🇨🇾'),
    ('CZ', 'CZE', '203', '{"fr":"Tchéquie","en":"Czechia"}', 'europe', 'none', 'Europe/Prague', '🇨🇿'),
    ('DE', 'DEU', '276', '{"fr":"Allemagne","en":"Germany"}', 'europe', 'none', 'Europe/Berlin', '🇩🇪'),
    ('DJ', 'DJI', '262', '{"fr":"Djibouti","en":"Djibouti"}', 'africa', 'none', 'Africa/Djibouti', '🇩🇯'),
    ('DK', 'DNK', '208', '{"fr":"Danemark","en":"Denmark"}', 'europe', 'none', 'Europe/Copenhagen', '🇩🇰'),
    ('DM', 'DMA', '212', '{"fr":"Dominique","en":"Dominica"}', 'north_america', 'none', 'America/Dominica', '🇩🇲'),
    ('DO', 'DOM', '214', '{"fr":"République dominicaine","en":"Dominican Republic"}', 'north_america', 'none', 'America/Santo_Domingo', '🇩🇴'),
    ('DZ', 'DZA', '012', '{"fr":"Algérie","en":"Algeria"}', 'africa', 'none', 'Africa/Algiers', '🇩🇿'),
    ('EC', 'ECU', '218', '{"fr":"Équateur","en":"Ecuador"}', 'south_america', 'none', 'America/Guayaquil', '🇪🇨'),
    ('EE', 'EST', '233', '{"fr":"Estonie","en":"Estonia"}', 'europe', 'none', 'Europe/Tallinn', '🇪🇪'),
    ('EH', 'ESH', '732', '{"fr":"Sahara occidental","en":"Western Sahara"}', 'africa', 'none', 'Africa/El_Aaiun', '🇪🇭'),
    ('ER', 'ERI', '232', '{"fr":"Érythrée","en":"Eritrea"}', 'africa', 'none', 'Africa/Asmara', '🇪🇷'),
    ('ES', 'ESP', '724', '{"fr":"Espagne","en":"Spain"}', 'europe', 'none', 'Europe/Madrid', '🇪🇸'),
    ('ET', 'ETH', '231', '{"fr":"Éthiopie","en":"Ethiopia"}', 'africa', 'none', 'Africa/Addis_Ababa', '🇪🇹'),
    ('FI', 'FIN', '246', '{"fr":"Finlande","en":"Finland"}', 'europe', 'none', 'Europe/Helsinki', '🇫🇮'),
    ('FJ', 'FJI', '242', '{"fr":"Fidji","en":"Fiji"}', 'oceania', 'none', 'Pacific/Fiji', '🇫🇯'),
    ('FK', 'FLK', '238', '{"fr":"Îles Malouines","en":"Falkland Islands"}', 'south_america', 'none', 'Atlantic/Stanley', '🇫🇰'),
    ('FM', 'FSM', '583', '{"fr":"Micronésie","en":"Micronesia"}', 'oceania', 'none', 'Pacific/Chuuk', '🇫🇲'),
    ('FO', 'FRO', '234', '{"fr":"Îles Féroé","en":"Faroe Islands"}', 'europe', 'none', 'Atlantic/Faroe', '🇫🇴'),
    ('GB', 'GBR', '826', '{"fr":"Royaume-Uni","en":"United Kingdom"}', 'europe', 'none', 'Europe/London', '🇬🇧'),
    ('GD', 'GRD', '308', '{"fr":"Grenade","en":"Grenada"}', 'north_america', 'none', 'America/Grenada', '🇬🇩'),
    ('GE', 'GEO', '268', '{"fr":"Géorgie","en":"Georgia"}', 'asia', 'none', 'Asia/Tbilisi', '🇬🇪'),
    ('GF', 'GUF', '254', '{"fr":"Guyane française","en":"French Guiana"}', 'south_america', 'none', 'America/Cayenne', '🇬🇫'),
    ('GG', 'GGY', '831', '{"fr":"Guernesey","en":"Guernsey"}', 'europe', 'none', 'Europe/Guernsey', '🇬🇬'),
    ('GH', 'GHA', '288', '{"fr":"Ghana","en":"Ghana"}', 'africa', 'none', 'Africa/Accra', '🇬🇭'),
    ('GI', 'GIB', '292', '{"fr":"Gibraltar","en":"Gibraltar"}', 'europe', 'none', 'Europe/Gibraltar', '🇬🇮'),
    ('GL', 'GRL', '304', '{"fr":"Groenland","en":"Greenland"}', 'north_america', 'none', 'America/Nuuk', '🇬🇱'),
    ('GM', 'GMB', '270', '{"fr":"Gambie","en":"Gambia"}', 'africa', 'none', 'Africa/Banjul', '🇬🇲'),
    ('GN', 'GIN', '324', '{"fr":"Guinée","en":"Guinea"}', 'africa', 'none', 'Africa/Conakry', '🇬🇳'),
    ('GP', 'GLP', '312', '{"fr":"Guadeloupe","en":"Guadeloupe"}', 'north_america', 'none', 'America/Guadeloupe', '🇬🇵'),
    ('GQ', 'GNQ', '226', '{"fr":"Guinée équatoriale","en":"Equatorial Guinea"}', 'africa', 'none', 'Africa/Malabo', '🇬🇶'),
    ('GR', 'GRC', '300', '{"fr":"Grèce","en":"Greece"}', 'europe', 'none', 'Europe/Athens', '🇬🇷'),
    ('GS', 'SGS', '239', '{"fr":"Géorgie du Sud-et-les Îles Sandwich du Sud","en":"South Georgia & South Sandwich Islands"}', 'antarctica', 'none', 'Atlantic/South_Georgia', '🇬🇸'),
    ('GT', 'GTM', '320', '{"fr":"Guatemala","en":"Guatemala"}', 'north_america', 'none', 'America/Guatemala', '🇬🇹'),
    ('GU', 'GUM', '316', '{"fr":"Guam","en":"Guam"}', 'oceania', 'none', 'Pacific/Guam', '🇬🇺'),
    ('GW', 'GNB', '624', '{"fr":"Guinée-Bissau","en":"Guinea-Bissau"}', 'africa', 'none', 'Africa/Bissau', '🇬🇼'),
    ('GY', 'GUY', '328', '{"fr":"Guyana","en":"Guyana"}', 'south_america', 'none', 'America/Guyana', '🇬🇾'),
    ('HK', 'HKG', '344', '{"fr":"R.A.S. chinoise de Hong Kong","en":"Hong Kong SAR China"}', 'asia', 'none', 'Asia/Hong_Kong', '🇭🇰'),
    ('HM', 'HMD', '334', '{"fr":"Îles Heard-et-MacDonald","en":"Heard & McDonald Islands"}', 'antarctica', 'none', 'UTC', '🇭🇲'),
    ('HN', 'HND', '340', '{"fr":"Honduras","en":"Honduras"}', 'north_america', 'none', 'America/Tegucigalpa', '🇭🇳'),
    ('HR', 'HRV', '191', '{"fr":"Croatie","en":"Croatia"}', 'europe', 'none', 'Europe/Zagreb', '🇭🇷'),
    ('HU', 'HUN', '348', '{"fr":"Hongrie","en":"Hungary"}', 'europe', 'none', 'Europe/Budapest', '🇭🇺'),
    ('ID', 'IDN', '360', '{"fr":"Indonésie","en":"Indonesia"}', 'asia', 'none', 'Asia/Jakarta', '🇮🇩'),
    ('IE', 'IRL', '372', '{"fr":"Irlande","en":"Ireland"}', 'europe', 'none', 'Europe/Dublin', '🇮🇪'),
    ('IL', 'ISR', '376', '{"fr":"Israël","en":"Israel"}', 'asia', 'none', 'Asia/Jerusalem', '🇮🇱'),
    ('IM', 'IMN', '833', '{"fr":"Île de Man","en":"Isle of Man"}', 'europe', 'none', 'Europe/Isle_of_Man', '🇮🇲'),
    ('IN', 'IND', '356', '{"fr":"Inde","en":"India"}', 'asia', 'none', 'Asia/Kolkata', '🇮🇳'),
    ('IO', 'IOT', '086', '{"fr":"Territoire britannique de l’océan Indien","en":"British Indian Ocean Territory"}', 'asia', 'none', 'Indian/Chagos', '🇮🇴'),
    ('IQ', 'IRQ', '368', '{"fr":"Irak","en":"Iraq"}', 'asia', 'none', 'Asia/Baghdad', '🇮🇶'),
    ('IR', 'IRN', '364', '{"fr":"Iran","en":"Iran"}', 'asia', 'none', 'Asia/Tehran', '🇮🇷'),
    ('IS', 'ISL', '352', '{"fr":"Islande","en":"Iceland"}', 'europe', 'none', 'Atlantic/Reykjavik', '🇮🇸'),
    ('IT', 'ITA', '380', '{"fr":"Italie","en":"Italy"}', 'europe', 'none', 'Europe/Rome', '🇮🇹'),
    ('JE', 'JEY', '832', '{"fr":"Jersey","en":"Jersey"}', 'europe', 'none', 'Europe/Jersey', '🇯🇪'),
    ('JM', 'JAM', '388', '{"fr":"Jamaïque","en":"Jamaica"}', 'north_america', 'none', 'America/Jamaica', '🇯🇲'),
    ('JO', 'JOR', '400', '{"fr":"Jordanie","en":"Jordan"}', 'asia', 'none', 'Asia/Amman', '🇯🇴'),
    ('JP', 'JPN', '392', '{"fr":"Japon","en":"Japan"}', 'asia', 'none', 'Asia/Tokyo', '🇯🇵'),
    ('KE', 'KEN', '404', '{"fr":"Kenya","en":"Kenya"}', 'africa', 'none', 'Africa/Nairobi', '🇰🇪'),
    ('KG', 'KGZ', '417', '{"fr":"Kirghizstan","en":"Kyrgyzstan"}', 'asia', 'none', 'Asia/Bishkek', '🇰🇬'),
    ('KH', 'KHM', '116', '{"fr":"Cambodge","en":"Cambodia"}', 'asia', 'none', 'Asia/Phnom_Penh', '🇰🇭'),
    ('KI', 'KIR', '296', '{"fr":"Kiribati","en":"Kiribati"}', 'oceania', 'none', 'Pacific/Tarawa', '🇰🇮'),
    ('KM', 'COM', '174', '{"fr":"Comores","en":"Comoros"}', 'africa', 'none', 'Indian/Comoro', '🇰🇲'),
    ('KN', 'KNA', '659', '{"fr":"Saint-Christophe-et-Niévès","en":"St. Kitts & Nevis"}', 'north_america', 'none', 'America/St_Kitts', '🇰🇳'),
    ('KP', 'PRK', '408', '{"fr":"Corée du Nord","en":"North Korea"}', 'asia', 'none', 'Asia/Pyongyang', '🇰🇵'),
    ('KR', 'KOR', '410', '{"fr":"Corée du Sud","en":"South Korea"}', 'asia', 'none', 'Asia/Seoul', '🇰🇷'),
    ('KW', 'KWT', '414', '{"fr":"Koweït","en":"Kuwait"}', 'asia', 'none', 'Asia/Kuwait', '🇰🇼'),
    ('KY', 'CYM', '136', '{"fr":"Îles Caïmans","en":"Cayman Islands"}', 'north_america', 'none', 'America/Cayman', '🇰🇾'),
    ('KZ', 'KAZ', '398', '{"fr":"Kazakhstan","en":"Kazakhstan"}', 'asia', 'none', 'Asia/Almaty', '🇰🇿'),
    ('LA', 'LAO', '418', '{"fr":"Laos","en":"Laos"}', 'asia', 'none', 'Asia/Vientiane', '🇱🇦'),
    ('LC', 'LCA', '662', '{"fr":"Sainte-Lucie","en":"St. Lucia"}', 'north_america', 'none', 'America/St_Lucia', '🇱🇨'),
    ('LI', 'LIE', '438', '{"fr":"Liechtenstein","en":"Liechtenstein"}', 'europe', 'none', 'Europe/Vaduz', '🇱🇮'),
    ('LK', 'LKA', '144', '{"fr":"Sri Lanka","en":"Sri Lanka"}', 'asia', 'none', 'Asia/Colombo', '🇱🇰'),
    ('LR', 'LBR', '430', '{"fr":"Liberia","en":"Liberia"}', 'africa', 'none', 'Africa/Monrovia', '🇱🇷'),
    ('LS', 'LSO', '426', '{"fr":"Lesotho","en":"Lesotho"}', 'africa', 'none', 'Africa/Maseru', '🇱🇸'),
    ('LT', 'LTU', '440', '{"fr":"Lituanie","en":"Lithuania"}', 'europe', 'none', 'Europe/Vilnius', '🇱🇹'),
    ('LU', 'LUX', '442', '{"fr":"Luxembourg","en":"Luxembourg"}', 'europe', 'none', 'Europe/Luxembourg', '🇱🇺'),
    ('LV', 'LVA', '428', '{"fr":"Lettonie","en":"Latvia"}', 'europe', 'none', 'Europe/Riga', '🇱🇻'),
    ('LY', 'LBY', '434', '{"fr":"Libye","en":"Libya"}', 'africa', 'none', 'Africa/Tripoli', '🇱🇾'),
    ('MC', 'MCO', '492', '{"fr":"Monaco","en":"Monaco"}', 'europe', 'none', 'Europe/Monaco', '🇲🇨'),
    ('MD', 'MDA', '498', '{"fr":"Moldavie","en":"Moldova"}', 'europe', 'none', 'Europe/Chisinau', '🇲🇩'),
    ('ME', 'MNE', '499', '{"fr":"Monténégro","en":"Montenegro"}', 'europe', 'none', 'Europe/Podgorica', '🇲🇪'),
    ('MF', 'MAF', '663', '{"fr":"Saint-Martin","en":"St. Martin"}', 'north_america', 'none', 'America/Marigot', '🇲🇫'),
    ('MH', 'MHL', '584', '{"fr":"Îles Marshall","en":"Marshall Islands"}', 'oceania', 'none', 'Pacific/Majuro', '🇲🇭'),
    ('MK', 'MKD', '807', '{"fr":"Macédoine du Nord","en":"North Macedonia"}', 'europe', 'none', 'Europe/Skopje', '🇲🇰'),
    ('MM', 'MMR', '104', '{"fr":"Myanmar (Birmanie)","en":"Myanmar (Burma)"}', 'asia', 'none', 'Asia/Yangon', '🇲🇲'),
    ('MN', 'MNG', '496', '{"fr":"Mongolie","en":"Mongolia"}', 'asia', 'none', 'Asia/Ulaanbaatar', '🇲🇳'),
    ('MO', 'MAC', '446', '{"fr":"R.A.S. chinoise de Macao","en":"Macao SAR China"}', 'asia', 'none', 'Asia/Macau', '🇲🇴'),
    ('MP', 'MNP', '580', '{"fr":"Îles Mariannes du Nord","en":"Northern Mariana Islands"}', 'oceania', 'none', 'Pacific/Saipan', '🇲🇵'),
    ('MQ', 'MTQ', '474', '{"fr":"Martinique","en":"Martinique"}', 'north_america', 'none', 'America/Martinique', '🇲🇶'),
    ('MR', 'MRT', '478', '{"fr":"Mauritanie","en":"Mauritania"}', 'africa', 'none', 'Africa/Nouakchott', '🇲🇷'),
    ('MS', 'MSR', '500', '{"fr":"Montserrat","en":"Montserrat"}', 'north_america', 'none', 'America/Montserrat', '🇲🇸'),
    ('MT', 'MLT', '470', '{"fr":"Malte","en":"Malta"}', 'europe', 'none', 'Europe/Malta', '🇲🇹'),
    ('MU', 'MUS', '480', '{"fr":"Maurice","en":"Mauritius"}', 'africa', 'none', 'Indian/Mauritius', '🇲🇺'),
    ('MV', 'MDV', '462', '{"fr":"Maldives","en":"Maldives"}', 'asia', 'none', 'Indian/Maldives', '🇲🇻'),
    ('MW', 'MWI', '454', '{"fr":"Malawi","en":"Malawi"}', 'africa', 'none', 'Africa/Blantyre', '🇲🇼'),
    ('MX', 'MEX', '484', '{"fr":"Mexique","en":"Mexico"}', 'north_america', 'none', 'America/Mexico_City', '🇲🇽'),
    ('MY', 'MYS', '458', '{"fr":"Malaisie","en":"Malaysia"}', 'asia', 'none', 'Asia/Kuala_Lumpur', '🇲🇾'),
    ('MZ', 'MOZ', '508', '{"fr":"Mozambique","en":"Mozambique"}', 'africa', 'none', 'Africa/Maputo', '🇲🇿'),
    ('NA', 'NAM', '516', '{"fr":"Namibie","en":"Namibia"}', 'africa', 'none', 'Africa/Windhoek', '🇳🇦'),
    ('NC', 'NCL', '540', '{"fr":"Nouvelle-Calédonie","en":"New Caledonia"}', 'oceania', 'none', 'Pacific/Noumea', '🇳🇨'),
    ('NF', 'NFK', '574', '{"fr":"Île Norfolk","en":"Norfolk Island"}', 'oceania', 'none', 'Pacific/Norfolk', '🇳🇫'),
    ('NG', 'NGA', '566', '{"fr":"Nigeria","en":"Nigeria"}', 'africa', 'none', 'Africa/Lagos', '🇳🇬'),
    ('NI', 'NIC', '558', '{"fr":"Nicaragua","en":"Nicaragua"}', 'north_america', 'none', 'America/Managua', '🇳🇮'),
    ('NL', 'NLD', '528', '{"fr":"Pays-Bas","en":"Netherlands"}', 'europe', 'none', 'Europe/Amsterdam', '🇳🇱'),
    ('NO', 'NOR', '578', '{"fr":"Norvège","en":"Norway"}', 'europe', 'none', 'Europe/Oslo', '🇳🇴'),
    ('NP', 'NPL', '524', '{"fr":"Népal","en":"Nepal"}', 'asia', 'none', 'Asia/Kathmandu', '🇳🇵'),
    ('NR', 'NRU', '520', '{"fr":"Nauru","en":"Nauru"}', 'oceania', 'none', 'Pacific/Nauru', '🇳🇷'),
    ('NU', 'NIU', '570', '{"fr":"Niue","en":"Niue"}', 'oceania', 'none', 'Pacific/Niue', '🇳🇺'),
    ('NZ', 'NZL', '554', '{"fr":"Nouvelle-Zélande","en":"New Zealand"}', 'oceania', 'none', 'Pacific/Auckland', '🇳🇿'),
    ('OM', 'OMN', '512', '{"fr":"Oman","en":"Oman"}', 'asia', 'none', 'Asia/Muscat', '🇴🇲'),
    ('PA', 'PAN', '591', '{"fr":"Panama","en":"Panama"}', 'north_america', 'none', 'America/Panama', '🇵🇦'),
    ('PE', 'PER', '604', '{"fr":"Pérou","en":"Peru"}', 'south_america', 'none', 'America/Lima', '🇵🇪'),
    ('PF', 'PYF', '258', '{"fr":"Polynésie française","en":"French Polynesia"}', 'oceania', 'none', 'Pacific/Tahiti', '🇵🇫'),
    ('PG', 'PNG', '598', '{"fr":"Papouasie-Nouvelle-Guinée","en":"Papua New Guinea"}', 'oceania', 'none', 'Pacific/Port_Moresby', '🇵🇬'),
    ('PH', 'PHL', '608', '{"fr":"Philippines","en":"Philippines"}', 'asia', 'none', 'Asia/Manila', '🇵🇭'),
    ('PK', 'PAK', '586', '{"fr":"Pakistan","en":"Pakistan"}', 'asia', 'none', 'Asia/Karachi', '🇵🇰'),
    ('PL', 'POL', '616', '{"fr":"Pologne","en":"Poland"}', 'europe', 'none', 'Europe/Warsaw', '🇵🇱'),
    ('PM', 'SPM', '666', '{"fr":"Saint-Pierre-et-Miquelon","en":"St. Pierre & Miquelon"}', 'north_america', 'none', 'America/Miquelon', '🇵🇲'),
    ('PN', 'PCN', '612', '{"fr":"Îles Pitcairn","en":"Pitcairn Islands"}', 'oceania', 'none', 'Pacific/Pitcairn', '🇵🇳'),
    ('PR', 'PRI', '630', '{"fr":"Porto Rico","en":"Puerto Rico"}', 'north_america', 'none', 'America/Puerto_Rico', '🇵🇷'),
    ('PS', 'PSE', '275', '{"fr":"Territoires palestiniens","en":"Palestinian Territories"}', 'asia', 'none', 'Asia/Hebron', '🇵🇸'),
    ('PT', 'PRT', '620', '{"fr":"Portugal","en":"Portugal"}', 'europe', 'none', 'Europe/Lisbon', '🇵🇹'),
    ('PW', 'PLW', '585', '{"fr":"Palaos","en":"Palau"}', 'oceania', 'none', 'Pacific/Palau', '🇵🇼'),
    ('PY', 'PRY', '600', '{"fr":"Paraguay","en":"Paraguay"}', 'south_america', 'none', 'America/Asuncion', '🇵🇾'),
    ('QA', 'QAT', '634', '{"fr":"Qatar","en":"Qatar"}', 'asia', 'none', 'Asia/Qatar', '🇶🇦'),
    ('RE', 'REU', '638', '{"fr":"La Réunion","en":"Réunion"}', 'africa', 'none', 'Indian/Reunion', '🇷🇪'),
    ('RO', 'ROU', '642', '{"fr":"Roumanie","en":"Romania"}', 'europe', 'none', 'Europe/Bucharest', '🇷🇴'),
    ('RS', 'SRB', '688', '{"fr":"Serbie","en":"Serbia"}', 'europe', 'none', 'Europe/Belgrade', '🇷🇸'),
    ('RU', 'RUS', '643', '{"fr":"Russie","en":"Russia"}', 'europe', 'none', 'Europe/Moscow', '🇷🇺'),
    ('RW', 'RWA', '646', '{"fr":"Rwanda","en":"Rwanda"}', 'africa', 'none', 'Africa/Kigali', '🇷🇼'),
    ('SB', 'SLB', '090', '{"fr":"Îles Salomon","en":"Solomon Islands"}', 'oceania', 'none', 'Pacific/Guadalcanal', '🇸🇧'),
    ('SC', 'SYC', '690', '{"fr":"Seychelles","en":"Seychelles"}', 'africa', 'none', 'Indian/Mahe', '🇸🇨'),
    ('SD', 'SDN', '729', '{"fr":"Soudan","en":"Sudan"}', 'africa', 'none', 'Africa/Khartoum', '🇸🇩'),
    ('SE', 'SWE', '752', '{"fr":"Suède","en":"Sweden"}', 'europe', 'none', 'Europe/Stockholm', '🇸🇪'),
    ('SG', 'SGP', '702', '{"fr":"Singapour","en":"Singapore"}', 'asia', 'none', 'Asia/Singapore', '🇸🇬'),
    ('SH', 'SHN', '654', '{"fr":"Sainte-Hélène","en":"St. Helena"}', 'africa', 'none', 'Atlantic/St_Helena', '🇸🇭'),
    ('SI', 'SVN', '705', '{"fr":"Slovénie","en":"Slovenia"}', 'europe', 'none', 'Europe/Ljubljana', '🇸🇮'),
    ('SJ', 'SJM', '744', '{"fr":"Svalbard et Jan Mayen","en":"Svalbard & Jan Mayen"}', 'europe', 'none', 'Arctic/Longyearbyen', '🇸🇯'),
    ('SK', 'SVK', '703', '{"fr":"Slovaquie","en":"Slovakia"}', 'europe', 'none', 'Europe/Bratislava', '🇸🇰'),
    ('SL', 'SLE', '694', '{"fr":"Sierra Leone","en":"Sierra Leone"}', 'africa', 'none', 'Africa/Freetown', '🇸🇱'),
    ('SM', 'SMR', '674', '{"fr":"Saint-Marin","en":"San Marino"}', 'europe', 'none', 'Europe/San_Marino', '🇸🇲'),
    ('SO', 'SOM', '706', '{"fr":"Somalie","en":"Somalia"}', 'africa', 'none', 'Africa/Mogadishu', '🇸🇴'),
    ('SR', 'SUR', '740', '{"fr":"Suriname","en":"Suriname"}', 'south_america', 'none', 'America/Paramaribo', '🇸🇷'),
    ('SS', 'SSD', '728', '{"fr":"Soudan du Sud","en":"South Sudan"}', 'africa', 'none', 'Africa/Juba', '🇸🇸'),
    ('ST', 'STP', '678', '{"fr":"Sao Tomé-et-Principe","en":"São Tomé & Príncipe"}', 'africa', 'none', 'Africa/Sao_Tome', '🇸🇹'),
    ('SV', 'SLV', '222', '{"fr":"Salvador","en":"El Salvador"}', 'north_america', 'none', 'America/El_Salvador', '🇸🇻'),
    ('SX', 'SXM', '534', '{"fr":"Saint-Martin (partie néerlandaise)","en":"Sint Maarten"}', 'north_america', 'none', 'America/Lower_Princes', '🇸🇽'),
    ('SY', 'SYR', '760', '{"fr":"Syrie","en":"Syria"}', 'asia', 'none', 'Asia/Damascus', '🇸🇾'),
    ('SZ', 'SWZ', '748', '{"fr":"Eswatini","en":"Eswatini"}', 'africa', 'none', 'Africa/Mbabane', '🇸🇿'),
    ('TC', 'TCA', '796', '{"fr":"Îles Turques-et-Caïques","en":"Turks & Caicos Islands"}', 'north_america', 'none', 'America/Grand_Turk', '🇹🇨'),
    ('TF', 'ATF', '260', '{"fr":"Terres australes françaises","en":"French Southern Territories"}', 'antarctica', 'none', 'Indian/Kerguelen', '🇹🇫'),
    ('TH', 'THA', '764', '{"fr":"Thaïlande","en":"Thailand"}', 'asia', 'none', 'Asia/Bangkok', '🇹🇭'),
    ('TJ', 'TJK', '762', '{"fr":"Tadjikistan","en":"Tajikistan"}', 'asia', 'none', 'Asia/Dushanbe', '🇹🇯'),
    ('TK', 'TKL', '772', '{"fr":"Tokelau","en":"Tokelau"}', 'oceania', 'none', 'Pacific/Fakaofo', '🇹🇰'),
    ('TL', 'TLS', '626', '{"fr":"Timor oriental","en":"Timor-Leste"}', 'asia', 'none', 'Asia/Dili', '🇹🇱'),
    ('TM', 'TKM', '795', '{"fr":"Turkménistan","en":"Turkmenistan"}', 'asia', 'none', 'Asia/Ashgabat', '🇹🇲'),
    ('TO', 'TON', '776', '{"fr":"Tonga","en":"Tonga"}', 'oceania', 'none', 'Pacific/Tongatapu', '🇹🇴'),
    ('TT', 'TTO', '780', '{"fr":"Trinité-et-Tobago","en":"Trinidad & Tobago"}', 'north_america', 'none', 'America/Port_of_Spain', '🇹🇹'),
    ('TV', 'TUV', '798', '{"fr":"Tuvalu","en":"Tuvalu"}', 'oceania', 'none', 'Pacific/Funafuti', '🇹🇻'),
    ('TW', 'TWN', '158', '{"fr":"Taïwan","en":"Taiwan"}', 'asia', 'none', 'Asia/Taipei', '🇹🇼'),
    ('TZ', 'TZA', '834', '{"fr":"Tanzanie","en":"Tanzania"}', 'africa', 'none', 'Africa/Dar_es_Salaam', '🇹🇿'),
    ('UA', 'UKR', '804', '{"fr":"Ukraine","en":"Ukraine"}', 'europe', 'none', 'Europe/Kyiv', '🇺🇦'),
    ('UG', 'UGA', '800', '{"fr":"Ouganda","en":"Uganda"}', 'africa', 'none', 'Africa/Kampala', '🇺🇬'),
    ('UM', 'UMI', '581', '{"fr":"Îles mineures éloignées des États-Unis","en":"U.S. Outlying Islands"}', 'oceania', 'none', 'Pacific/Wake', '🇺🇲'),
    ('US', 'USA', '840', '{"fr":"États-Unis","en":"United States"}', 'north_america', 'none', 'America/New_York', '🇺🇸'),
    ('UY', 'URY', '858', '{"fr":"Uruguay","en":"Uruguay"}', 'south_america', 'none', 'America/Montevideo', '🇺🇾'),
    ('UZ', 'UZB', '860', '{"fr":"Ouzbékistan","en":"Uzbekistan"}', 'asia', 'none', 'Asia/Tashkent', '🇺🇿'),
    ('VA', 'VAT', '336', '{"fr":"État de la Cité du Vatican","en":"Vatican City"}', 'europe', 'none', 'Europe/Vatican', '🇻🇦'),
    ('VC', 'VCT', '670', '{"fr":"Saint-Vincent-et-les Grenadines","en":"St. Vincent & Grenadines"}', 'north_america', 'none', 'America/St_Vincent', '🇻🇨'),
    ('VE', 'VEN', '862', '{"fr":"Venezuela","en":"Venezuela"}', 'south_america', 'none', 'America/Caracas', '🇻🇪'),
    ('VG', 'VGB', '092', '{"fr":"Îles Vierges britanniques","en":"British Virgin Islands"}', 'north_america', 'none', 'America/Tortola', '🇻🇬'),
    ('VI', 'VIR', '850', '{"fr":"Îles Vierges des États-Unis","en":"U.S. Virgin Islands"}', 'north_america', 'none', 'America/St_Thomas', '🇻🇮'),
    ('VU', 'VUT', '548', '{"fr":"Vanuatu","en":"Vanuatu"}', 'oceania', 'none', 'Pacific/Efate', '🇻🇺'),
    ('WF', 'WLF', '876', '{"fr":"Wallis-et-Futuna","en":"Wallis & Futuna"}', 'oceania', 'none', 'Pacific/Wallis', '🇼🇫'),
    ('WS', 'WSM', '882', '{"fr":"Samoa","en":"Samoa"}', 'oceania', 'none', 'Pacific/Apia', '🇼🇸'),
    ('YE', 'YEM', '887', '{"fr":"Yémen","en":"Yemen"}', 'asia', 'none', 'Asia/Aden', '🇾🇪'),
    ('YT', 'MYT', '175', '{"fr":"Mayotte","en":"Mayotte"}', 'africa', 'none', 'Indian/Mayotte', '🇾🇹'),
    ('ZA', 'ZAF', '710', '{"fr":"Afrique du Sud","en":"South Africa"}', 'africa', 'none', 'Africa/Johannesburg', '🇿🇦'),
    ('ZM', 'ZMB', '894', '{"fr":"Zambie","en":"Zambia"}', 'africa', 'none', 'Africa/Lusaka', '🇿🇲'),
    ('ZW', 'ZWE', '716', '{"fr":"Zimbabwe","en":"Zimbabwe"}', 'africa', 'none', 'Africa/Harare', '🇿🇼')
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
