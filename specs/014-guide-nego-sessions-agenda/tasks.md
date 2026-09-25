# Tasks: Guide Négo — sessions de négociation : l'agenda et son import (étape 3a)

**Input**: [spec.md](spec.md) · [plan.md](plan.md) · [research.md](research.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

**Dossier** : `/Users/mac/Documents/projets/IFDD/epavillon_v2-dev2` — base `epavillon_dev2`, API 8096, site 3005.
**Règles** : CLAUDE.md et la constitution ; le protocole `/Users/mac/Documents/projets/IFDD/epavillon_v2/.orchestration/protocole.md`.
**Une phase = un commit**, message en français, terminé par `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`.
**Contrôles ciblés** entre les phases : `cargo test -p negotiation`, `npm run typecheck`, `npm run check:guide-nego`, `npm run test:guide-nego`. Jamais `make check` ni `down -v`.

Chemins relatifs au dossier ; `neg/` = `backend/crates/modules/negotiation/`, `fe/` = `frontend/`.

---

## Phase 1 — Le modèle (bloquant)

**But** : le SQL d'abord, la base migrée sans être détruite.

- [x] T001 Semer les vocabulaires `negotiation_meeting_type` (9 termes, `metadata` : `denominations`, `source_categories`, `requires_title_match`, `default_for`) et `negotiation_group` (13 termes, `metadata.denominations` : noms et sigles lus dans la source réelle), avec `label` fr/en, après leur inscription dans `reference.taxonomies` (`is_system`), dans `docs/database/020_reference.sql` — data-model § 2, research R4
- [x] T002 Étendre `negotiation.meetings` (onze colonnes, `ck_meetings_source_complete`, `end_at` nullable, contraintes retouchées `ck_meetings_period`, `ck_meetings_imported_end`, `ck_meetings_onsite_venue`, `ck_meetings_import_cancellation` (`source`, `postponed`, `removed`), index `ux_meetings_source` et `ix_meetings_event_day`) et exempter les lignes importées dans `negotiation.tg_meeting_status_event`, dans `docs/database/100_negotiations.sql` — data-model § 3
- [x] T003 Créer `agenda_items`, `meeting_changes`, `official_imports`, `import_runs`, `title_translations` (avec le `COMMENT ON TABLE` de l'écart XII), `group_subscriptions`, `agenda_entries`, leurs `xmod_fk_*`, gardes de vocabulaire, triggers d'audit et `updated_at`, et la fonction `negotiation.import_is_serving(event_id)`, dans `docs/database/100_negotiations.sql` — data-model § 4 et § 6, research R5
- [x] T004 Semer `ai.drafting_model` et la ligne `official_imports` de `cop31` éteinte (lecteur `archive`, `official_programme_url = https://unfccc.int/cop31/schedule`) par `INSERT … SELECT … FROM event.events WHERE slug = 'cop31' ON CONFLICT DO NOTHING`, dans `docs/database/900_seed.sql` — data-model § 5
- [x] T005 Écrire `specs/014-guide-nego-sessions-agenda/migration.sql`, rejouable (patron de `specs/010-guide-nego-accueil-profil/migration.sql`), qui porte T001 à T004 sur une base en service
- [x] T006 Charger `docs/database/` dans une base jetable (`createdb` sur le conteneur `epavillon-postgres`, nom `epavillon_dev2_jetable`), jouer `migration.sql` deux fois sur `epavillon_dev2`, comparer les deux `pg_dump --schema-only` triés, puis supprimer la base jetable
- [x] T007 Consigner le changement de modèle en une ligne dans `docs/progression/modele.md` (seule écriture permise hors `docs/AppNego/`, ADR-017)

**Commit** : « feat(guide-nego): étape 3a, phase 1 — le modèle des sessions de négociation ».

---

## Phase 2 — L'import (US2, P1)

**But** : le travail lit la source, n'écrit que les écarts, et l'affichage se coupe au seuil.
**Test indépendant** : SC-001, SC-002, SC-003 sur le jeu archivé, sans navigateur.

- [x] T008 [US2] Définir le trait `SourceOfficielle`, la forme pivot `SessionLue` et `EchecLecture` dans `neg/src/import/source.rs` ; créer `neg/src/import/mod.rs` qui déclare **tous** les sous-modules de l'import (source, ccnucc, archive, reel, denominations, comparaison, traduction), et le module `import` dans `neg/src/lib.rs` — research R2
- [x] T009 [P] [US2] Constituer le jeu archivé : réduire le JSON réel de la COP30 (`.sources-ccnucc/cop30-conference-calendar-12286.json`, à la racine du dossier, **hors Git** ; à défaut, `https://unfccc.int/unfccc-conference-calendar/12286` pris au navigateur) à **quelques dizaines de réunions** des 17 et 18 novembre 2025 (dont un « POSTPONED », une coordination d'un groupe reconnu et une non reconnue), **forme et clés inchangées** — le dépôt est public, dans `neg/src/import/archives/cop30/lecture-1.json` ; `lecture-2.json` avec exactement quatre écarts (une session avancée, une changée de salle, une disparue, une nouvelle) ; `injoignable` ; `illisible.json` (JSON malformé) ; un `LISEZMOI.md` d'une page : source, date de prise, conditions d'utilisation, **accord du secrétariat en cours**, ce qui est reconstitué ; vérifier par `git status` que rien de `.sources-ccnucc/` n'est suivi — research R2, R3
- [x] T010a [US2] Écrire l'analyse du format CCNUCC commune aux deux lecteurs (JSON du calendrier, `access` "0"/"1", préfixes « CANCELLED » et « POSTPONED », code et intitulé du point de l'ordre du jour extraits du titre dans ses deux ordres, heure naïve + `time_correction_minutes` puis fuseau de l'édition) dans `neg/src/import/ccnucc.rs`, avec tests unitaires sur des titres réels (« HoDs draftng group CMA 11 (a) -NCQG ») — research R2
- [x] T010 [US2] Écrire `LecteurArchive` (fichiers embarqués par `include_str!`, translation des jours par `archive_first_day` dans le fuseau de l'édition, `injoignable` → échec) dans `neg/src/import/archive.rs`, avec tests unitaires
- [x] T011 [US2] Écrire, après T010a, `LecteurReel` (reqwest, délai 15 s, analyse de `ccnucc.rs` ; une page anti-robot vide est une lecture manquée au message clair) dans `neg/src/import/reel.rs` ; ajouter `reqwest.workspace = true` à `neg/Cargo.toml`
- [x] T012 [P] [US2] Écrire la normalisation et la résolution par les `metadata` des vocabulaires dans `neg/src/import/denominations.rs`, avec tests unitaires : accents, casse, sigles ; catégorie non admise → écartée ; type par défaut de la catégorie ; `requires_title_match` ; coordination sans groupe résolu (« YOUNGO Morning Coordination Meeting ») → **retenue, groupe nul** — research R4
- [x] T013 [US2] Écrire la comparaison pure pivot × état en base → écarts (apparue, changée champ par champ, absente, reparue ; liste vide = lecture manquée) dans `neg/src/import/comparaison.rs`, avec tests unitaires — research R6, R7
- [x] T014 [US2] Écrire le dépôt de l'import (valeurs posées par l'import de data-model § 3 — `kind`, espace `climat`, `slug`, `format = 'onsite'`, `is_ifdd_organized = false`, `timezone`, `title` ; lecture de l'état, upsert des points d'ordre du jour, écriture des sessions, changements, `absent_reads`, `last_read_at` en une requête, compteurs — `change_count` = sessions touchées —, groupe re-résolu à chaque lecture sans écart ni changement, journal, purge du journal à 30 jours) dans `neg/src/repo/import.rs`
- [x] T015 [US2] Écrire le travail `negotiation.import_official_sessions` qui se replanifie (patron `neg/src/jobs/purge.rs`, clé `import:<edition>:<floor(epoch/interval)>`, réussit toujours côté file, ne se replanifie pas éteint ; un travail `manuel: true` ne se replanifie jamais — research R7) dans `neg/src/jobs/import.rs` ; l'inscrire dans `job_handlers()` de `neg/src/lib.rs`
- [x] T016 [US2] Réarmer la chaîne de chaque import allumé dans `armer_les_recurrents()` de `backend/crates/worker/src/main.rs`
- [x] T017 [P] [US2] Test sur base réelle : première lecture écrit toutes les sessions de `lecture-1` avec origine et lien ; seconde lecture identique → zéro écriture de session, zéro écart (SC-001), dans `neg/tests/import_nominal.rs`
- [x] T018 [P] [US2] Test : `lecture-2` → exactement quatre écarts, valeurs précédentes gardées ; disparue inchangée à la première absence, `cancelled`/`removed` à la seconde ; reparue → `scheduled` (SC-002, FR-015), dans `neg/tests/import_ecarts.rs`
- [x] T019 [P] [US2] Test : `injoignable` et `illisible.json` comptent comme lectures manquées ; `injoignable` jusqu'au seuil → `import_is_serving` faux, `failing_since` posé ; lecture réussie → vrai ; import éteint → faux ; dernière réussite trop ancienne (worker arrêté simulé) → faux (SC-003, research R5), dans `neg/tests/import_coupure.rs`
- [x] T020 [P] [US2] Test : l'import n'émet aucun événement d'outbox, même à l'annulation (FR-021), et écrit sous le contexte du travail, dans `neg/tests/import_sans_effet.rs`

**Commit** : « feat(guide-nego): étape 3a, phase 2 — l'import de la source officielle ».

---

## Phase 3 — La traduction des titres (US2, P1)

**But** : une traduction par titre, avec modèle et date ; sans clé, l'anglais seul. Aucun test n'appelle OpenRouter.

- [x] T021 [US2] Définir le trait `Traducteur` et le client OpenRouter (modèle lu dans `ai.drafting_model`, lots de 40, réponse JSON alignée, clé jamais journalisée ni renvoyée dans une erreur) dans `neg/src/import/traduction.rs` — research R11
- [x] T022 [US2] Lire `OPENROUTER_API_KEY` depuis l'environnement dans `NegotiationConfig` (`backend/crates/kernel/src/config.rs`), transmise à `job_handlers(db, &config, …)` ; absente → aucun travail de traduction posé ; `.env.example` porte déjà le nom
- [x] T023 [US2] Écrire le travail `negotiation.translate_session_titles` (titres sans traduction, un appel par lot, écrit `title_translations`, échec → fin sans écrire) dans `neg/src/jobs/traduction.rs` ; l'import le pose quand des titres manquent (`neg/src/jobs/import.rs`) ; l'inscrire dans `job_handlers()`
- [x] T024 [P] [US2] Test avec un traducteur fixe : un titre partagé par deux sessions → une traduction ; titre anglais changé → nouvelle traduction ; échec → rien d'écrit ; sans clé → rien de posé (SC-010), dans `neg/tests/traduction_titres.rs`

**Commit** : « feat(guide-nego): étape 3a, phase 3 — la traduction automatique des titres ».

---

## Phase 4 — L'API de l'application (US1, US4)

**But** : `contracts/api-sessions.md` servi, engendré, testé.

- [ ] T025 Ajouter les sept codes stables et leurs messages français au catalogue dans `backend/crates/kernel/src/error.rs`
- [ ] T026 [US1] Écrire les formes rendues `OfficialSessions`, `OfficialSession` dans `neg/src/domain/sessions.rs`, et le dépôt de lecture (sessions de l'édition, traduction, précédent, type, groupe, thématique par le point d'ordre du jour, fuseau de l'édition, `import_is_serving`) dans `neg/src/repo/sessions.rs`
- [ ] T027 [US1] Écrire `GET /negotiation/sessions?edition=` (publique, `ETag` et `304`, liste vide quand coupé) dans `neg/src/service/sessions.rs` et `neg/src/routes/sessions.rs`
- [ ] T028 [P] [US1] Écrire `GET`/`PUT /negotiation/me/groups` sur le patron exact des thématiques de 0c (`neg/src/{domain,repo,service,routes}/themes.rs`) dans `neg/src/{domain,repo,service,routes}/groups.rs`
- [ ] T029 [US4] Écrire `GET /negotiation/me/agenda`, `PUT` et `DELETE /negotiation/me/agenda/{session_id}` (idempotents, `404`, `409` sur annulée absente, rappel désarmé sur annulée ; `GET` rend `remind` effectif — faux pour une session annulée) dans `neg/src/{domain,repo,service,routes}/agenda.rs`
- [ ] T030 Monter les routes dans `neg/src/routes/mod.rs` et `neg/src/lib.rs`, les déclarer dans `neg/src/routes/openapi.rs`, lancer `make openapi` pour régénérer `fe/app/types/api.ts`
- [ ] T030a Écrire les formes `OfficialSessions`, `OfficialSession`, `MyGroups`, `MyAgenda` dans `fe/app/types/negotiation-sessions.ts`, ré-exportées par `fe/app/types/index.ts`, pour que `make check-api-contract` passe dès cette phase
- [ ] T031 [P] [US1] Test : import éteint → `state: cut`, `disabled`, liste vide (SC-004) ; coupé → liste vide même avec des lignes en base ; servi → formes du contrat, `previous` sur la déplacée, `theme` hérité du point, `304` sur `If-None-Match`, dans `neg/tests/sessions_public.rs`
- [ ] T032 [P] [US1] Tests des groupes : nominal, `412` périmé, code inconnu `422`, liste vide permise, audit, dans `neg/tests/groupes.rs`
- [ ] T033 [P] [US4] Tests de l'agenda : ajout, rejeu idempotent, retrait idempotent, `404`, `409`, rappel sur annulée, rappel effectif faux après annulation par l'import, sans compte `401`, audit, aucun refus pour chevauchement, dans `neg/tests/agenda.rs`

**Commit** : « feat(guide-nego): étape 3a, phase 4 — l'API des sessions, des groupes et de l'agenda ».

---

## Phase 5 — Le back-office (US2)

**But** : l'IFDD allume l'import, lit son état, rattache l'ordre du jour aux thématiques.

- [ ] T034 [US2] Écrire `GET`/`PUT /admin/negotiation/import`, `POST /admin/negotiation/import/read`, `GET /admin/negotiation/agenda-items`, `PUT /admin/negotiation/agenda-items/{id}` (garde `Requires<SpaceManage>` globale ; allumer pose la première lecture dans la transaction) dans `neg/src/{domain,repo,service,routes}/admin_import.rs`, montées et déclarées à l'OpenAPI ; `make openapi`
- [ ] T035 [P] [US2] Tests : garde globale (un administrateur d'une seule édition est refusé) dans un fichier **nouveau** `neg/tests/perimetre_import.rs` — `perimetre_url_forgee.rs` est à 881 lignes ; y ajouter seulement `admin_import.rs` à la liste des fichiers de routes relus ; dans `neg/tests/admin_import.rs` : allumer pose un travail, deux « Lire maintenant » de suite font deux lectures sans doubler la chaîne, éteindre coupe aussitôt, seuil pris à la lecture suivante, `400 NEGOTIATION_IMPORT_CONFIG_INVALID` sur lecteur réel sans adresse, rattachement d'un point
- [ ] T036 [US2] Ajouter les méthodes d'administration de l'import et de l'ordre du jour à `fe/app/composables/api/admin-negotiations.ts` et les formes d'administration à `fe/app/types/negotiation-sessions.ts` (créé en T030a)
- [ ] T037 [US2] Écrire l'écran « Import » de l'édition que sert l'application (interrupteur `UiSwitch`, lecteur, jeu archivé, premier jour, correction horaire, adresses, intervalle, seuil, état servi/coupé, dernière réussite, erreur, écarts, journal, « Lire maintenant ») dans `fe/app/pages/admin/negociations/import.vue` et `fe/app/components/admin/negotiation/ImportStatus.vue`, textes dans `fe/i18n/locales/{fr,en}/pages/admin.negociations.import.json`
- [ ] T038 [US2] Écrire l'écran « Ordre du jour » (liste des points, thématique à choisir, compte des points sans thématique) dans `fe/app/pages/admin/negociations/ordre-du-jour.vue` et `fe/app/components/admin/negotiation/AgendaItemRow.vue`, textes dans `fe/i18n/locales/{fr,en}/pages/admin.negociations.ordre-du-jour.json`
- [ ] T039 [US2] Ajouter les deux entrées au menu du back-office de Guide Négo, à côté de « Documents », dans `fe/app/layouts/admin.vue` (ou le fichier qu'il lit pour son menu)

**Commit** : « feat(guide-nego): étape 3a, phase 5 — le back-office de l'import ».

---

## Phase 6 — La plomberie client (bloquant pour 7 à 9)

- [ ] T041 Écrire le client `fe/app/composables/api/negotiation-sessions.ts` (sessions, groupes, agenda) et **la seule ligne de montage** dans `fe/app/composables/useApi.ts`
- [ ] T042 Ajouter `slug`, `timezone` et `city` à l'édition gardée dans `fe/app/utils/guide-nego/edition.ts` et à ce qui la sert
- [ ] T043 [P] Écrire les règles pures de `fe/app/utils/guide-nego/sessions.ts` (état affiché, jours de la bande dans le fuseau de la COP, jour choisi FR-003, tri FR-005, filtre FR-006 — une coordination sans groupe seulement sous « Toutes » —, état vide et prochain créneau FR-011) et leurs tests dans `fe/tests/guide-nego/sessions.test.ts`
- [ ] T044 [P] Écrire les règles pures de `fe/app/utils/guide-nego/agenda.ts` (chevauchements FR-033, prochaine session FR-036, rappel dû FR-034 — jamais sur une annulée, FR-035) et leurs tests dans `fe/tests/guide-nego/agenda.test.ts`
- [ ] T045 Écrire `useGnSessions` (clé `sessions:<edition>`, réponse coupée qui remplace la copie — FR-039), `useGnGroupes` (clé `mes-groupes`, file avec `If-Match`), `useGnAgenda` (clé `mon-agenda`, garde réécrite aussitôt, `PUT`/`DELETE` en file) dans `fe/app/composables/guide-nego/`

**Commit** : « feat(guide-nego): étape 3a, phase 6 — la plomberie des sessions côté application ».

---

## Phase 7 — US1 : la liste d'un jour (P1) 🎯 MVP

**But** : écran 07 et ses quatre variantes. **Test indépendant** : quickstart § 1, 2, 4, 5.

- [ ] T046 [P] [US1] Créer `GnBandeJours` (défilement horizontal, jour choisi, jour courant, jours passés) dans `fe/app/components/guide-nego/GnBandeJours.vue`, textes `fe/i18n/locales/{fr,en}/components/gn-bande-jours.json`
- [ ] T047 [P] [US1] Créer `GnEtatSession` (pictogramme + mot + couleur pour les cinq états, « Déplacée — était à … », « Annulée à … », et selon `cancelled.reason` « retirée du programme officiel » ou « reportée par la source ») dans `fe/app/components/guide-nego/GnEtatSession.vue`, textes `gn-etat-session.json`
- [ ] T048 [US1] Créer `GnLigneSession` (heures en 20 px, type, titre fr, « EN » + titre anglais + « Traduction automatique », salle, accès, thématique ou « Thématique non précisée » ou « Mon groupe », état, chevron) dans `fe/app/components/guide-nego/GnLigneSession.vue`, textes `gn-ligne-session.json`
- [ ] T049 [P] [US1] Créer `GnLectureImpossible` (« Lecture impossible », depuis quand, lien vers le programme officiel, « Réessayer ») dans `fe/app/components/guide-nego/GnLectureImpossible.vue`, textes `gn-lecture-impossible.json`
- [ ] T050 [US1] Remplacer `fe/app/pages/guide-nego/negociations.vue` par `fe/app/pages/guide-nego/negociations/index.vue` : en-tête, bande, filtre « Mes thématiques / Toutes » et compteur, étiquette « Source officielle — lu à », liste, lien de pied, états chargement, vide (FR-011), coupé, hors connexion avec bandeau de première ouverture, accès « Mon agenda » ; textes dans `fe/i18n/locales/{fr,en}/pages/guide-nego.negociations.json`
- [ ] T051 [US1] Ajouter le bloc « Mon groupe de négociation » à `fe/app/pages/guide-nego/thematiques.vue` (cases, vocabulaire `negotiation_group` lu comme les thématiques, file), textes dans `guide-nego.thematiques.json`
- [ ] T052 [US1] Ajouter les quatre composants à la planche (`fe/app/components/guide-nego/planche/`), dans une section « Sessions »
- [ ] T053 [US1] Vérifier au navigateur, version construite, 360 px, clair et sombre, l'écran contre `docs/AppNego/design/ecrans/07-sessions.html`, et quickstart § 1, 2, 4, 5

**Commit** : « feat(guide-nego): étape 3a, phase 7 — la liste des sessions de négociation ».

---

## Phase 8 — US3 : la fiche (P2)

- [ ] T054 [P] [US3] Créer `GnValeurChangee` (ancienne valeur barrée → nouvelle, ou valeur seule) dans `fe/app/components/guide-nego/GnValeurChangee.vue`, textes `gn-valeur-changee.json`, ajouté à la planche
- [ ] T055 [US3] Écrire `fe/app/pages/guide-nego/negociations/[id].vue` : titres, état (annulée : motif et heure du constat), « Source officielle · lu à », heure avec jour et fuseau, salle, type avec terme anglais touchable qui ouvre `/guide-nego/lexique?terme=<texte anglais>` (research R10 ; aucun code de lexique), ordre du jour ou « Hors ordre du jour officiel », accès, thématique, lien vers l'original (pas de section de documents : FR-029), « Ajouter à mon agenda » / « Dans mon agenda » (désactivé si annulée ; sans compte → connexion), interrupteur du rappel et sa phrase ; états chargement, jamais lue hors connexion, coupé, inconnue ; textes dans `fe/i18n/locales/{fr,en}/pages/guide-nego.negociations-fiche.json`
- [ ] T056 [US3] Vérifier au navigateur contre `08-detail-de-session.html` (variantes 1a à 1d, sans l'encart de signalement), quickstart § 6

**Commit** : « feat(guide-nego): étape 3a, phase 8 — la fiche d'une session ».

---

## Phase 9 — US4 : Mon agenda, le rappel, « Ma journée » (P2)

- [ ] T057 [US4] Écrire `fe/app/pages/guide-nego/negociations/agenda.vue` : bande des jours, section « Sessions de négociation » et compteur, lignes (titre fr, salle, thématique, état hors « Prévue », « Rappel 15 minutes avant »), chevauchements, annulées barrées ; états chargement, vide, sans compte, hors connexion ; textes dans `fe/i18n/locales/{fr,en}/pages/guide-nego.negociations-agenda.json`
- [ ] T058 [US4] Écrire `useGnRappel` et `GnBandeauRappel` (vérification chaque minute et au retour au premier plan, une fois par session, refermable) dans `fe/app/composables/guide-nego/useGnRappel.ts` et `fe/app/components/guide-nego/GnBandeauRappel.vue`, monté dans `fe/app/layouts/guide-nego.vue` ; textes `gn-rappel.json` ; ajouté à la planche
- [ ] T059 [US4] Remplir **le seul** bloc `prochaine-session` de « Ma journée » dans `fe/app/pages/guide-nego/index.vue` (agenda, sinon thématiques, sinon la ligne vide actuelle ; heure avec fuseau, titre fr et anglais, salle) ; `fe/tests/guide-nego/ma-journee.test.ts` reste vert
- [ ] T060 [US4] Vérifier au navigateur contre la partie « Mon agenda » de `09-signaler-et-mon-agenda.html` et le bloc de `02-socle.html`, quickstart § 7 et § 8

**Commit** : « feat(guide-nego): étape 3a, phase 9 — Mon agenda, le rappel et Ma journée ».

---

## Phase 10 — Recette et finitions

- [ ] T061 Dérouler `quickstart.md` en entier sur la version construite, à 360 px, clair et sombre, chronométrer SC-005 (salle et heure de la prochaine session en moins de dix secondes), et corriger ce qui est trouvé
- [ ] T062 [P] Inscrire l'écart 45 (titres traduits sans relecture, décision du 25/09) et la ligne sous l'interrupteur du rappel dans `docs/AppNego/05-design.md`
- [ ] T063 [P] Ajouter au § 15 de `docs/DEPLOIEMENT.md` les essais sur appareil réel : liste en mode avion (Android, iPhone), bandeau du rappel application ouverte, `migration.sql` de 3a dans l'ordre des migrations
- [ ] T064 Relire les traductions `en` des fichiers i18n nouveaux, clé pour clé avec `fr`
- [ ] T065 Ajouter une ligne de trois phrases au plus par phase dans `docs/AppNego/progress.md` et mettre à jour la ligne d'état « 3a »

**Commit** : « feat(guide-nego): étape 3a, phase 10 — recette et finitions ».

---

## Dependencies

- Phase 1 bloque tout. Phase 2 bloque 3, 4, 5. Phase 4 bloque 6. Phase 6 bloque 7, 8, 9. 7 → 8 → 9.
- Phase 5 (back-office) ne dépend que de 2 et 4 ; elle peut avancer en parallèle de 6.
- US2 (import) est testable seule dès la phase 2 ; US1 dès la phase 7 ; US3 et US4 s'appuient sur US1.
- US5 (hors connexion) n'a pas de phase à elle : elle est tenue par la phase 6 et vérifiée en 7, 8, 9 et 10.

## Parallel

- Phase 2 : T009 et T012 ensemble ; T011 après T010a ; puis T017 à T020 ensemble.
- Phase 4 : T028 en parallèle de T026–T027 ; T030a après T030 ; T031 à T033 ensemble.
- Phase 6 : T043 et T044 ensemble.
- Phase 7 : T046, T047, T049 ensemble.

## Implementation Strategy

MVP = phases 1 à 7 : l'import tourne sur l'archive, se coupe seul, et la liste d'un jour s'affiche —
c'est le critère de l'étape. La fiche, l'agenda et le rappel suivent.
