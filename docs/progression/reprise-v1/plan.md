# Reprise des données de la v1 (Supabase) : plan

> Rédigé le 27/10/2026. Périmètre arrêté par le commanditaire : **les données Supabase** (activités 2025, et la COP31 de 2026), plus les journées spéciales codées en dur dans la v1 (D14). PACO n'est pas repris (D9). Les années 2021 à 2024 (serveur Laravel) sont une autre reprise, hors périmètre. Les écarts tranchés vivent dans [`ecarts.md`](ecarts.md), un par ligne.

## 1. Ce qu'on sait, et ce qu'on ne sait pas encore

**La structure réelle de Supabase est inconnue.** Le SQL du dépôt v1 (`bank/shema_et_requettes/database_complete.sql`) n'a jamais été rejoué tel quel : il est syntaxiquement invalide, porte du code supprimé depuis, et des migrations ont été jouées directement dans Supabase. Exemples d'écarts probables : `activity_status` à 3 valeurs dans le fichier, 9 en production ; anciennes valeurs d'`activity_theme` (`climate_resilience`, `nature_oceans`…) encore portées par des activités 2025 ; tables utilisées par le code et absentes de tout SQL (`organization_requests`, `event_registrations`, `activity_panelists`). **Rien ne s'écrit avant d'avoir extrait le schéma réel.**

**Volumes indicatifs** (README v1, septembre 2025) : 2 556 utilisateurs, 450 organisations. À remesurer.

**Supabase contient aussi la COP31 (2026)** : l'appel a tourné sur la v1, et elle fait foi. La production v2 porte 1 édition, 13 propositions, 12 séances, 4 organisations, 8 personnes : **données d'essai, effacées à la bascule** sauf le compte du commanditaire (D2).

**Les journées spéciales de la COP30 ne sont pas en base** : leur programme est écrit dans des pages Vue de la v1 (`JourneeJeunesseCOP30.vue`, `JourneeFinance.vue`), qui gonflaient aussi les compteurs de la CdP 30 de « 20 activités, 14 organisations » (constantes posées à la main, qui disparaissent en v2). Deux événements le sont aussi : le séminaire de Larnaka et Nicosie (1er au 3/12/2025) et l'événement parallèle du FPHN (New York, 14/07/2026). Relevé du 27/10 dans [`ops/reprise-v1/journees-speciales/`](../../../ops/reprise-v1/journees-speciales/) : 15 séances pour la journée Jeunesse, 11 pour la Finance, 14 pour Chypre, 5 points pour le FPHN. Aucune page n'appelle Supabase, mais des séances portées par une organisation y sont peut-être aussi comme activités : rapprochement par organisation et titre (D14).

**`docs/database/910_migration_v1.sql` est à moitié écrit et ne tourne pas** : `migrate_activities` est un bouchon ; `propose_organization_clusters` échoue à la première paire floue (clé étrangère vers `org.organizations` posée avec des identifiants v1) ; `migrate_organizations` fusionne en silence par slug avec les fiches de production (dont `ifdd`) et force le type `ngo_association` ; `v_reconciliation` compare `id_map` à lui-même. On garde son principe (quatre temps, `id_map`), on réécrit l'outillage.

**Inventaires complets** : rendus des deux relevés du 27/10 (v1 côté Supabase, modèle cible), résumés aux §§ 4 et 5.

## 2. Principes

1. **Répéter avant de jouer.** Tout se joue d'abord sur une copie locale de la production, autant de fois qu'il faut. La production ne reçoit que la dernière répétition réussie, à l'identique.
2. **Rejouable de bout en bout.** Une commande repart de l'extraction et refait tout. Corriger une règle de correspondance, c'est relancer, jamais retoucher des lignes à la main.
3. **Chaque ligne v1 rend des comptes.** Dans `legacy.id_map` : reprise, fusionnée ou écartée, **avec la raison**. Un écart non expliqué bloque la bascule.
4. **Aucun effet de bord.** Aucun courriel, aucune notification, aucune réunion Zoom créée par la reprise (§ 5).
5. **On n'invente rien, on ne jette rien.** Un champ ou une donnée v1 sans place en v2 **étend le modèle** (règle du commanditaire, 27/10) : SQL d'abord dans `docs/database/`, migration jouée en production avant les données, puis code. Une seule exception : ce que la v2 porte déjà sous une autre forme (un statut v1 qui mêle deux choses, des thèmes en texte libre devenus vocabulaire) se fait correspondre, sans colonne en double. Chaque extension est listée dans `ecarts.md`.
6. **Données personnelles** : extractions dans `sauvegardes/reprise-v1/` (ignoré par Git), jamais dans le dépôt, un journal ou un message.

## 3. Les étapes

| # | Étape | Qui | Résultat |
|---|---|---|---|
| 0 | **Accès** | commanditaire | chaîne de connexion Postgres de Supabase (connexion directe, port 5432) et clé `service_role`, dans `.env` seulement ; accès à l'ancien cPanel pour ses fichiers |
| 1 | **Extraction** | Claude | `pg_dump` du schéma réel (`public`, `auth`, `storage`) et des données ; tous les fichiers des buckets (`epavillonp`, `epavillonv`, `activity-media`, `video-thumbnails`, `documents`) avec leur empreinte ; les fichiers de l'ancien serveur que la base désigne (`images_uploades/`, `documents_uploades/`…), lus par son adresse IP ; le programme des journées spéciales relevé dans le code v1 (`ops/reprise-v1/journees-speciales/`) |
| 2 | **Carte réelle** | Claude | comptage par table ; écarts entre le schéma réel et l'inventaire du dépôt ; valeurs réellement présentes de chaque ENUM ; URL de fichiers par forme (Supabase, ancien serveur, relative, autre) |
| 3 | **Écarts** | ensemble | une décision par écart dans `ecarts.md` (§ 6), une question à la fois |
| 4 | **Outillage** | Claude | `ops/reprise-v1/` : scripts numérotés, rejouables, un par domaine ; `910` corrigé pour ne garder que la zone de transit et `id_map` |
| 5 | **Résolution** | ensemble | doublons d'organisations proposés par groupes (domaine de courriel, nom normalisé, sigle), validés par vous ; personnes rapprochées par courriel, y compris avec celles déjà en production |
| 6 | **Transformation** | Claude | écriture dans les schémas v2, dans l'ordre du § 4 |
| 7 | **Réconciliation** | Claude, puis vous | chaque table v1 : lignes reprises + fusionnées + écartées = lignes extraites ; contrôles visuels (pages d'activités, photos, logos) en administrateur |
| 8 | **Bascule** | Claude, avec votre feu vert | extraction finale, sauvegarde de la production, jeu en production, vérification, réouverture (§ 7) |
| 9 | **Après** | ensemble | redirections des anciennes adresses, courriel aux membres des organisations retenues (D1), suppression de `legacy` hors `id_map`, révocation des clés de la v1 **sauf celles dont PACO a besoin** (D9) |

## 4. Ordre de reprise et correspondances pressenties

Chaque ligne sera confirmée sur la carte réelle (étape 2).

| Ordre | v1 | v2 | Points durs |
|---|---|---|---|
| 1 | `countries` | `reference.countries` | rapprochement par code ISO |
| 1 | valeurs d'ENUM | `reference.taxonomy_terms` | termes manquants : `ace`, `mecanismes_de_cooperation`, `bilan_mondial`, anciennes thématiques (D6) |
| 2 | `auth.users` + `users` | `identity.people` + `accounts` (`password`, sans empreinte, nouveau mot de passe exigé) | comptes repris, empreintes non (D1) ; `email_confirmed_at` → `email_verified_at`, `created_at` et `last_sign_in_at` repris (audit) ; identités Google et Microsoft à voir sur la carte réelle ; rapprochement par courriel avec le compte conservé du commanditaire ; `created_at` faussé par l'import Laravel |
| 2 | `user_roles` | `identity.role_assignments` | `paco` → `standard` ; `negotiator` en portée globale ouvre tous les espaces |
| 2 | `negotiators` | `negotiator_profiles`, `negotiator_designations` | thèmes en texte libre |
| 3 | `organizations` (+ alias, validations) | `org.organizations`, `organization_names`, `organization_domains` | doublons (règle métier n° 1) ; nom unique par pays ; type obligatoire |
| 3 | `users.organization_id` | `org.memberships` | une adhésion active exige une fonction (`job_title`) que la v1 n'avait pas |
| 4 | `events` | `event.events` (séries déjà semées) | l'édition COP31 d'essai est effacée avant (D2) ; PACO écarté (D9) |
| 5 | `activities` | `programme.proposals` (+ `sessions` si retenue) | deux statuts mêlés ; un seul porteur ; dates COP31 décalées de 3 h par l'ancien fuseau (D8) ; insérer **directement à l'état final** |
| 5 | `activity_speakers` | `identity.people` + `proposal_speakers` / `session_speakers` | une ligne par activité, rapprochement par courriel |
| 5 | `activity_documents` | `media.assets` + `proposal_documents` | |
| 5 | `activity_ratings`, `revision_comments` | `programme.reviews`, `proposal_comments` | une revue par évaluateur ; recalcul des moyennes (`refresh_proposal_score`) |
| 5 | journées spéciales (code v1, pas Supabase) | `event.programme_tracks` (`special_day`) + `programme.sessions` sans proposition, `session_tracks`, `session_speakers`, `session_organizations` ; Chypre et FPHN : `event.events` + `event_days` | heures écrites en dur (Belém UTC-3, Chypre UTC+2, New York UTC-4) ; clôtures sans heure de fin ; intervenants sans adresse (D16) ; ce que le modèle ne range pas (D17) |
| 6 | `activities.youtube_link` | `live.streams` (rediffusion) | identifiant ou URL de formes variées ; une vidéo ne sert qu'une séance |
| 7 | `activity_registrations` (+ invités, `session_edition`) | `identity.people` + `programme.registrations` | **déclenche Zoom** (§ 5) ; une seule inscription par séance et personne (invité puis compte = doublon) |
| 8 | fichiers | `media.assets` + `media.attachments` | texte alternatif obligatoire pour une image ; formats imposés (32:9, 16:9, 1:1) ; un même contenu une seule fois |
| 9 | `negotiation_documents` | `negotiation.documents` | `type` et `catégorie` inversés pour ceux venus de Laravel ; fichiers sur l'ancien serveur |
| 9 | `negotiation_sessions`, `francophonie_meetings` | `negotiation.meetings` | place dans les agendas (D10) |
| 10 | `user_testimonials`, `video_testimonials`, `innovations_practices` | `content.highlights` | |
| 10 | `media_gallery` | aucune table dédiée | D7 |

**À l'écart, sauf avis contraire** (D3, revu le 27/10 selon le principe 5) : vecteurs du chatbot (ils se recalculent avec le modèle de la v2, ce ne sont pas des données), réunions Zoom expirées (mots de passe en clair), journaux techniques, PACO (D9). **Tout le reste est repris**, dans son module même fermé : notifications, messagerie, mises en relation, rendez-vous, sondages, lettres d'information et leurs abonnés, historique des courriels, statistiques de consultation, conversations du chatbot, formations.

## 5. Garde-fous techniques

Relevés dans le modèle le 27/10, à câbler dans l'outillage :

- **Plateforme en maintenance et worker arrêté** pendant toute écriture.
- **Purge des essais en premier lot** (D2) : tout ce que l'usage a créé (personnes, comptes, sessions, organisations, édition, appel, propositions, séances, inscriptions, notifications, courriels, médias et leurs objets dans Garage, contenus Guide Négo d'exemple, historique des travaux et de l'outbox), sauf la personne du commanditaire, son compte et ses rôles. Ce que le schéma a semé ou que l'administration règle reste : `platform.settings`, drapeaux, référentiels, taxonomies, séries, rôles et permissions. Liste fermée, table par table, comparée à une base neuve.
- **Retouches de la v2 pour D1**, livrées avec l'outillage et migrées avant les données :
  - modèle : `identity.accounts.password_reset_required_at` ; `ck_accounts_password_shape` admet un compte `password` sans empreinte quand cette date est posée. Un compte sans mot de passe vit ainsi dans le modèle, au lieu d'une empreinte factice qui répondrait plus vite qu'Argon2id et trahirait le compte à la montre ;
  - connexion : rien à changer, un compte sans empreinte paie déjà le hachage factice et rend `invalid_credentials` ;
  - `set_password` efface l'exigence ; la réinitialisation confirmée vaut vérification de l'adresse (le lien est arrivé dans la boîte) ;
  - message de refus, site et Guide Négo, le même pour tous (la connexion ne dit jamais qu'un compte existe) : « L'adresse ou le mot de passe ne correspond pas. Pour des raisons de sécurité, certains comptes doivent choisir un nouveau mot de passe : si c'est votre cas, utilisez « Mot de passe oublié ». » Aucune mention de l'ancienne plateforme.
- **Courriel aux organisations retenues** : le lien de réinitialisation vit une heure, trop peu pour un envoi groupé. Le courriel mène donc à « mot de passe oublié » ; envoi étalé, l'ancien serveur bornant le nombre de messages par heure.
- Écrire sous une personne technique « Reprise v1 » (`app.actor_id`) et une étiquette de lot (`app.request_id`) ; avant de relancer le worker, marquer publiés les événements d'outbox du lot et annuler les travaux `live.*` et de rappel, **en gardant** `media.process_asset`.
- **Une inscription reprise sur une séance diffusée ferait créer une réunion Zoom et écrire aux inscrits** : neutralisation vérifiée par un test avant tout jeu.
- Propositions et séances insérées **à leur état final, dates fournies** : un `UPDATE` d'état réécrit `decided_at` et émet des notifications.
- Séance insérée sans jauge ni formulaire ni clôture, posés après les inscriptions ; sinon le contrôle d'inscription refuse l'historique.
- Fichiers : déposés dans Garage puis déclarés `uploaded`, pour que le worker produise les déclinaisons ; un texte alternatif pour chaque image.
- Aucune fusion silencieuse : un rapprochement avec une ligne de production est une décision de l'étape 5, jamais un `ON CONFLICT`.

## 6. Écarts à trancher

Posés un par un au commanditaire, réponses consignées dans `ecarts.md`.

| | Question | Proposition |
|---|---|---|
| D1 | Mots de passe | **tranché, révisé** : comptes et vérifications repris, nouveau mot de passe exigé (`ecarts.md`) |
| D2 | COP31 et données de la production v2 | **tranché** : la v1 fait foi, la production v2 est purgée sauf le compte du commanditaire |
| D3 | Ce qui n'est pas repris (fin du § 4) | liste courte, sauf avis contraire : vecteurs du chatbot, Zoom expirés, journaux techniques, PACO |
| D4 | Doublons d'organisations : qui valide | Claude propose par groupes, le commanditaire valide |
| D5 | Fonction exigée pour une adhésion active | « Non renseignée » ou adhésion en attente |
| D6 | Thématiques sans équivalent | terme créé ou rapprochement, une par une |
| D7 | Médiathèque d'activité | étendre le modèle : galerie rattachée à l'activité (principe 5) |
| D8 | Dates COP31 saisies avec l'ancien fuseau | correction de 3 h vérifiée sur les journées spéciales connues |
| D9 | PACO | **tranché** : non repris, déporté à part sur Supabase |
| D10 | Sessions de négociation v1 | agenda des sessions, ou réunions de la Francophonie |
| D11 | Anciennes adresses (`/activities/<uuid>`…) | redirection permanente par `id_map` |
| D12 | Annonce aux utilisateurs | un courriel après la bascule, selon D1 |
| D13 | Date limite | à demander |
| D14 | Journées spéciales codées en dur | **tranché** : reprises depuis le code v1 ; Chypre et FPHN, codés de même, suivent sauf avis contraire |
| D15 | PACO est injoignable depuis la bascule DNS du 26/10 (ses pages vivaient sous l'ancien domaine) | urgence selon la prochaine séance PACO |
| D16 | Intervenants sans adresse électronique : toute personne v2 en exige une (`identity.people.primary_email`), et un intervenant de séance est une personne | étendre le modèle pour qu'un intervenant existe sans adresse ; la forme se choisit sur la carte réelle, qui dira combien d'intervenants Supabase sont dans ce cas ; jamais d'adresse fabriquée |
| D17 | Ce que les pages disaient et que le modèle ne range pas : objectifs et public d'une journée, ses « parties », modérateur de la journée, personnes ressources, image de la journée, direct de toute la journée, organisation intervenante | étendre le modèle, une place par information (principe 5) |

## 7. La bascule, geste par geste

1. Maintenance allumée (déjà le cas), worker arrêté.
2. Extraction finale de Supabase et des fichiers.
3. `./deploy.sh backup` de la production.
4. Migration du schéma (retouches D1, extensions du principe 5), purge des données d'essai (D2), puis jeu de l'outillage, lot par lot, chacun dans sa transaction.
5. Réconciliation : zéro écart non expliqué, sinon restauration de la sauvegarde.
6. Neutralisation de l'outbox et des travaux du lot, puis relance du worker (traitement des fichiers).
7. Vérification en administrateur, puis réouverture.
8. Redirections et courriel aux utilisateurs.
