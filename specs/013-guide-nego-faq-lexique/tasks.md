---
description: "Tâches de l'étape 2 — FAQ, parcours « Ma première COP » et lexique"
---

# Tasks: Guide Négo — FAQ, parcours « Ma première COP » et lexique (étape 2)

**Input** : les documents de `/specs/013-guide-nego-faq-lexique/`

**Prérequis** : [plan.md](plan.md) · [spec.md](spec.md) · [research.md](research.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

**Tests** : **oui, énumérés par les contrats**.
- Sur base réelle (`crates/modules/negotiation/tests/`) : [api-savoir.md](contracts/api-savoir.md), [api-admin-savoir.md](contracts/api-admin-savoir.md) — URL forgée, invariants traduits, anonymat (SC-009), rejeu d'un `client_ref`.
- Sans navigateur (`frontend/tests/guide-nego/*.test.ts`, `node --test`) : fusion du paquet, recherche floue, résolution ([resolution-lexique.md](contracts/resolution-lexique.md)).

**Branche** : `013-guide-nego-faq-lexique`, existe. **Un commit par phase.** Ports de dev1 : API 8095, site 3004 ; base `epavillon`.

## Format : `[ID] [P?] [Récit] Description`

- **[P]** : parallélisable — fichiers différents, aucune dépendance sur une tâche non finie.
- **[US1]…[US9]** : le récit servi. Les phases de fondation et la recette n'en portent pas.

## Les portes

- **Pendant le cycle** : `npm run typecheck`, `npm run test:guide-nego`, `npm run check:guide-nego`, `cargo test -p negotiation`, `make openapi`, `make check-api-contract`.
- **En fin de cycle seulement, API arrêtée, sous le verrou du protocole** : `make check-safe`.

> **`make check`, `make check-db` et `down -v` ne se lancent jamais** : la base locale se **migre**.

---

## Phase 1 : Fondation — le modèle

**But** : §10 Savoir en base, sur la base locale sans destruction, avec les données d'essai.

- [x] T001 Écrire les vocabulaires `faq_section` (4 termes, `icon` = pictogramme de Guide Négo) et `glossary_family` (3 termes) dans `docs/database/020_reference.sql` §5, sur le patron de `negotiation_theme` ([data-model.md § Vocabulaires](data-model.md))
- [x] T002 Écrire la section « §10 Savoir » en fin de `docs/database/100_negotiations.sql` (termes de vocabulaire **par identifiant** — `section_term_id`, `family_term_id`, `theme_term_id` — gardés par `tg_check_term_taxonomy`, qui lit un `uuid`) : 4 ENUM, `faq_entries`, `faq_related`, `knowledge_sources`, `faq_reads`, `faq_feedback`, `faq_reports`, `expert_questions`, `pathway_groups`, `pathway_steps`, `pathway_checks`, `glossary_entries`, `glossary_related`, `glossary_favorites`, `glossary_proposals`, `glossary_proposal_authors`, avec contraintes nommées, index (`gin_trgm_ops` sur `question_norm`, `term_norm`, `gin` sur `variants_norm`), `updated_at`, audit, `xmod_fk_*`, `COMMENT ON` en français ([data-model.md](data-model.md))
- [x] T003 Dans le même §10 : `negotiation.tg_touch_knowledge_parent()` sur les tables enfants, `negotiation.tg_glossary_slug()` (pose `slug` et `variants_norm`), `first_published_at` et le refus de suppression d'une entrée déjà publiée, `negotiation.glossary_resolve(text)`, `negotiation.knowledge_fingerprint()`, l'événement d'audit `negotiation.expert_question.answered` (patron de `tg_access_request_event`)
- [x] T004 Au §9 de `docs/database/100_negotiations.sql` : permissions `negotiation.knowledge.publish` (→ `admin`, `expert`) et `negotiation.knowledge.review` (→ `expert`), mettre à jour le commentaire qui annonçait l'étape 2
- [x] T005 Charger `docs/database/` sur une base jetable (pas `epavillon`), vérifier `platform.cross_module_fk_report` vide et chaque invariant par un `INSERT` refusé ; noter les commandes dans `specs/013-guide-nego-faq-lexique/quickstart.md` § 0 si elles diffèrent
- [x] T006 Écrire `specs/013-guide-nego-faq-lexique/migration.sql`, rejouable, blocs recopiés mot pour mot de T001–T004 (en-tête et forme de `specs/011-guide-nego-documents/migration.sql`) ; le jouer deux fois sur `epavillon`, comparer `pg_dump --schema-only` avec la base jetable
- [x] T007 Écrire `specs/013-guide-nego-faq-lexique/donnees-essai.sql` depuis `docs/AppNego/design/donnees-savoir.md` et `donnees-lexique.md` : 18 entrées de FAQ (publiée seulement celle qui a sa réponse, datée et vérifiée par le premier compte `expert`, arrêt clair s'il n'y en a pas), 18 étapes en 4 groupes avec leurs liens, 19 termes (les 4 « rédigés » sans source en brouillon, 15 publiés), sources, liés, variantes utiles (`bracketed`, `brackets`) ; `ON CONFLICT DO NOTHING` ; joué deux fois sur `epavillon` ([R15](research.md))
- [x] T008 Consigner le changement de modèle dans `docs/progression/modele.md` (ADR-017)

---

## Phase 2 : Fondation — l'API du paquet

**But** : `GET /negotiation/knowledge` servi, entier et par différence, testé.

- [x] T009 Ajouter les dix-sept codes au catalogue `backend/crates/kernel/src/error.rs`, section « Guide Négo, savoir (étape 2) » ([api-savoir.md § Codes](contracts/api-savoir.md), [api-admin-savoir.md](contracts/api-admin-savoir.md)) ; traduire les contraintes du §10 là où l'étape 1 traduit les siennes
- [x] T010 [P] Ajouter `KnowledgePublish` et `KnowledgeReview` à `backend/crates/modules/negotiation/src/domain/permissions.rs`
- [x] T011 [P] Écrire les formes et la validation du savoir dans `backend/crates/modules/negotiation/src/domain/savoir.rs` (paquet, entrées, sources, parcours, `removed`)
- [x] T012 Écrire `backend/crates/modules/negotiation/src/repo/savoir_paquet.rs` : lecture du publié entier, différence depuis `since − 5 min` avec `removed`, « les plus lues » sur 30 jours avec repli sur `editorial_rank`, empreinte par `negotiation.knowledge_fingerprint()`
- [x] T013 Écrire `service/savoir_paquet.rs` et `routes/savoir.rs` (`GET /negotiation/knowledge`, `ETag`, `If-None-Match` → `304` par `routes::inchange()`, `Cache-Control: public, no-cache`, `served_at`) ; monter dans `src/lib.rs`, ajouter à `routes/openapi.rs`
- [x] T014 Écrire le harnais `backend/crates/modules/negotiation/tests/commun/savoir.rs` (fabriques : entrée de FAQ, terme, étape, expert) et `tests/savoir_paquet.rs` : brouillon jamais servi, `to_review` servi, `since` rend le changé et `removed`, chevauchement, enfant modifié qui fait partir le parent, `304`, une entrée publiée non supprimable (code traduit), `glossary_resolve` dans ses quatre ordres
- [x] T015 `make openapi`, `cargo test -p negotiation`

---

## Phase 3 : Fondation — la plomberie du téléphone

**But** : le savoir gardé, fusionné, cherché et résolu, prouvé sans navigateur.

- [x] T016 [P] Écrire `frontend/app/types/negotiation-savoir.ts` depuis [api-savoir.md](contracts/api-savoir.md)
- [ ] T017 [P] Écrire `frontend/app/mocks/negotiation-savoir.ts` depuis les mêmes données d'essai que T007 (19 termes dont *contact group*, l'entrée de FAQ complète, le parcours)
- [ ] T018 Écrire `frontend/app/composables/api/guide-nego-savoir.ts` (**le paquet seulement**, par `lireEtiquete` avec `since` ; chaque méthode d'écriture s'ajoute dans la tâche qui livre sa route, sinon `check-api-contract` la refuse) et la monter dans `frontend/app/composables/useApi.ts` (une ligne)
- [ ] T019 [P] Écrire `frontend/app/utils/guide-nego/savoir.ts::fusionner(garde, difference)` (dont le filtrage de `related_ids` sur les entrées présentes) et son test `frontend/tests/guide-nego/savoir.test.ts`
- [ ] T020 [P] Écrire `frontend/app/utils/guide-nego/recherche-floue.ts` (index préparé, quatre rangs, Damerau-Levenshtein bornée, « vous cherchiez peut-être », « aussi dans les traductions ») et `frontend/tests/guide-nego/recherche-floue.test.ts` (les 19 termes avec une faute ou en français : dans les trois premiers — SC-003 ; 500 entrées sous 16 ms par requête)
- [ ] T021 [P] Écrire `frontend/app/utils/guide-nego/lexique.ts` (`normaliserTerme`, `resoudreLeTerme`, regroupement par lettre) et `frontend/tests/guide-nego/lexique.test.ts`, dont vingt chaînes communes avec un test Rust qui les passe à `platform.normalize_label` (`backend/crates/modules/negotiation/tests/savoir_normalisation.rs`)
- [ ] T022 Écrire `frontend/app/composables/guide-nego/useGnSavoir.ts` sur `useGnLecture('savoir', …)` : garde, relecture à l'ouverture avec réseau, `since`, fusion, index de recherche préparé une fois
- [ ] T023 `npm run typecheck`, `npm run test:guide-nego`, `make check-api-contract`

---

## Phase 4 : Récit 1 — Un mot entendu en salle, trouvé sans réseau (P1) 🎯 MVP

**But** : le critère de sortie. **Test indépendant** : quickstart § 1.

- [ ] T024 [P] [US1] Créer `frontend/app/components/guide-nego/GnLigneTerme.vue` (terme en italique, traduction, première phrase, surlignage) et l'ajouter à la planche `components/guide-nego/planche/`
- [ ] T025 [P] [US1] Créer `GnRailAlphabet.vue` (toucher et glisser, lettres vides grisées, lettre en cours, 44 px de cible) et l'ajouter à la planche
- [ ] T026 [US1] Remplacer `frontend/app/pages/guide-nego/lexique.vue` : champ actif, « N entrées, sans réseau », derniers consultés (`localStorage`), résultats à la frappe, « vous cherchiez peut-être », « aussi dans les traductions », « aucun résultat » avec « Proposer » (désactivé jusqu'à US7) et « Parcourir la liste », résolution de `?terme=` ([resolution-lexique.md](contracts/resolution-lexique.md)), quatre états, fermeture vers l'écran d'origine
- [ ] T027 [US1] Créer `frontend/app/pages/guide-nego/lexique/[slug].vue` : famille, terme, traduction, définition, « Entendu en salle », sigle, sources, termes liés, « Favori », « Partager » (partage du téléphone, sinon copie du lien)
- [ ] T028 [US1] Créer `frontend/app/pages/guide-nego/lexique/liste.vue` : groupes par lettre et comptes, rail, filtres par famille (`GnPilule`)
- [ ] T029 [US1] Écrire les routes des favoris (`GET /negotiation/me/glossary-favorites`, `PUT`/`DELETE …/{entry_id}`) dans `backend/crates/modules/negotiation/src/routes/savoir.rs` et son service, avec `tests/savoir_favoris.rs` ; `make openapi`, puis leurs méthodes dans `composables/api/guide-nego-savoir.ts`
- [ ] T030 [US1] Écrire `frontend/app/composables/guide-nego/useGnFavorisLexique.ts` (local sans compte, file de 0c en famille `lexique.favori.<id>` avec compte, fusion à la connexion — R7) et `frontend/app/pages/guide-nego/lexique/favoris.vue` ; l'accès depuis le lexique
- [ ] T031 [US1] i18n `frontend/i18n/locales/{fr,en}/pages/guide-nego.lexique.json`, `guide-nego.lexique-entree.json`, `guide-nego.lexique-liste.json`, `guide-nego.lexique-favoris.json`
- [ ] T032 [US1] Vérifier au navigateur, hors connexion, quickstart § 1 étapes 1 à 6 à 360 px, deux thèmes ; `npm run check:guide-nego`

---

## Phase 5 : Récit 2 — La FAQ se lit en entier sans réseau (P1)

**Test indépendant** : quickstart § 2, étapes 1 à 4.

- [ ] T033 [P] [US2] Créer `GnLigneQuestion.vue` (question, bouclier « Vérifié le … ») et `GnSource.vue` (citation en retrait ou ligne de document qui s'ouvre), sur la planche
- [ ] T034 [US2] Créer `frontend/app/pages/guide-nego/ressources/faq/index.vue` : compte, ligne de synchronisation, recherche à la frappe (R4 sur question puis réponse), rubriques et comptes, ligne du parcours et sa progression (lue par `useGnParcours` si présent, sinon masquée jusqu'à US3), « Les plus lues », « Poser une question à un expert » ; quatre états
- [ ] T035 [US2] Créer `ressources/faq/rubrique/[code].vue` et `ressources/faq/[id].vue` : rubrique, question, « Vérifié le », réponse, « Un expert relit cette réponse » si `to_review` (FR-019 bis), sources (document ouvert à la page citée, ou écran « pas sur votre téléphone »), questions liées, emplacements de « Cette réponse vous a-t-elle aidée ? » et « Dépassé ou faux » (actifs à US5)
- [ ] T036 [US2] `POST /negotiation/faq/{id}/read` (route, service, test dans `tests/savoir_lectures.rs`) et son appel une fois par entrée, par jour et par téléphone (R12)
- [ ] T037 [US2] Relier Ressources à la FAQ dans `frontend/app/pages/guide-nego/ressources/index.vue`
- [ ] T038 [US2] i18n `guide-nego.faq.json`, `guide-nego.faq-entree.json`, `guide-nego.faq-rubrique.json`
- [ ] T039 [US2] Vérifier au navigateur quickstart § 2 étapes 1 à 3 ; `npm run check:guide-nego`

---

## Phase 6 : Récit 9 — Le back-office tient la FAQ, le parcours et le lexique (P1)

**Test indépendant** : quickstart § 6.

- [ ] T040 [US9] Écrire `repo/savoir_faq.rs`, `repo/savoir_lexique.rs`, `repo/savoir_parcours.rs` et `service/savoir_admin.rs` : liste (`q` par trigramme), fiche, création, modification (sources et liés en bloc), `verify`, `publish`, `to-review`, `unpublish`, suppression d'un brouillon, parcours et ordre ([api-admin-savoir.md](contracts/api-admin-savoir.md))
- [ ] T041 [US9] Écrire `routes/admin_savoir.rs` avec l'extracteur `LectureSavoir` (publish **ou** review, patron de `LectureDocuments`) ; monter, `openapi.rs`
- [ ] T042 [US9] Tests `tests/savoir_admin.rs` : chaque route nominale, `NEGOTIATION_FAQ_UNVERIFIED`, suppression refusée d'un publié, `slug` jamais accepté en entrée, `verify` refusé à `admin` ; ajouter les nouvelles routes à `tests/perimetre_url_forgee.rs`
- [ ] T043 [US9] `frontend/app/types/admin-negotiation-savoir.ts` et `frontend/app/composables/api/admin-negotiation-savoir.ts` (routes de T041 seulement ; celles de la file s'ajoutent en T050, T056, T069), montée dans `useApi.ts` (une ligne)
- [ ] T044 [P] [US9] Composants `frontend/app/components/admin/negotiation/{FaqForm,GlossaryForm,SourcesField}.vue` sur `ui/` ; `SourcesField` choisit un document de la bibliothèque (section, pages) ou une référence extérieure, avec citation
- [ ] T045 [US9] Pages `frontend/app/pages/admin/negociations/faq/{index,[id]}.vue` et `lexique/{index,[id]}.vue` : filtres, états, actions selon les permissions lues comme `documents/index.vue`
- [ ] T046 [US9] `components/admin/negotiation/PathwayEditor.vue` et `pages/admin/negociations/parcours/index.vue` : groupes, étapes, complément, ordre, publication
- [ ] T047 [US9] Entrées « FAQ », « Lexique », « Parcours » et « File des experts » dans `frontend/app/layouts/admin.vue`, gardées par les deux permissions ; i18n `admin.negociations.faq.json`, `.lexique.json`, `.parcours.json`, `.file.json`
- [ ] T048 [US9] Vérifier quickstart § 6 étapes 1 à 3 ; `make openapi`, `cargo test -p negotiation`, `npm run typecheck`

---

## Phase 7 : Récit 5 — Dire qu'une réponse ne va pas (P2)

**Test indépendant** : quickstart § 2 étape 5.

- [ ] T049 [US5] Routes et service des retours et signalements (`PUT …/feedback` avec l'ouverture `from_feedback`, `POST …/reports` avec `client_ref` et plafond, `GET /negotiation/me/faq-feedback`) dans `routes/savoir.rs`, `service/savoir_retours.rs` ; tests `tests/savoir_retours.rs` (rejeu, plafond 429, une voix, motifs requis)
- [ ] T050 [US5] File des experts, part « signalements » : `repo/savoir_file.rs`, `service/savoir_file.rs`, `routes/admin_file.rs` (`GET /admin/negotiation/queue?kind=reports`, `…/reports/{id}/close`) ; test d'anonymat qui cherche l'identifiant et le nom de l'auteur dans chaque réponse, fiche FAQ du back-office comprise (SC-009) ; test : clore avec `revised`, `confirmed` ou `dismissed` laisse `faq_entries` identique, même `updated_at` (FR-018) ; URL forgée
- [ ] T051 [P] [US5] Créer `GnRetourUtile.vue` (« Oui / Non », « Merci. » et coche verte) sur la planche
- [ ] T052 [US5] Feuilles « Qu'est-ce qui manque ? » et « Dépassé ou faux » dans `ressources/faq/[id].vue` (via `GnFeuilleBasse`, `GnCase`, `GnZoneTexte`), écritures par la file de 0c, sans compte → connexion puis retour à l'entrée
- [ ] T053 [US5] `components/admin/negotiation/QueueItem.vue` et `pages/admin/negociations/file/index.vue`, onglet « Signalements », groupés par entrée, clôture avec issue
- [ ] T054 [US5] i18n ; vérifier quickstart § 2 étape 5

---

## Phase 8 : Récit 6 — Poser une question à un expert (P2)

**Test indépendant** : quickstart § 3.

- [ ] T055 [US6] Routes et service `POST` et `GET /negotiation/me/questions` (`space.access`, `client_ref`) dans `routes/savoir.rs`, `service/savoir_questions.rs` ; tests `tests/savoir_questions.rs`
- [ ] T056 [US6] File, part « questions » : `…/questions/{id}/answer` (met en file `negotiation.expert_question.answered_email`), `…/questions/{id}/promote` (brouillon sans auteur, `NEGOTIATION_QUESTION_NO_CONSENT`) ; tests d'anonymat et de promotion
- [ ] T057 [US6] Travail `negotiation.expert_question.answered_email` (file par défaut, clé `question_id`) dans `backend/crates/modules/negotiation/src/jobs/emails.rs`, gabarit `fr`/`en` dans `src/mail.rs`, enregistré dans `lib.rs::job_handlers()` ; test de mise en file idempotente
- [ ] T058 [US6] `frontend/app/composables/guide-nego/useGnQuestions.ts` (file de 0c, garde personnelle vidée à la déconnexion) ; pages `ressources/faq/question.vue` (formulaire, verrou `GnVerrou` sans accès, écran « Envoyé ») et `ressources/faq/mes-questions.vue`
- [ ] T059 [US6] Onglet « Questions » de `pages/admin/negociations/file/index.vue` : répondre, promouvoir
- [ ] T060 [US6] i18n `guide-nego.question.json`, `guide-nego.mes-questions.json` ; vérifier quickstart § 3 (courriels dans Mailpit)

---

## Phase 9 : Récit 3 — « Ma première COP » (P2)

**Test indépendant** : quickstart § 4.

- [ ] T061 [US3] Routes `GET /negotiation/me/pathway`, `PUT`/`DELETE …/{step_id}` ; tests `tests/savoir_parcours.rs` (idempotence, étape inconnue 404)
- [ ] T062 [P] [US3] Créer `GnEtapeParcours.vue` (case de 24 px, ligne entière cible, gris jamais barré, complément et lien « Lire : … ») sur la planche
- [ ] T063 [US3] `frontend/app/composables/guide-nego/useGnParcours.ts` : local sans compte, file en famille `parcours.<step_id>` avec compte, fusion à la connexion, ligne « mises à jour depuis un autre appareil » quand la relecture diffère (R7)
- [ ] T064 [US3] `frontend/app/pages/guide-nego/ressources/parcours.vue` : progression, prochaine étape, groupes « n sur N », bandeau hors connexion, pied ; liens vers document (section ou page), FAQ, lexique ; brancher la ligne du parcours de la FAQ (T034)
- [ ] T065 [US3] i18n `guide-nego.parcours.json` ; vérifier quickstart § 4

---

## Phase 10 : Récit 4 — La recherche globale (P2)

**Test indépendant** : quickstart § 5 étape 1.

- [ ] T066 [P] [US4] Créer `GnGroupeResultats.vue` et `GnLoupe.vue` sur la planche ; ajouter la loupe à l'en-tête des écrans racines (« Ma journée », « Ressources ») par une prop de `GnEntete.vue` / `GnEcran.vue` (R13)
- [ ] T067 [US4] `frontend/app/pages/guide-nego/recherche.vue` : deux caractères, groupes Lexique, FAQ, Documents avec comptes et mots marqués ; documents en ligne par `chercherDansLeTexte`, hors connexion par la bibliothèque gardée et `chercherDansLeDocument` sur les copies gardées (cinq passages au plus par document) ; ligne « Pas encore ici : … » ; mention hors connexion ; « aucun résultat » propose de soumettre au lexique
- [ ] T068 [US4] i18n `guide-nego.recherche.json` ; inscrire l'écart de la loupe dans `docs/AppNego/05-design.md` ; vérifier quickstart § 5 étape 1

---

## Phase 11 : Récits 7 et 8 — Proposer un terme, la feuille du lecteur (P3)

**Test indépendant** : quickstart § 5 étapes 2 et 3.

- [ ] T069 [US7] Route `POST /negotiation/glossary/proposals` (regroupement par `term_norm`, `NEGOTIATION_GLOSSARY_TERM_EXISTS`, plafond) ; file, part « propositions » : lecture avec entrées proches par `similarity`, `accept`, `reject` ; travail `negotiation.glossary_proposal.published_email` (file par défaut, clé `(proposal_id, person_id)`) mis en file à la **publication** d'une entrée née d'une proposition, gabarit `fr`/`en` dans `src/mail.rs`, enregistré dans `lib.rs::job_handlers()` ; tests `tests/savoir_propositions.rs` (regroupement, rejeu, anonymat, courriels)
- [ ] T070 [US7] Feuille « Proposer un terme » (terme prérempli, contexte 600, sans compte → connexion en gardant la saisie) ouverte depuis « aucun résultat » du lexique et de la recherche globale ; onglet « Termes proposés » de la file au back-office
- [ ] T071 [US8] Créer `frontend/app/components/guide-nego/GnFeuilleTerme.vue` (entrée résolue par `resoudreLeTerme`, pages du document, « Ouvrir dans le lexique », « Revenir au texte » ; sinon « pas encore dans le lexique » et « Proposer ») sur la planche, et y déplacer la feuille de `pages/guide-nego/ressources/documents/[id]/lire.vue`
- [ ] T072 [US7] [US8] i18n ; vérifier quickstart § 5 étapes 2 et 3

---

## Phase 12 : Recette et finitions

- [ ] T073 Dérouler [quickstart.md](quickstart.md) en entier sur la **version construite**, à 360 px, clair et sombre ; corriger ce qu'il trouve
- [ ] T074 Ajouter les essais sur appareil réel de l'étape 2 à la liste du § 15 de `docs/DEPLOIEMENT.md` (lexique en mode avion, rail au doigt, partage, clavier ouvert à « Aa »)
- [ ] T075 Vérifier que les fichiers touchés restent sous mille lignes (`useApi.ts`, `lire.vue`, `error.rs`, `tests/commun/mod.rs`, `tests/perimetre_url_forgee.rs`)
- [ ] T076 `make check-safe` sous le verrou du protocole ; ligne d'état de l'étape dans `docs/AppNego/progress.md`

---

## Dépendances et ordre

- **Phases 1 → 2 → 3** : séquentielles. Le modèle précède tout ; la plomberie du téléphone s'appuie sur le contrat engendré.
- **Phase 4 (US1)** dépend de 3 : c'est le MVP et le critère de sortie.
- **Phase 5 (US2)** dépend de 3. **Phase 6 (US9)** dépend de 2 ; elle peut se mener en parallèle des phases 4 et 5 (back-office d'un côté, Guide Négo de l'autre).
- **Phase 7 (US5)** dépend de 5 et 6 (la file vit au back-office). **Phase 8 (US6)** dépend de 7 (la page de la file). **Phase 9 (US3)** dépend de 3 et 5.
- **Phase 10 (US4)** dépend de 4 et 5. **Phase 11** dépend de 4, 7 et 10.
- **Phase 12** en dernier.

### En parallèle, dans une même phase

- **Phase 2** : T010 et T011.
- **Phase 3** : T016 et T017, puis T019, T020 et T021.
- **Phase 4** : T024 et T025.
- **Phase 6** : T044 pendant T040–T042.

## Stratégie

- **Le MVP de l'étape est le critère de sortie** : phases 1 à 5. *contact group* est trouvé hors connexion (US1), une entrée de FAQ porte sa date de vérification (US2), sur les données d'essai. La phase 6 rend le contenu publiable en production.
- **Ensuite, par valeur** : les retours (US5) et les questions (US6), qui font vivre la FAQ pendant la COP ; le parcours (US3) ; la recherche globale (US4) ; les termes proposés et la feuille du lecteur (US7, US8).
- **Chaque phase se ferme par son commit** et ses contrôles ciblés ; `make check-safe` une seule fois, en fin de cycle.
