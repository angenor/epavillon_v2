# Progression

**Ce fichier est la mémoire du projet entre deux sessions Claude Code.**

Le contexte d'une session se perd ; le dépôt reste. Toute session commence par lire ce fichier et se termine par le mettre à jour. Une session qui ne le fait pas oblige la suivante à tout redécouvrir.

**Dernière mise à jour** : 16 août 2026 — composants d'interface et guide de style (A0.4).

---

## État général

| Domaine | État |
|---------|------|
| Modèle de données | ✅ Complet et validé — 18 fichiers, 142 tables, chargement vérifié sur PostgreSQL 17 + pgvector |
| Cadrage et décisions | ✅ Complet — 14 ADR, périmètre du jalon arrêté. Audité le 16/08, corrigé |
| Environnement local | ✅ Monté et vérifié le 16/08 — `ops/docker-compose.dev.yml`, `ops/garage.toml`, `Makefile`, `.gitignore`, `.env.example`. Cinq services démarrés, schéma chargé, `make check` au vert |
| Front | 🟨 Socle posé le 16/08 — Nuxt 4.5, TypeScript strict, Tailwind v4, Pinia, i18n fr/en, jetons de design, deux layouts. **Types du modèle dérivés le 16/08** : 14 fichiers, 62 tables du jalon, 2 vues, 8 retours de fonction, puis alignés le 16/08 sur les deux corrections du modèle (`AdministeredEvents`, `ProposalDashboardRow`). **Données simulées le 16/08** : 22 fichiers sous `app/mocks/`, 13 organisations dont un doublon, 25 personnes, 40 propositions, 32 revues, 30 sessions, 60 inscriptions ; couche d'accès `useApi()` réécrite. **Bibliothèque d'interface le 16/08** : 38 composants sous `app/components/ui/`, les 27 du prompt plus 11 briques ; guide de style vivant à `/style-guide`, 7 sections ancrées, rendu sur données réelles. `make check-front` au vert. Aucun écran métier encore |
| API | ⬜ Rien de commencé |

### Ce qui a été vérifié le 16/08, et comment

| Contrôle | Résultat |
|---|---|
| `docker compose up -d` — cinq services | Démarrés ; images `pgvector/pgvector:pg17`, `valkey:8-alpine`, `jaeger:1.60`, `mailpit:latest`, `garage:v1.0.1` |
| Chargement intégral du schéma | `docker compose logs postgres` : **zéro `ERROR:` / `FATAL:` / `PANIC:`** hors messages bénins du healthcheck. Les deux `NOTICE` attendus de `900_seed.sql` sont présents, dont « Frontières de modules conformes » |
| Schémas attendus | 15 présents (`legacy` compris) ; **142 tables** hors partitions — le compte annoncé, à condition de compter les tables partitionnées parentes |
| Extensions | `pgcrypto`, `citext`, `pg_trgm`, `unaccent`, `btree_gist`, `vector`, `pg_stat_statements` ; `shared_preload_libraries = pg_stat_statements` effectif |
| `cross_module_fk_report WHERE NOT is_compliant` | **0** |
| `analytics.refresh_all(true)` | 7 vues matérialisées rafraîchies, **0 échec** dans `refresh_log` |
| Le portail échoue-t-il vraiment ? | Testé : ligne d'échec factice insérée dans `refresh_log` → `make check-db-safe` sort en **code 2**. Ligne retirée ensuite |
| `make check` de bout en bout | Vert, code de sortie 0 (`down -v` → rechargement complet → assertions) |
| Mailpit | Interface HTTP 200 ; courriel envoyé sur `localhost:1025` et **capturé** (1 message, sujet correct) |
| Jaeger | Interface HTTP 200 ; ports OTLP 4317/4318 publiés |
| Valkey | `PING` → `PONG` |
| Garage | Layout assigné, bucket `epavillon` créé, clé `epavillon-dev` en lecture/écriture. **Écriture réelle prouvée** : `PUT` puis `GET` d'un objet signé SigV4 → 200 / 200 |

---

## Journal

Une ligne par session. La plus récente en haut. Court : ce qui a été fait, ce qui bloque, ce qui vient.

| Date | Session | Fait | À suivre |
|------|---------|------|----------|
| 2026-08-16 | Composants d'interface et guide de style (A0.4) | **38 composants sous `frontend/app/components/ui/`** — les 27 du prompt, plus 11 briques que leur écriture a rendues nécessaires : `Icon` (30 pictogrammes en trait, aucun emoji), `Spinner`, `FormField` (l'enveloppe qui tient `aria-describedby` pour tous les champs), `ThemeToggle` et `LocaleSwitch` (la duplication que la note du layout admin demandait de résorber), `ZonedTime`, `LiveBadge`, `CapacityMeter`, `IncidentBanner`, plus les deux composants métier `SessionCard` et `StatusTimeline`. **Chaque composant porte ses états** : repos, survol, focus clavier, actif, désactivé, chargement ; les champs y ajoutent vide, rempli, erreur avec message, aide contextuelle et lecture seule — `readonly` et `disabled` traités comme deux choses différentes, ce qu'ils sont. **Deux fichiers de types** : `types/ui.ts` (vocabulaire des composants) et `types/live.ts` (les incidents seuls du module Direct, motif transverse du jalon). **Un composable** : `useLiveSession()`, qui tient la règle « un seul direct » PAR CONSTRUCTION plutôt que par discipline. **Un utilitaire** : `utils/contrast.ts`, qui calcule les rapports WCAG à l'affichage au lieu de les recopier. **Les deux layouts refactorisés** pour consommer `UiNavBar`, `UiSideNav`, `UiBreadcrumb`, `UiThemeToggle` et `UiLocaleSwitch`. **`frontend/app/pages/style-guide.vue`** : 7 sections ancrées (jetons, base, formulaires, données, navigation, métier, motifs), 6 sections déportées en composants pour rester lisibles, **données réelles chargées par `useApi()`** — 12 lignes de tableau, 6 cartes de séance, thématiques et pays résolus depuis la base. 14 fichiers de traduction (7 × 2 locales). Vérifié : `make check-front` vert, rendu serveur en 200, **zéro clé brute en fr comme en en**, « 14:30 — 16:00 (heure de Belém, UTC−3) » produit tel quel, une seule pastille « en direct » sur les six cartes, `data-theme="dark"` posé dès le rendu | Prompt A1 — authentification. **Deux écarts du modèle à trancher avant A3 : le pays de l'organisation et les libellés de thématiques absents de `v_public_schedule`** (n° 14 et 15 ci-dessous) |
| 2026-08-16 | Données simulées (A0.3) | **22 fichiers sous `frontend/app/mocks/`**, tous typés `satisfies` contre les types d'A0.2 : `ids.ts` (identifiants partagés, UUID v7 valides construits de façon déterministe), `reference`, `org`, `memberships`, `people`, `event`, `rooms`, `tracks`, `calls`, `criteria`, `proposals/` (7 fichiers), `reviews`, `sessions/` (6 fichiers), `registration-form`, `registrations`, `views`, `conflicts`, `index`. **Tout le contenu demandé y est** : COP31 Belém 9–20/11/2027 et ses 12 jours, appel ouvert (prolongé), 6 critères dont un éliminatoire, 3 salles dont une virtuelle, 1 canal, 2 journées spéciales, 13 organisations **dont deux fiches pour la même entité** (OSED, nom complet + sigle, même domaine), 25 personnes dont une administratrice limitée à la COP31 et une sans aucun droit, 40 propositions couvrant **les 8 statuts** avec 5 co-organisées, 30 sessions dont **2 en conflit de salle et 2 en conflit de diffusion**, 60 inscriptions à canaux d'acquisition variés. Vocabulaires, libellés et codes repris **à l'identique** de `020_reference.sql`, `030_identity.sql`, `060_events.sql` et `900_seed.sql`. **`useApi()` réécrit** : chaque appel déclare au même endroit sa route d'API et sa lecture simulée, bascule par `NUXT_PUBLIC_API_BASE`, import dynamique des mocks, filtrage par périmètre d'administration avec `ForbiddenError`. Vérifié : `npm run typecheck` à 0 erreur (couverture des mocks prouvée par une erreur introduite puis retirée), exécution complète des 22 modules, intégrité référentielle contrôlée sur 9 familles de liens, `make check-front` vert | Prompt A0.4 — composants d'interface et guide de style. Deux points à trancher, voir « Écarts » : la portée du conflit « stand unique » et le format de la réponse à un champ `country` |
| 2026-08-16 | Traitement des neuf écarts du modèle | Les neuf écarts d'A0.2 classés en trois piles et **tous soldés** : deux corrigés dans le SQL, trois transformés en questions au commanditaire, quatre inscrits dans les prompts B. **Corrections** : `identity.administered_events()` ne renvoie plus de `NULL` (`030_identity.sql`) et `v_proposal_dashboard` expose `title` (brut) et `title_text` (résolu) au lieu d'un seul `title` de type flottant (`070_programme_proposals.sql`). Base rechargée de zéro (`down -v`), `make check` vert, comportement **vérifié sur les trois cas** — administrateur global, d'un seul événement, sans aucun droit — ainsi que le filtre `is_global OR event_id = ANY(event_ids)`, qui ne renvoyait rien *sans erreur* auparavant. Types répercutés (`identity.ts`, `views.ts`), `npm run typecheck` à 0 erreur. Exigences d'API écrites dans `PROMPTS_DEVELOPPEMENT.md` (B0 principe 5, B3, B4, B5) | Trois arbitrages attendus du commanditaire (voir « Points bloqués ») ; ils ne bloquent ni A0.3 ni A0.4. Prompt A0.3 — données simulées |
| 2026-08-16 | Types dérivés du modèle (A0.2) | Lecture intégrale de `020`, `030`, `040`, `050`, `060`, `070`, `075`. **14 fichiers de types** créés sous `frontend/app/types/` — `reference`, `identity`, `org`, `media`, `event/{series,edition,venue,call}`, `programme/{proposal,review,session,registration}`, `views`, plus `index.ts` qui ne fait que ré-exporter ; `shared.ts` complété (alias d'identifiants, `Numeric`, `Int8`, `TsTzRange`, codes de taxonomie et de permission). **Les 62 tables des sept fichiers SQL sont couvertes**, champ pour champ. Vérifié mécaniquement, pas à l'œil : un script compare les colonnes déclarées dans les `CREATE TABLE` aux champs des interfaces, puis les `NOT NULL` aux `\| null` — 6 écarts, tous des omissions volontaires, et 2 nullabilités resserrées sciemment (voir plus bas). `make check-front` au vert | Prompt A0.3 — données simulées, à écrire **contre ces types** (`satisfies Proposal[]`) plutôt qu'à main levée |
| 2026-08-16 | Socle du frontend (A0.1) | `frontend/` créé : Nuxt 4.5.2, TypeScript strict, TailwindCSS v4 (greffon Vite, sans fichier de configuration JS), Pinia, @nuxtjs/i18n 10.6. **Jetons de design** dérivés de la charte, échelles de nuances **calculées en OKLCH** et annotées de leur ratio de contraste WCAG (`design-tokens.css`, 460 lignes). Thème clair/sombre par cookie, appliqué dès le rendu serveur. Layouts `public` et `admin`. Traductions découpées par écran avec agrégation automatique (`modules/i18n-messages.ts`). Utilitaires `resolveI18nText()` et formatage de dates avec fuseau. `useApi()` déclaré, inutilisé. `make check-front` au vert (typecheck + build) | Prompt A0.2 — types TypeScript dérivés du SQL |
| 2026-08-16 | Environnement local | `ops/docker-compose.dev.yml`, `ops/garage.toml`, `Makefile`, `.gitignore`, `.env.example` créés ; quatre `.DS_Store` retirés de l'index ; les cinq services démarrés et vérifiés un par un ; Garage initialisé et écriture S3 prouvée ; `ENVIRONNEMENT_LOCAL.md` synchronisé avec les fichiers réels | Prompt A0.1 — socle Nuxt, Tailwind, i18n, jetons de design |
| 2026-08-16 | Audit documentaire | Revue des 10 fichiers Markdown ([AUDIT_DOCUMENTATION.md](AUDIT_DOCUMENTATION.md)). Restauration de `docs/historique/`, correction des vestiges de décisions retournées, recomptage du modèle, préambule ajouté à tous les prompts | Trancher les arbitrages listés plus bas, puis monter l'environnement local |
| 2026-08-16 | Mise en place | Dossier de projet créé, documentation réorganisée, `CLAUDE.md` et index du modèle écrits | Monter l'environnement local, puis produire le guide de style |

---

## Front — suivi des prompts

Cocher au fur et à mesure. Les prompts correspondants sont dans [PROMPTS_DEVELOPPEMENT.md](PROMPTS_DEVELOPPEMENT.md).

| Prompt | Écran | État | Notes |
|--------|-------|------|-------|
| A0.1 | Socle Nuxt, Tailwind, i18n, jetons | ✅ | Fait le 16/08. Écarts au prompt et pièges rencontrés : voir plus bas |
| A0.2 | Types TypeScript dérivés du SQL | ✅ | Fait le 16/08 ; **neuf écarts relevés, tous soldés le même jour** (deux corrigés dans le SQL, trois en attente d'arbitrage, quatre reportés aux prompts B). Périmètre du jalon : `reference`, `identity`, `org`, `media`, `event`, `programme`, les deux vues. Modules hors jalon (Direct, Publications, Négociations, Engagement, Formations, Outils, Analytique) non couverts — ils viendront avec leurs écrans |
| A0.3 | Données simulées | ✅ | Fait le 16/08. 22 fichiers, découpés par entité puis par statut (propositions) et par état de publication (sessions). Trois fichiers hors de l'arborescence du prompt : `memberships.ts` (volume), `registration-form.ts`, `views.ts` et `conflicts.ts` (les deux vues et `detect_conflicts()` reconstitués). Écart de périmètre assumé : les revues détaillées couvrent 13 dossiers sur les 27 notés — voir plus bas |
| A0.4 | Composants d'interface + page de guide de style | ✅ | Fait le 16/08. 38 composants dans `app/components/ui/` — les 27 demandés, plus 11 briques nécessaires à leur écriture (voir le journal). Guide de style à `/style-guide`, 7 sections ancrées, sur données réelles via `useApi()`. Deux écarts modèle/interface relevés (n° 14, 15) et deux points de vocabulaire tranchés (n° 16, 17) |
| A1 | Authentification | ⬜ | |
| A2 | Rattachement à une organisation | ⬜ | Écran critique — qualité du référentiel |
| A3 | Page publique de l'événement | ⬜ | |
| A4 | Formulaire de soumission | ⬜ | |
| A5 | Espace organisation | ⬜ | |
| A6 | Tableau de bord back-office | ⬜ | |
| A7 | Liste des propositions | ⬜ | |
| A8 | Fiche d'évaluation | ⬜ | Écran le plus dense |
| A9 | Planificateur de créneaux | ⬜ | |
| A10 | Gestion des événements | ⬜ | |
| A11 | Organisations et fusion | ⬜ | |
| A12 | Utilisateurs et rôles | ⬜ | |
| A13 | Messages d'incident | ⬜ | |
| A14 | Page « En cours de maintenance » | ⬜ | |

## API — suivi des prompts

| Prompt | Module | État | Notes |
|--------|--------|------|-------|
| B0 | Constitution Spec Kit | ⬜ | |
| B1 | Socle + Identité | ⬜ | |
| B2 | Organisations | ⬜ | |
| B3 | Événements | ⬜ | |
| B4 | Propositions | ⬜ | |
| B5 | Sessions | ⬜ | |
| B6 | Média + Engagement | ⬜ | |
| B7 | Raccordement du front | ⬜ | |

---

## Modifications du modèle de données

Toute modification d'un fichier de `docs/database/` se note ici. C'est ce qui permet de savoir, plus tard, pourquoi le schéma diffère de ce que décrit le cadrage.

| Date | Fichier | Changement | Motif |
|------|---------|------------|-------|
| 08-16 | `030_identity.sql` | `administered_events()` : `COALESCE` sur les deux agrégats (`false`, `'{}'`) et `array_agg(DISTINCT …)` ; en-tête et `COMMENT ON` réécrits | Écart n°5. La fonction agrège sans `GROUP BY` : sans aucune attribution, `is_global` **et** `event_ids` valaient `NULL`. Deux conséquences — un garde testant `NOT is_global` confondait « aucun droit » et « administrateur d'un événement », et `event_id = ANY(NULL)` vaut `NULL`, donc le filtre d'accès **ne renvoyait rien sans lever d'erreur**. Le `DISTINCT` évite qu'une personne cumulant `admin` et `programmer` sur la même édition la voie comptée deux fois |
| 08-16 | `070_programme_proposals.sql` | `v_proposal_dashboard` : `title` expose désormais le `i18n_text` brut, la valeur résolue passe sous `title_text` ; deux `COMMENT ON COLUMN` ajoutés | Écart n°1. Le même nom de champ portait un `text` dans la vue et un `i18n_text` sur la table ; `resolveI18nText()` appliqué dessus rendait une chaîne vide **sans erreur**. `title` désigne maintenant partout la même chose — la table, `v_public_schedule` et cette vue — et la liste du back-office change de langue sans requête. La valeur résolue reste disponible pour ce que le JSON ne sait pas faire : trier, filtrer et exporter en SQL |
| 08-16 | `075_programme_sessions.sql` | Suppression des contraintes d'exclusion `ex_sessions_no_room_overlap` et `ex_sessions_no_broadcast_overlap` ; remplacées par `detect_conflicts()` et `publication_readiness()` | Le commanditaire : les chevauchements ne doivent pas être bloqués, l'admin réorganise par glisser-déposer |
| 08-16 | `060_events.sql` | Index unique `ux_calls_one_per_event` — un seul appel à propositions par édition | Une version antérieure en autorisait plusieurs ; le commanditaire a tranché |
| 08-16 | `070` et `075` | Ajout de `proposal_organizations` et `session_organizations` (porteur principal, co-organisateurs, partenaires, soutiens) | La co-organisation n'existait pas dans le modèle ; elle est courante en réalité |
| 08-16 | `040_organizations.sql` | Colonne `dedupe_on` dans `organization_references` + déduplication dans `merge_organizations()` | Sans elle, la fusion échouait dès que deux organisations co-organisaient la même activité |
| 08-16 | `060_events.sql` | Ajout de `event.programme_tracks` + `programme.session_tracks` ; `event_days` ramené à son rôle de calendrier | Une journée spéciale n'occupe pas forcément le jour entier et peut déborder sur deux jours |
| 08-16 | `060` et `075` | Ajout de `event.broadcast_channels` et de `sessions.broadcast_channel_id` | Règle « un seul direct à la fois » : le canal devient une ressource réservable |
| 08-16 | `030_identity.sql` | Rôle `admin` attribuable sur la portée `event` + `administered_events()` + trigger de contrôle de portée | Confier un événement à un responsable sans lui ouvrir le reste (cas rencontré en v1) |
| 08-16 | `010_platform.sql` | Ajout de `entity_history()` ; historique dérivé du journal d'audit | Le commanditaire demande l'historique des modifications d'activité |
| 08-16 | `125_training.sql` | Nouveau module Formations (12 tables) | Remonté au produit minimum viable à la demande du commanditaire |
| 08-16 | `030_identity.sql` | `privacy_requests.due_at` passé de colonne générée à `DEFAULT` | `timestamptz + interval` est STABLE, donc interdit dans une expression `GENERATED` |

---

## Écarts constatés entre le modèle et l'interface

À remplir pendant la phase front. Chaque écart est soit un défaut du modèle, soit un défaut de l'interface — il se tranche, il ne se contourne pas par une conversion.

| Écart | Où | Décision |
|-------|----|----------|
| — | — | Aucun écart au 16/08 (A0.1) : le socle ne consomme encore aucune table. Les deux points de contact avec le modèle ont été alignés sur le SQL — `resolveI18nText()` reproduit l'ordre de repli de `platform.t()` (locale, langue de base, français, première valeur), et les utilitaires de date exigent un fuseau explicite, comme l'impose le commentaire de `event.events.timezone` |

### Écarts relevés en écrivant les composants d'interface (A0.4, 16/08)

Quatre points. Les deux premiers sont de vrais manques de la vue de programmation et se paieront sur l'écran A3 ; les deux derniers sont des écarts entre le PROMPT et le MODÈLE, tranchés en faveur du modèle, qui fait foi.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **14** | **`v_public_schedule` ne joint pas le PAYS de l'organisation** — elle expose `organization_name` et `organization_acronym`, rien de plus | `075` § 6 | Le prompt A0.4 demande explicitement « organisation avec pays » sur la carte de séance, et c'est justifié : sur une COP, le pays situe l'organisation aussi sûrement que son nom. Sans lui dans la vue, chaque écran de programmation doit charger la table des organisations ET celle des pays pour un seul mot — ou renoncer. La vue perd alors sa raison d'être : répondre à un écran en une requête | **Contourné, pas résolu.** `UiSessionCard` reçoit `organizationCountry` en propriété séparée, et le guide de style le compose depuis `organizations` et `countries`. **À trancher avant A3** : ajouter `organization_country_code` (ou le nom résolu) à la vue. Modification du SQL, donc rechargement de la base |
| **15** | **La vue agrège COMPLÈTEMENT les journées spéciales mais seulement les CODES des thématiques** — `tracks` porte `slug`, `title`, `color` et `kind` ; `theme_codes` ne porte que des codes | `075` § 6 | Asymétrie sans raison apparente. Les pastilles thématiques ont besoin d'un libellé et d'une couleur, tous deux dans `reference.taxonomy_terms` : l'écran doit donc charger la taxonomie et faire la correspondance lui-même. Ce n'est pas coûteux — la taxonomie se charge une fois — mais c'est une jointure qu'un écran refait alors que la vue la fait déjà pour les fils | **Contourné.** `UiSessionCard` reçoit `themes: ThemeBadge[]`, résolus par l'appelant. À arbitrer avec le n° 14 : les deux se corrigent dans la même passe sur la vue |
| **16** | **Le prompt demande SIX états temporels, le modèle en calcule CINQ** — « à venir / en cours / en direct / terminé / reporté / annulé » contre `upcoming`, `ongoing`, `past`, `postponed`, `cancelled` | `075` § 6 vs prompt A0.4 | Aucun, une fois le point compris : « en direct » N'EST PAS un état temporel. Il ne dépend pas de l'heure mais de la diffusion — `sessions.status = 'live'`, un canal réservé, une équipe technique. Une séance en cours n'est pas nécessairement diffusée, et une séance diffusée reste « en cours » au sens du calendrier. Les confondre aurait mis le repère « en direct » sur toutes les séances de 14 h | **Modèle retenu.** `UiSessionCard` rend les cinq états de la vue ; le repère « en direct » est un composant à part (`UiLiveBadge`) adossé à `useLiveSession()` |
| **17** | **Le prompt demande TROIS niveaux de gravité d'incident, le modèle en déclare QUATRE** — `info`, `warning`, `error`, `critical` | `080` § 5 | Rien, sinon la tentation de fondre `error` et `critical`. Or ils ne se traitent pas pareil : le guide en fait deux rendus distincts, et **aucun des deux n'est refermable**, quoi qu'en dise `is_dismissible`. Un incident majeur qu'on chasse d'un clic est un incident qui ne sera pas lu | **Modèle retenu.** Les quatre gravités sont rendues. `critical` se distingue par un trait de gauche épais, pas par une cinquième couleur — la palette n'a que quatre familles d'état |

### Écarts relevés en écrivant les données simulées (A0.3, 16/08)

Quatre points, de natures différentes. Les deux premiers engagent le modèle ou l'API et ne se tranchent pas depuis un fichier de mocks ; les deux derniers sont des choix de jeu de données, assumés et réversibles.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **10** | **Le conflit « stand unique » vise TOUTE paire de séances simultanées de l'édition**, y compris une séance en ligne tenue dans une salle virtuelle — qui n'occupe pourtant aucun mètre carré du pavillon | `075` § 7, `detect_conflicts()`, branche `venue_capacity` | Reproduit sur les mocks : la séance en ligne « La relève francophone » du 17/11 remonte en gravité `blocking` face à une séance physique du même créneau. Le planificateur (A9) affichera un conflit bloquant qui n'en est pas un, et l'équipe apprendra à ignorer le bandeau d'alerte — exactement ce qu'une alerte ne doit jamais devenir | **À trancher.** Piste : restreindre la branche `venue_capacity` aux séances dont la salle est physique (`enforce_room_exclusivity`) ou dont le format n'est pas `online`. Modification du SQL, donc rechargement de la base : à faire avant A9, pas pendant |
| **11** | **Le format de la réponse à un champ de type `country` n'est fixé nulle part** — `registration_form_fields.field_type = 'country'`, et `registrations.answers` est un `jsonb` libre | `075` § 3 et 4 | Deux implémentations divergentes sont possibles (code ISO 3166-1 alpha-2 ou `uuid` de `reference.countries`), et rien ne les départage. Un export mélangeant les deux formes est irrécupérable | **Reporté au prompt B5** : le contrat d'API fixe la forme et la valide. Les mocks retiennent **l'ISO2** (`"SN"`), lisible et stable ; la décision reste à confirmer |
| **12** | **Le détail des revues ne couvre que 13 des 27 dossiers notés** | `app/mocks/reviews.ts` | Aucun écran du jalon ne le voit : la fiche d'évaluation (A8) travaille sur les dossiers en cours, tous couverts. Écrire les 39 revues manquantes représentait 700 notes à la main, illisibles et sans lecteur | Choix de jeu de données, documenté en tête de `reviews.ts`. Un contrôle exécuté au chargement du module compare la moyenne des revues aux agrégats de la proposition et **lève une erreur en cas de divergence** : la cohérence est garantie là où le détail existe |
| **13** | **L'appel à propositions est ouvert alors qu'une partie du programme est déjà publiée** | `app/mocks/event.ts`, `calls.ts` | Aucun ; c'est le fonctionnement réel — l'IFDD publie une première version du programme dès les premières décisions et continue de recevoir des dossiers pour les créneaux restants. Les écrans doivent tenir les deux états à la fois | Retenu et documenté en tête de `event.ts`. À ne pas « corriger » en fermant l'appel : ce serait perdre le cas |

### Écarts relevés en dérivant les types (A0.2, 16/08)

Neuf points où le modèle et ce que l'interface demanderait naturellement ne coïncidaient pas. **Traités le 16/08 ; aucune ligne ne reste sans suite.** Ils n'étaient pas de même nature, et les traiter d'un seul geste aurait produit cinq décisions produit inventées. D'où trois piles :

- **Défaut du modèle** (n°1, n°5) → corrigé dans le SQL, base rechargée, comportement vérifié.
- **Arbitrage du commanditaire** (n°2, n°8, n°9) → question rédigée dans « Points bloqués ou en attente ». Ne pas trancher seul.
- **Obligation d'API** (n°3, n°4, n°6, n°7) → exigence écrite dans le prompt B concerné, là où elle sera lue au moment utile.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **1** | **La vue du back-office renvoie un titre DÉJÀ RÉSOLU** — `v_proposal_dashboard.title` vaut `platform.t(p.title)`, un `text`, là où `proposals.title` est un `i18n_text` | `070` § 7 | Le même champ porte deux types selon sa source. La liste des propositions ne peut pas changer de langue sans requête, et un développeur qui applique `resolveI18nText()` dessus obtient une chaîne vide sans erreur | **Corrigé le 16/08**, voir `070_programme_proposals.sql` § 7 : `title` expose le JSON brut, `title_text` la valeur résolue. Retenu parce que `title` désigne alors la même chose partout — table, `v_public_schedule`, cette vue — pendant que le tri, le filtrage et l'export SQL gardent ce dont ils ont besoin. Répercuté dans `views.ts` |
| **2** | **Le nom d'une organisation n'est pas multilingue** — `org.organizations.legal_name` est un `text` | `040` § 1 | La programmation publique en anglais affiche un nom légal français. Les traductions existent (`organization_names`, `kind = 'translation'`, avec `locale` et `is_confirmed`) mais ni `v_public_schedule` ni la fiche ne les joignent | **En attente d'arbitrage** — voir « Points bloqués ou en attente » |
| **3** | **Les thématiques d'une proposition sont hors de la proposition** — elles vivent dans `reference.entity_terms`, polymorphe et sans clé étrangère vers `proposals` | `020` § 4 | Aucun type ne peut relier `entity_id` à `Proposal.id` : le formulaire de soumission fait un aller-retour sur une table générique, et rien ne l'empêche d'écrire un mauvais couple `(schéma, table)` | **Reporté au prompt B4** : le triplet `('programme','proposals',id)` est fixé par le service et jamais accepté depuis la requête ; lecture par `terms_of()`, purge à la suppression |
| **4** | **La machine à états est une donnée que rien n'expose** — `proposal_transitions_allowed` dit quelles actions sont possibles, mais aucune vue ne la joint à la proposition | `070` § 1 | Sans point d'accès dédié, le front réimplémentera le graphe des transitions en dur : exactement ce que la mise en données voulait éviter | **Reporté au prompt B4** : l'API expose les transitions autorisées pour cette proposition **et** cette personne, motif exigé compris |
| **5** | **`administered_events()` a trois réponses pour une question binaire** — la fonction agrège sans `GROUP BY` : elle renvoie toujours une ligne, et sans aucune attribution `is_global` **et** `event_ids` valent `null` | `030` § 3 | Un garde de navigation qui teste `if (!scope.is_global)` traite « aucun droit » comme « administrateur d'événement » et affiche une liste vide au lieu d'un refus | **Corrigé le 16/08**, voir `030_identity.sql` § 3 : `(false, '{}')` pour « aucun droit », `DISTINCT` sur les éditions. Vérifié sur les trois cas et sur le filtre `= ANY(...)`, qui se taisait au lieu de filtrer. Répercuté dans `identity.ts` |
| **6** | **Les réponses d'inscription ne sont pas typables** — `registrations.answers` est un `jsonb` dont les clés sont les `code` de `registration_form_fields` | `075` § 4 | `Record<string, unknown>` est le seul type honnête ; toute la validation est dynamique, contre le formulaire chargé | **Reporté au prompt B5** : conséquence voulue du formulaire configurable, à ne pas « améliorer » par un type figé. Le trigger ne contrôle que les réponses obligatoires, la clôture et la jauge — types, options et règles incombent à l'API |
| **7** | **Trois colonnes de `sessions` sont écrites par trigger** — `time_range`, `enforce_room_exclusivity`, et `broadcast_channel_id` quand `is_streamed` passe à vrai | `075` § 1 | Un formulaire qui les envoie verra ses valeurs écrasées sans avertissement. Marquées `readonly` dans les types pour les deux premières ; la troisième ne peut pas l'être, elle est saisissable | **Reporté au prompt B5** : le contrat d'écriture ne les contient pas ; une tentative d'écriture donne une erreur nommant le champ, jamais un écrasement silencieux |
| **8** | **Aucune vue « mes propositions »** — l'espace organisation (A5) et la page publique d'une session (A3) devront composer leurs jointures | `070`, `075` | Deux écrans du jalon n'ont pas d'équivalent de `v_proposal_dashboard` ni de `v_public_schedule` : le N+1 est à éviter côté API, pas côté front | **En attente d'arbitrage** — voir « Points bloqués ou en attente » : ce qu'une organisation a le droit de voir de son dossier détermine la vue |
| **9** | **Le numéro de dossier dépend d'un champ facultatif** — `reference_code` est préfixé par `events.acronym`, nullable, avec repli sur les huit premiers caractères du slug | `070` § 2 | Une édition sans sigle donne un préfixe tronqué et pénible à épeler au téléphone. **Reproduit en base le 16/08** : slug `cop31-test` → dossier « COP31-TE-00001 » | **En attente d'arbitrage** — voir « Points bloqués ou en attente ». Piste inscrite au prompt B3 (contrainte dans le formulaire, pas dans la base) ; `acronym` reste nullable tant que l'arbitrage n'est pas rendu |

### Écarts entre le prompt A0.1 et ce qui a été livré

Trois libertés prises, toutes documentées dans le code concerné.

| Ce que demandait le prompt | Ce qui a été fait | Pourquoi |
|---|---|---|
| « Ne crée ni composant d'interface, ni page » | Une page d'amorçage `app/pages/index.vue`, dix lignes | Sans aucune route, Nuxt ne monte ni routeur, ni layout, ni i18n d'URL : `npm run build` réussit sans rien exercer et `make check` ne prouve rien. La page n'utilise que ce que le socle fournit et sera **remplacée** par le prompt A3, pas complétée. Son en-tête le dit |
| « Chaque locale a un point d'entrée qui agrège l'arborescence avec `import.meta.glob` » | Un module Nuxt local, `frontend/modules/i18n-messages.ts`, qui écrit l'agrégation dans `i18n/locales/.generated/` | `import.meta.glob` ne fonctionne pas au rendu serveur — voir le piège détaillé ci-dessous. L'objectif du prompt est tenu : ajouter un écran reste « ajouter un fichier JSON », sans toucher ni à `nuxt.config.ts` ni aux points d'entrée de locale |
| Deux layouts, `public` et `admin` | Idem, plus : `public` est le layout **par défaut**, choisi dans `app.vue` | Nuxt attend un layout nommé `default` ; il n'y en a pas. `app.vue` retient `route.meta.layout ?? 'public'`, ce qui évite qu'une page sans `definePageMeta` se retrouve sans cadre |

### Pièges rencontrés et ce qu'ils ont coûté

| Symptôme | Cause | Correction |
|---|---|---|
| La page se rend **avec ses clés brutes** (« nav.site.name ») en production, alors que le mode développement est correct | `import.meta.glob` dans un fichier de locale : Nitro compile ces fichiers hors du pipeline Vite et remplace `import.meta` par `globalThis._importMeta_`, qui n'a pas de `glob`. Aucun avertissement à la construction, et les alertes de clé manquante de vue-i18n sont muettes en production | `modules/i18n-messages.ts` génère des imports statiques. Le piège est raconté dans son en-tête pour qu'il ne soit pas réintroduit |
| Deuxième tentative : template dans `.nuxt/` importé par `#build/…` | « Vue app aliases are not allowed in server runtime » — Nitro refuse les alias applicatifs | Fichier généré dans `i18n/locales/.generated/`, importé par chemin relatif. Ni alias, ni chemin absolu, ni `import.meta` |
| Le module ne voyait pas l'ajout d'un fichier de traduction | Le hook `builder:watch` de Nuxt ne surveille que `app/` et quelques fichiers de la racine, jamais `i18n/` | Observateur chokidar dédié, actif en développement seulement. **Vérifié** : ajout puis suppression d'un fichier régénèrent l'agrégation sans redémarrage |
| `npm install` échoue sur un conflit de pairs | `esbuild@0.25` hissé par `unplugin` contre `esbuild ^0.27 \|\| ^0.28` exigé par Vite 8 | `overrides: { "esbuild": "^0.28.0" }` dans `frontend/package.json` |
| `nuxt typecheck` : `node:fs`, `process` introuvables | `@types/node` absent | Ajouté en dépendance de développement |
| `lazy: true` refusé par la configuration i18n | L'option a disparu en @nuxtjs/i18n v10 : le chargement à la demande est le comportement par défaut dès qu'une locale a un `file` | Option retirée ; le découpage par locale est vérifié dans la sortie de construction (un fragment par langue) |

### Ce qui a été vérifié le 16/08 sur les composants et le guide de style, et comment

| Contrôle | Résultat |
|---|---|
| `npm run typecheck` (vue-tsc, strict) | 0 erreur. Quatre erreurs rencontrées et corrigées en chemin, toutes sur `UiTable` : la contrainte générique `T extends Record<string, unknown>` refuse une interface (pas de signature d'index implicite), d'où `T extends object` et un accès indexé explicite ; les créneaux `cell-*` sont désormais **typés** par `defineSlots`, ce qui donne `row` typé dans l'écran appelant et fait échouer la compilation si une colonne de vue est renommée |
| `npm run build` puis rendu serveur | Construction et rendu sans erreur ; `/style-guide` répond **200**, 327 ko |
| **Aucune clé brute affichée**, en français comme en anglais | Vérifié par balayage du HTML rendu sur les deux locales : zéro occurrence de `style-guide.*`, `session-card.*`, `form.*`, `data.*`, `incident-banner.*`, `live-badge.*`, `status-timeline.*` |
| Le format de date exigé par le prompt | « **14:30 — 16:00 (heure de Belém, UTC−3)** » produit tel quel, et « (Belém time, UTC−3) » en anglais. Le signe est le **moins typographique** (U+2212), pas un trait d'union — celui-ci sert déjà à séparer les heures |
| **Règle « un seul direct »** | **3 occurrences de « En direct » dans la page : une seule carte sur les six**, plus les deux pastilles explicitement forcées de la section « motifs ». La version dense écarte volontairement la séance en direct pour ne pas brouiller la démonstration |
| Densité réelle du tableau | 15 `<tr>` et 97 `<td>` rendus : 12 lignes × 8 colonnes pour la liste des propositions, plus l'en-tête, plus le tableau vide de démonstration. La pagination annonce bien 40 dossiers au total |
| Six états de carte de séance | Les cinq `temporal_state` de la vue tous rendus (`upcoming`, `ongoing`, `past`, `postponed`, `cancelled`), le sixième — « en direct » — par le registre partagé |
| Quatre gravités d'incident | Les quatre bandeaux rendus, libellés « Information », « Avertissement », « Incident », « Incident majeur » |
| Thème sombre posé dès le rendu serveur | `epavillon_theme=dark` → `<html data-theme="dark">`. Le HTML rendu est **identique** au thème clair : le thème est entièrement porté par les jetons CSS, aucun composant ne le teste |
| Sept sections ancrées | `jetons`, `composants-base`, `formulaires`, `donnees`, `navigation`, `composants-metier`, `motifs` — toutes présentes comme `<section id>` |
| Accessibilité, contrôles mécaniques | 33 contrôles de formulaire rendus, 3 `aria-invalid="true"`, 9 `role="alert"`, 4 `aria-current`, 3 `role="meter"`, 2 `<dialog>`, 68 états désactivés |
| Aucun fichier de code applicatif > 1000 lignes | Le plus long des fichiers créés est `TokensSection.vue` (311 lignes) ; le plus long du frontend reste `app/mocks/proposals/accepted.ts` (659) |
| `make check-front` | Vert |

### Ce qui a été vérifié le 16/08 sur les données simulées, et comment

| Contrôle | Résultat |
|---|---|
| `npm run typecheck` sur les 22 fichiers de mocks | 0 erreur. **La couverture a été prouvée, pas supposée** : une affectation fautive introduite dans `criteria.ts` fait bien échouer le contrôle (code 2, `TS2322`), puis a été retirée |
| Exécution réelle des modules (pas seulement compilation) | Les 22 fichiers chargés et parcourus : aucune des fabriques ne lève, le contrôle de cohérence des notes passe |
| Comptes attendus | 13 organisations, 25 personnes, 25 adhésions, 12 jours, 3 salles, 2 journées spéciales, 6 critères, **40 propositions**, 12 lignes de co-organisation, 47 intervenants de dossier, 32 revues, 192 notes par critère, **30 sessions** (20 publiées), **60 inscriptions** |
| Les huit statuts de proposition | Tous représentés : 16 retenues, 6 déposées, 5 en évaluation, 5 brouillons, 3 en correction, 3 écartées, 1 retirée, 1 annulée. 40 numéros de dossier distincts |
| Intégrité référentielle | Neuf familles de liens vérifiées une à une (proposition → organisation, déposant, contact, pays ; session → proposition, organisation, salle ; inscription → session, personne ; intervenants, affectations, thématiques) : **aucun lien mort** |
| Contraintes de la base rejouées en TypeScript | `ck_proposals_submitted_at`, `ck_sessions_cancelled_reason`, `ck_sessions_period`, `ck_registrations_waitlist`, `ck_duplicate_candidates_ordered` et l'unicité `(session, personne)` hors annulation : toutes respectées |
| Cohérence des notes | Contrôle intégré à `reviews.ts` : pour les 13 dossiers couverts, la moyenne des revues soumises redonne exactement `weighted_score`, `average_score`, `review_count` et `is_knocked_out` |
| `detect_conflicts()` sur les mocks | **1 conflit de salle** (mangroves ↔ pastoralisme, Baobab, 14/11 14 h), **1 conflit de diffusion** (article 6 ↔ Fonds vert, 12/11 14 h), 1 conflit d'organisation (ROAC programmée deux fois), 3 conflits « stand unique » — dont un discutable, voir l'écart n° 10 |
| Les deux vues reconstituées | `v_public_schedule` : 20 lignes, états `upcoming`, `postponed` et `cancelled` présents. `v_proposal_dashboard` : 40 lignes, rangs 1 à 40, `reviews_missing` et `open_change_requests` cohérents avec les revues et les fils de correction |
| Aucun fichier de mocks > 1000 lignes | Le plus long est `proposals/accepted.ts` (659 lignes) |
| `make check-front` | Vert |

### Ce qui a été vérifié le 16/08 sur le frontend, et comment

| Contrôle | Résultat |
|---|---|
| `npm run typecheck` (vue-tsc, strict) | 0 erreur |
| `npm run build` puis `node .output/server/index.mjs` | Construction et rendu serveur sans erreur |
| Traductions résolues en production | `/` → « ePavillon », `/en` → « Home », « Programme ». Plus aucune clé brute |
| Règle de nommage des clés | Testée hors Nuxt : `pages/proposal.form.step-speakers.json` → `proposal.form['step-speakers'].title`, et deux fichiers alimentant `proposal.form` fusionnent sans s'écraser |
| Thème posé dès le rendu serveur | Sans cookie : `<html lang="fr-FR">` ; `epavillon_theme=dark` → `data-theme="dark"` ; `=light` → `data-theme="light"` |
| Feuille de style produite | `--color-surface` défini deux fois (clair puis sombre), quatre règles `[data-theme=dark]`, cinq `[data-theme=light]`, variante `dark:` conforme aux deux gardes |
| Balises de langue | `hreflang` x-default/fr/fr-FR/en/en-GB, `canonical`, `og:locale` + alternates |
| Régénération à chaud des traductions | Ajout d'un fichier → 4 imports ; suppression → 3 |
| `make check-front` | Vert |
| Aucun fichier de code applicatif > 1000 lignes | Le plus long est `design-tokens.css` (460 lignes) |

Le journal du serveur avertit `No match found for location with path "/evenements"` et consorts : la navigation des layouts pointe vers les écrans du jalon, qui n'existent pas encore. Ces avertissements disparaîtront au fur et à mesure des prompts A1 à A5.

### Écarts entre `ENVIRONNEMENT_LOCAL.md` et ce qui a réellement démarré

Relevés le 16/08 en montant l'environnement. Tous corrigés dans le document et dans les fichiers.

| Ce que décrivait le document | Ce qui s'est passé | Correction retenue |
|---|---|---|
| `check-db: … up -d && sleep 12` | Le healthcheck `pg_isready` passe au vert **en 2 s**, alors que les 18 fichiers SQL sont encore en cours d'exécution : pendant l'initialisation, le serveur temporaire écoute déjà la socket locale. Un `sleep` fixe est un pari sur la vitesse de la machine | Cible `wait-db` : conteneur sain **et** `legacy.id_map` présente — le dernier objet du dernier fichier chargé |
| Rien sur les journaux d'initialisation | Une erreur de chargement laisse une base incomplète sans arrêter le conteneur — le piège annoncé. Aucune assertion ne le couvrait | Cible `assert-init-logs`, intégrée à `check-db` |
| — | Le healthcheck déclenché pendant l'arrêt du serveur temporaire laisse un `FATAL: the database system is shutting down` **bénin** qui faisait échouer à tort l'assertion précédente | `shutting down` et `starting up` explicitement écartés du filtre |
| Ports fixes `5432`, `3900`, `3903`… | Occupés sur la machine de développement par deux autres projets (`uafricas_postgres`, `kaya-objets`) | Chaque port publié devient `${VAR:-défaut}` ; la liste est en fin de `.env.example`, les valeurs par défaut sont inchangées |
| — | `docker compose` lit le `.env` du dossier du fichier compose (`ops/`), pas celui de la racine : les ports auraient été ignorés en silence | Le `Makefile` passe `--env-file .env` quand ce fichier existe |
| « [Garage] demande une initialisation manuelle », procédure donnée en quatre commandes | La procédure **était déjà écrite** dans le document, contrairement à ce qu'annonçait la consigne de session. Ce qui manquait : son automatisation et deux pièges | Cible `make garage-init` ; le document renvoie vers elle |
| `garage layout apply --version 1` | Rejouer la commande avec un numéro erroné donne `Invalid new layout version` — et l'identifiant du nœud **change à chaque `down -v`** | La cible lit « Current cluster layout version » et ajoute 1 ; elle relève le nœud elle-même |
| `check-front: cd frontend && …` | `frontend/` et `backend/` n'existent pas encore : `make check` échouait sur une absence de dossier, pas sur un défaut | Les deux cibles annoncent l'absence et rendent la main sans échouer |
| `.env.example` : `S3_BUCKET=epavillon`, `SMTP_HOST`/`SMTP_PORT` | La consigne de session demandait `S3_BUCKET=epavillon-dev` et un `SMTP_URL` unique | Le document a été suivi : `epavillon` est le nom que crée `garage bucket create`, et deux façons de configurer le SMTP auraient été une ambiguïté de plus. `epavillon-dev` reste le nom de la **clé** d'accès |

---

## Décisions prises en cours de route

Ce qui n'était pas dans le cadrage initial et qu'il a fallu trancher. **Quand un document semble contredire une décision, c'est ce tableau qui dit laquelle est la plus récente.**

| Date | Décision | Raison |
|------|----------|--------|
| 08-16 | **La règle « un seul direct à la fois » est tenue par CONSTRUCTION, pas par discipline** : `useLiveSession()` ne retient qu'un identifiant de séance, et `UiLiveBadge` ne rend rien pour les autres. Une carte ne décide jamais seule d'afficher son repère | Si chaque carte lisait `status === 'live'` pour elle-même, deux cartes clignoteraient dès qu'une séance resterait marquée « en direct » après son heure de fin — cas parfaitement banal, personne ne ferme une séance à la seconde près. La règle métier n° 4 devient alors une intention, pas une garantie |
| 08-16 | **Une couleur venue de la BASE est rendue en POINT, jamais en fond de texte** — thématiques (`taxonomy_terms.color_hex`), journées spéciales (`programme_tracks.color_hex`) | Ces couleurs sont saisies au back-office : rien ne garantit leur contraste, ni en thème clair, ni en thème sombre. Un administrateur qui choisit un jaune pâle rendrait le libellé illisible, et l'interface n'aurait aucun moyen de l'en empêcher. Le point porte la teinte, le texte garde les jetons du thème : les deux règles — « aucune couleur en dur » et « la couleur vient de la base » — tiennent alors ensemble |
| 08-16 | **Les rapports de contraste du guide de style sont CALCULÉS à l'affichage** (`utils/contrast.ts`), à partir des variables CSS réellement appliquées — jamais recopiés de `design-tokens.css` | Deux vérités divergent toujours, et celle qu'on recopie diverge la première. Bénéfice imprévu : basculer le thème recalcule tout, et l'on voit immédiatement si un rôle passe sous le seuil en sombre — ce qu'un tableau figé ne montrerait jamais. Le calcul demande le DOM : au rendu serveur, la palette affiche ses squelettes, ce qui est une démonstration de plus |
| 08-16 | **Modale et tiroir s'appuient sur `<dialog>` natif**, listes et dates sur les contrôles natifs | Trois choses gratuites que personne ne réimplémente correctement : la couche supérieure du navigateur (aucun conflit de `z-index` avec un en-tête collant), le piégeage du focus, et la fermeture par Échap. Idem pour `<select>` et `<input type="date">` : sélecteur du système sur mobile, clavier gratuit, jamais coupés par le débordement d'un conteneur. Une liste reconstruite ne se justifie que pour chercher dans des centaines d'entrées — c'est `UiSearchInput`, pas `UiSelect` |
| 08-16 | **`readonly` et `disabled` sont traités comme deux choses différentes**, dans tous les champs | Un champ désactivé sort du parcours de tabulation et sa valeur n'est pas soumise ; un champ en lecture seule reste focalisable, copiable et soumis. Les confondre, c'est empêcher quelqu'un de copier son numéro de dossier pour l'envoyer par courriel. Deux conséquences techniques : `<select>` ignore `readonly` (rendu par un champ désactivé DOUBLÉ d'un champ caché qui porte la valeur), et `UiCheckbox` annule le basculement sans se retirer du parcours |
| 08-16 | **Un état vide distingue « rien n'existe encore » de « le filtre ne ramène rien »** (`UiEmptyState`, propriété `filtered`) | Le premier propose de créer, le second de rétablir les filtres. Les confondre produit le classique « Aucun résultat — Créer une organisation » affiché à quelqu'un qui a mal orthographié un sigle : c'est ainsi qu'on fabrique des doublons, le défaut n° 1 de la v1 |
| 08-16 | **`UiErrorState` n'affiche son bouton de reprise que si un écouteur `retry` est fourni** | Proposer de réessayer une action qui échouera toujours est pire que de ne rien proposer. L'écran porte aussi un identifiant de requête sélectionnable (`app.request_id`) : c'est ce qui transforme un « ça ne marche pas » en incident retrouvable dans les journaux |
| 08-16 | **Le guide de style passe par `useApi()` comme n'importe quel écran** — aucun mock importé, pas même là | La règle ne souffre pas d'exception, et le bénéfice est réel : les composants sont éprouvés sur ce qu'ils rencontreront — titres d'activité longs, noms d'organisation à rallonge, jauges dépassées. La page traite d'ailleurs ses propres quatre états, ce qui la rend cohérente avec ce qu'elle prescrit |
| 08-16 | **Une seule altération des données dans le guide : l'état temporel des six cartes de séance** | Les données simulées se tiennent toutes en novembre 2027 : `temporal_state` y vaut « à venir » partout, ce qui est exact mais ne démontre rien. Les six cartes reprennent six VRAIES séances et n'en forcent que cet état ; titre, organisation, salle, thématiques, jauge et fuseau restent ceux de la base. L'altération est annoncée en tête du composant et dans la note de la section |
| 08-16 | **Les deux layouts consomment désormais les composants d'interface** (`UiNavBar`, `UiSideNav`, `UiBreadcrumb`, `UiThemeToggle`, `UiLocaleSwitch`) | Le layout du back-office portait la note : « les trois pictogrammes sont dupliqués depuis le layout public, le prompt A0.4 les factorisera ». Créer les composants sans les brancher aurait laissé la duplication en place ET ajouté du code mort |
| 08-16 | **Un fichier `types/live.ts` limité aux incidents**, alors que le module Direct est hors jalon | Le bandeau d'incident est un motif transverse : programmation publique, fiche d'activité, back-office (A13). Il est consommé bien avant qu'une visioconférence soit branchée. Les tables `meetings`, `streams` et `provider_webhook_events` ne sont PAS couvertes ; elles viendront avec leurs écrans |
| 08-16 | **Les données simulées ne recopient jamais une valeur qu'un calcul peut produire.** Une revue s'écrit comme six notes sur cinq ; la note pondérée, la note sur 20 et le drapeau d'élimination en découlent, comme le fait `refresh_proposal_score()`. Les deux vues du modèle et `detect_conflicts()` sont **dérivées** des mocks, jamais rédigées à la main | Une valeur recopiée à côté de sa source diverge dès la première modification, et l'écran affiche alors deux notes différentes pour le même dossier. Un contrôle exécuté au chargement de `reviews.ts` compare les agrégats aux revues et échoue pendant le développement plutôt qu'à l'écran |
| 08-16 | **Les identifiants des mocks sont des UUID v7 valides, construits de façon déterministe** : famille d'entité dans le quatrième groupe, numéro d'ordre dans le dernier (`…-7040-8000-000000000017` = proposition n° 17). Ils sont exposés sous des clés parlantes (`ORG.roac`, `PROPOSAL.adaptationCotiere`) | Deux exigences à tenir ensemble : la forme doit rester substituable par ce que produit `platform.uuid_v7()`, et un identifiant croisé dans une console doit se retrouver par simple recherche. Un tirage au sort interdisait la seconde, une chaîne fantaisiste la première |
| 08-16 | **Les inscriptions simulées ne connaissent que trois statuts** — inscrit, en liste d'attente, annulé | `attended` et `no_show` supposent que la séance ait eu lieu ; l'édition simulée se tient en novembre 2027. Les inventer aurait produit des taux de participation qui ne veulent rien dire, et que quelqu'un aurait fini par afficher |
| 08-16 | **`useApi()` déclare, pour chaque appel, sa route d'API ET sa lecture simulée au même endroit** ; les mocks sont chargés par import dynamique | Le raccordement (B7) devient la suppression d'une branche, pas une réécriture. L'import dynamique évite d'embarquer les données simulées dans le paquet une fois l'API configurée |
| 08-16 | **Une lecture hors périmètre d'administration lève une erreur, elle ne renvoie pas une liste vide** (`ForbiddenError` dans `useApi()`) | Une liste vide se lit comme « rien à afficher » et masque un défaut d'habilitation. C'est la même confusion que celle corrigée dans `administered_events()` : « aucun droit » et « aucune donnée » doivent rester distincts. Le contrôle sera doublé côté API — celui du front sert l'écran « accès refusé », pas la sécurité |
| 08-16 | **Un écart entre le modèle et l'interface se classe avant de se traiter** : défaut du modèle → corrigé dans le SQL ; arbitrage produit → question écrite au commanditaire ; obligation d'API → exigence inscrite dans le prompt B concerné | Les neuf écarts d'A0.2 n'étaient pas de même nature. Les traiter d'un même geste aurait produit cinq décisions produit inventées par le développeur, et laissé quatre obligations d'API dans un tableau que personne ne relit au moment de coder l'API |
| 08-16 | **Un même nom de champ ne désigne jamais deux types**, y compris entre une table et une vue. Quand une vue expose à la fois la donnée brute et sa version calculée, c'est la donnée brute qui garde le nom d'origine (`title`), la version calculée qui prend un suffixe (`title_text`) | Une conversion silencieuse ne se voit pas : `resolveI18nText()` appliqué à un `text` rend une chaîne vide sans erreur, et le bogue apparaît sur un écran, pas à la compilation |
| 08-16 | **Une fonction qui décrit un périmètre d'accès ne renvoie jamais `NULL`** : « aucun droit » s'écrit avec des valeurs pleines (`false`, `'{}'`) | Trois états doivent rester distincts — tout, une partie, rien — et `NULL` les confond deux à deux. En SQL, il fait pire : `x = ANY(NULL)` vaut `NULL`, donc un filtre d'habilitation ne renvoie rien **sans lever d'erreur**, ce qui se lit comme une liste vide légitime |
| 08-16 | **Les secrets d'authentification ne figurent dans aucun type du frontend** — `accounts.password_hash`, `mfa_secret_encrypted`, `mfa_recovery_codes`, `sessions.refresh_token_hash`, `one_time_tokens.token_hash` sont omis, l'omission est écrite en tête de `identity.ts` | Le prompt demandait une transcription littérale ; un type de frontend est une invitation à demander le champ à l'API. Ces colonnes ne franchissent jamais la frontière, les déclarer n'aurait servi qu'à laisser croire le contraire. Les trois colonnes `search_vector` (`tsvector`) sont omises pour la même raison, sans enjeu de sécurité : aucune représentation utile côté client |
| 08-16 | **Un fichier `media.ts`, hors de l'arborescence prescrite par le prompt A0.2** | Le formulaire de soumission téléverse des documents (`proposal_documents.asset_id` → `media.assets`) et les fiches d'organisation portent un logo : le jalon consomme le module Média, quoi qu'en dise la liste de fichiers du prompt. Le nommer d'après son schéma reste la règle générale |
| 08-16 | **Deux colonnes générées sont typées non nulles alors que le SQL ne l'impose pas** — `people.display_name` et `sessions.time_range` | Leurs expressions ne dépendent que de colonnes `NOT NULL` : `btrim(first_name \|\| ' ' \|\| last_name)` et `tstzrange(starts_at, ends_at)`. Les typer nullables aurait imposé un `?? ''` à chaque affichage d'un nom de personne, pour un cas qui ne se produit jamais. Les trois autres colonnes générées (`*_normalized`) restent nullables : `normalize_label()` renvoie `NULL` sur une entrée vide |
| 08-16 | **`event.events` devient `EventEdition`, `identity.sessions` devient `AuthSession`** | `Event` est un type global du DOM : l'ombrer rendrait incompréhensible toute erreur dans un composant qui manipule aussi des événements du navigateur. `Session` est réservé à `programme.sessions`, l'entité que les écrans manipulent constamment. Ce sont les deux seuls noms qui s'écartent de la table d'origine, chacun le dit dans son commentaire |
| 08-16 | **Les alias d'identifiants sont documentaires, pas marqués** — `type ProposalId = Uuid` | Une marque (`Uuid & { __brand: 'proposal' }`) donnerait une vraie sécurité, au prix d'une conversion sur chaque donnée simulée et chaque réponse d'API. Le rapport n'est pas favorable à ce stade ; l'alias sert la lisibilité des signatures |
| 08-16 | **Les colonnes écrites par trigger sont marquées `readonly`** — `time_range`, `enforce_room_exclusivity`, `is_exclusive`, les `*_normalized`, `display_name` | Un formulaire qui les envoie voit ses valeurs écrasées sans avertissement. Le compilateur est le seul endroit où cette règle est vérifiable sans relire le SQL |
| 08-16 | **`identity.ts` couvre tout son schéma, profils négociateurs compris**, alors que l'espace Négociations est hors jalon | Deux tables courtes. Les omettre aurait laissé un trou inexplicable au milieu d'un fichier, et la prochaine session aurait perdu du temps à vérifier si c'était un oubli |
| 08-16 | **Les échelles de nuances sont calculées, pas choisies à l'œil** — conversion de chaque couleur de charte en OKLCH, interpolation monotone de la luminosité, réduction jusqu'au gamut sRGB, puis calcul du ratio WCAG contre le fond clair et contre le fond sombre. Chaque nuance porte son verdict en commentaire | Une palette estimée se paie en allers-retours d'accessibilité sur chaque écran. Le calcul a d'emblée montré ce qu'on aurait découvert trop tard : le cyan de charte plafonne à 2,91:1 sur blanc, et **aucune nuance de jaune en deçà de 600 ne peut porter du texte sur fond clair** |
| 08-16 | **La couleur de charte reste dans son échelle, à son palier naturel** : cyan et rouge à 500, jaune à 400, violet à 700, bleu riche à 900, les deux gris à 200 et 700 | Le violet et le bleu riche sont des couleurs sombres, le jaune une couleur claire : les aligner tous sur 500 aurait produit des échelles fausses et rendu les couleurs officielles introuvables |
| 08-16 | **Le fond du thème sombre est le noir de charte `#231F20`**, pas un noir pur ni un gris bleuté | C'est une valeur du document officiel ; toutes les annotations de contraste sont calculées contre elle |
| 08-16 | **Les jetons de rôle sont déclarés dans `@theme` de Tailwind v4**, les surcharges de thème dans des blocs CSS ordinaires plus spécifiques | Un seul nom par rôle, un seul fichier, et les utilitaires (`bg-surface`, `text-text-muted`) suivent le thème sans configuration. Le détour par `@theme inline` aurait créé une définition circulaire — les noms imposés par le projet sont ceux de l'espace de noms de Tailwind |
| 08-16 | **Le thème est mémorisé dans un cookie**, pas dans `localStorage`, et l'attribut `data-theme` est absent en mode « système » | Le cookie est le seul moyen que le serveur pose déjà le bon thème : sinon la page s'affiche en clair une fraction de seconde avant de basculer. L'absence d'attribut laisse `prefers-color-scheme` décider |
| 08-16 | **L'agrégation des traductions passe par un module Nuxt local**, pas par `import.meta.glob` | `import.meta.glob` ne survit pas au rendu serveur, et l'échec est silencieux (voir les pièges ci-dessus) |
| 08-16 | **Les utilitaires de date ne connaissent pas i18n** : ils produisent des morceaux (« 14:30 », « Belém »), et `useDateTime()` les assemble avec les gabarits de `_common.json` | C'est ce qui permet de tenir les deux règles à la fois : aucune chaîne en dur, et toute date affichée porte son fuseau |
| 08-16 | **Les scripts npm lisent le `.env` de la racine** (`--dotenv ../.env`) | Une seule source de configuration pour l'API, le front et les services. `.env.example` gagne `NUXT_PUBLIC_SITE_URL`, exigée par i18n pour produire des `hreflang` valides |
| 08-16 | **Les logos officiels sont copiés dans `frontend/public/logos/`** en version grise et blanche | Les fichiers de `docs/` ne sont pas servis par le serveur web. Deux variantes plutôt qu'un filtre CSS : le logo institutionnel ne se recolore pas |
| 08-16 | **Les chevauchements de créneaux sont signalés, jamais bloqués.** Seule la publication du programme est conditionnée | Une contrainte qui refuse l'écriture transforme l'outil d'arbitrage en mur : un planificateur travaille par déplacements successifs et passe par des états incohérents (ADR-13) |
| 08-16 | **Un seul appel à propositions par édition**, zéro s'il n'y a pas de pavillon | Les journées thématiques sont composées *après* sélection, à partir du vivier commun — elles n'ouvrent pas leur propre fenêtre |
| 08-16 | **La co-organisation est de premier ordre** : porteur principal + co-organisateurs, partenaires, soutiens | Sans elle, les co-organisateurs restaient dans le texte de présentation, invisibles des filtres et des statistiques |
| 08-16 | **Les journées spéciales ne sont pas des jours de calendrier** mais des fils composés à la main (`programme_tracks`) | Une journée spéciale peut n'occuper qu'une matinée, un jour peut en porter deux, un fil peut déborder sur deux jours |
| 08-16 | **Le rôle d'administrateur est attribuable sur un seul événement** | Évite de redévelopper une page d'administration séparée comme en v1 (ADR-14) |
| 08-16 | **Le module Formations entre au produit minimum viable** | Demande explicite du commanditaire : « le module formation est important, il doit être construit au MVP » |
| 08-16 | **Le workspace Cargo vit dans `backend/`**, pas à la racine | Symétrie avec `frontend/` ; à la racine, le dépôt se lisait comme « un projet Rust contenant un frontend » |
| 08-16 | **Traductions, types et mocks découpés par écran**, pas par domaine | Un fichier par domaine reste trop volumineux : le seul formulaire de soumission compte sept étapes |
| 08-16 | **Pas d'intégration continue, mais trois vérifications locales** dans un `Makefile` | Développeur seul et pressé : une chaîne complète est du temps pris sur la livraison |
| 08-16 | **Les quiz de formation ne réutilisent pas le module `tool`** | `tool` est conçu pour être extrait ; un quiz de formation est indissociable de la progression et de l'attestation |
| 08-16 | **Les ports publiés sont paramétrables par `.env`**, valeurs par défaut inchangées | Trois d'entre eux étaient déjà pris par d'autres projets de la machine ; arrêter les conteneurs d'un autre projet pour faire tourner celui-ci n'est pas une option acceptable |
| 08-16 | **Ce sont les fichiers qui font foi, plus le document** — `ENVIRONNEMENT_LOCAL.md` ne recopie plus intégralement le `Makefile` | Deux copies d'un même contenu divergent toujours ; le document garde ce qu'un fichier ne dit pas : les intentions et les pièges |
| 08-16 | **La vérification de Garage passe par une écriture réelle**, pas par `bucket info` | Un nœud sans layout répond à l'API S3 et affiche un bucket correct tout en refusant chaque dépôt : seul un `PUT` distingue les deux situations |

---

## Points bloqués ou en attente

### Trois questions au commanditaire (écarts n°2, n°8, n°9 — posées le 16/08)

Ces trois écarts engagent le produit ou l'ergonomie, pas la justesse du modèle : **ils ne se tranchent pas depuis le code**. Chaque question se répond par le choix d'une option ; aucune ne bloque le travail en cours, l'interface tiendra provisoirement l'option recommandée.

| N° | La question | Les options, et ce que chacune coûte | Recommandation |
|---|---|---|---|
| **2** | Sur les pages en anglais, faut-il afficher le nom **traduit** d'une organisation quand elle en a fourni un — « UNDP » plutôt que « PNUD » — ou toujours son nom légal officiel ? | **A. Afficher la traduction quand elle existe**, nom légal à défaut. Les traductions sont déjà collectées par la plateforme, avec leur langue ; il faut en revanche que quelqu'un les **valide** au back-office, car celles qui viennent d'un import ne sont pas vérifiées et ne doivent pas s'afficher telles quelles. Une jointure de plus sur la programmation publique et la fiche d'organisation. **B. Toujours le nom légal.** Rien à développer, mais un visiteur anglophone lit « Institut de la Francophonie pour le développement durable », et les organisations internationales qui ont un nom anglais officiel n'apparaissent jamais sous ce nom | **A.** La donnée est déjà là ; ne pas l'afficher revient à la collecter pour rien. Le travail de validation est faible : quelques centaines d'organisations actives |
| **8** | Sur son espace, une organisation qui suit son dossier doit-elle voir **la note et le classement** attribués par le comité, ou seulement l'état d'avancement, les demandes de correction et la décision finale ? | **A. État, corrections et décision seulement.** C'est ce que montrait la v1. Deux requêtes prêtes à l'emploi sont à ajouter au modèle — « mes dossiers » et la fiche publique d'une activité — ce qui garantit que la règle de visibilité est écrite à un seul endroit plutôt que répétée dans chaque écran. **B. Y ajouter la note et le rang.** Transparence maximale, mais le classement devient visible d'organisations concurrentes, chaque refus s'argumente chiffre en main, et le comité perd la liberté de noter franchement | **A.** L'évaluation reste interne ; ce qui est dû au soumissionnaire, c'est un motif clair, pas une note |
| **9** | Faut-il **obliger** l'équipe à saisir un sigle (« COP31 », « PACO ») à la création de chaque édition ? Ce sigle préfixe le numéro de dossier communiqué aux organisations | **A. Obligatoire dans le formulaire d'administration, pour les éditions tenant un pavillon** ; la base continue d'accepter une édition sans sigle. Un champ de plus à remplir pour les COP, rien à changer ailleurs. **B. Obligatoire partout, imposé par la base.** Casse les webinaires du cycle PACO, qui n'ont pas de sigle, et la reprise des données de la v1 : il faudrait inventer un sigle pour chaque édition passée. **C. Ne rien changer.** Les dossiers d'une édition sans sigle porteront un numéro tronqué — vérifié en base : « COP31-TE-00001 » — à épeler lettre à lettre au téléphone | **A.** Le besoin est réel, mais c'est une règle de saisie, pas une règle du modèle : l'y inscrire interdirait un cas d'usage existant |

### Autres points

| Sujet | Nature | Depuis |
|-------|--------|--------|
| **`v_public_schedule` n'expose ni le pays de l'organisation, ni les libellés et couleurs des thématiques** — la carte de séance a besoin des deux, et la vue est censée répondre à l'écran en une requête (écarts n° 14 et 15). Contourné par des propriétés séparées dans `UiSessionCard` | Correction du SQL à décider **avant A3**. Les deux se traitent dans la même passe sur la vue | 2026-08-16 |
| **Portée du conflit « stand unique »** — `detect_conflicts()` remonte en gravité bloquante toute paire de séances simultanées de l'édition, séance en ligne comprise, alors qu'une séance en ligne n'occupe pas le stand. Reproduit sur les données simulées (écart n° 10) | Correction du SQL à décider **avant A9** | 2026-08-16 |
| **Les fontes de la charte ne sont pas dans le dépôt** — `Helvetica` et `NeueMaverick` sont déclarées, mais aucun fichier de police n'est fourni : les replis système font foi tant qu'elles n'ont pas été obtenues | Fichiers à obtenir auprès du service communication de l'IFDD | 2026-08-16 |
| **Portée de la règle « un seul direct »** — le modèle garantit un direct par *canal* ; si deux événements ouvrent chacun le leur, deux directs simultanés redeviennent possibles. Faut-il un verrou global ? | Arbitrage à rendre | 2026-08-16 |
| **Édition, date et ville de référence** — les données simulées disent « COP31, Belém, novembre 2027 », or Belém accueillait la COP30 | Information à fixer | 2026-08-16 |
| **Évaluation en aveugle** — le commanditaire demande que les révisionnistes voient les notes des autres et la moyenne ; le modèle propose l'aveugle en option. Quelle valeur par défaut ? | Arbitrage à rendre | 2026-08-16 |
| **Reprise des données v1** — placée au jalon 2, alors que le critère de sortie du jalon 1 exige des « données réelles importées » | Incohérence de planification | 2026-08-16 |
| **Qui sert la COP31** — v1 ou v2, et comment les propositions collectées d'un côté rejoignent l'exécution de l'autre | Arbitrage à rendre | 2026-08-16 |
| **Accessibilité et bilinguisme** — imposés comme règles de code, mais absents des exigences produit et des non-objectifs. Écartés ou oubliés ? | Arbitrage à rendre | 2026-08-16 |
| **Synchronisation Google Agenda / Apple** — demandée par le commanditaire en « phase 2+ », absente de toute la documentation | À réintégrer au cadrage | 2026-08-16 |
| Sens exact de « QCD » pour les quiz de formation | À confirmer auprès de l'IFDD | 2026-08-16 |
| Statut OIF des pays | Liste officielle à obtenir, ne peut pas être devinée | 2026-08-16 |
| Reprise ou abandon de la messagerie directe | Arbitrage à rendre | 2026-08-16 |
