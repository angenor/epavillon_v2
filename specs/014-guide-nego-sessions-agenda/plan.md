# Implementation Plan: Guide Négo — sessions de négociation : l'agenda et son import (étape 3a)

**Branch**: `014-guide-nego-sessions-agenda` | **Date**: 2026-09-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/014-guide-nego-sessions-agenda/spec.md`

**Artefacts** : [research.md](research.md) · [data-model.md](data-model.md) ·
[contracts/](contracts/) · [quickstart.md](quickstart.md)

---

## Summary

Les réunions officielles de la CCNUCC arrivent dans Guide Négo par un import qui lit la source, ne
réécrit que les écarts et **coupe l'affichage** quand la source se tait. L'application les montre jour
par jour, filtrées par thématiques et par groupe, avec leur état et leur fuseau ; la fiche dit ce qui a
changé et d'où cela vient ; « Mon agenda » suit des sessions, signale les chevauchements et rappelle,
dans l'application ouverte, quinze minutes avant.

**L'approche, en une phrase par pièce.**

- **Le modèle** étend `negotiation.meetings` (ADR-008) et ajoute sept tables — points de l'ordre du
  jour, changements, import, journal, traductions, groupes suivis, agenda — et deux
  vocabulaires ([data-model](data-model.md)).
- **L'import** est un travail du worker qui se replanifie, derrière un trait à deux lecteurs ; le jeu
  archivé, rangé dans le dépôt, prouve tout le mécanisme ([R2](research.md), [R3](research.md), [R7](research.md)).
- **La coupure** est une fonction SQL unique, qui voit aussi un worker arrêté ([R5](research.md)).
- **La traduction** est un second travail, par lots, vers OpenRouter ; sans clé, l'anglais seul ([R11](research.md)).
- **L'application** lit toute la COP en une réponse gardée : toutes les fiches se lisent sans réseau
  ([R9](research.md)) ; l'agenda et les groupes passent par la file de 0c.

---

## Technical Context

**Langage / version** : Rust stable (API, worker) · TypeScript strict (Nuxt 4, aucun `any`)

**Dépendances principales** : Actix Web, SQLx vérifié, PostgreSQL 17, Nuxt 4, TailwindCSS v4.
`reqwest` (déjà au workspace) **entre dans le crate `negotiation`** pour le lecteur réel et
OpenRouter. Aucune dépendance nouvelle au workspace ni au front.

**Stockage** : PostgreSQL — `negotiation` (sept tables, colonnes de `meetings`), `reference` (deux
vocabulaires), `platform.settings` (un réglage). Sur l'appareil : garde IndexedDB `guide-nego`
version 3 inchangée — trois clés dans `lectures`, la file `ecritures` de 0c.

**Tests** : `cargo test -p negotiation` sur base réelle et jetable (`kernel::testing::TestDb`) ;
`node --test` pour les règles pures de `utils/guide-nego/` ; écrans à 320 · 360 · 390 px, deux thèmes.

**Plateforme cible** : application installable (PWA), rendu côté navigateur, 360 px de référence ;
back-office de l'ePavillon.

**Objectifs de performance** : la liste s'affiche depuis la garde sans attendre le réseau ; la réponse
de toute une COP reste sous 150 Ko comprimée ; une lecture de l'import sans écart n'écrit que
`last_read_at`, en une requête.

**Contraintes** : hors connexion d'abord ; jamais une liste périmée présentée comme fraîche ; aucun
fichier de plus de 1000 lignes (`useApi.ts` à ~895 : une ligne de montage seulement) ; aucun composant
ni jeton du site dans Guide Négo ; `make check` interdit ; aucun test n'appelle OpenRouter.

**Échelle / périmètre** : trois écrans nouveaux, deux complétés (« Mes thématiques », « Ma journée »),
deux écrans de back-office, six routes d'application, cinq d'administration, deux travaux récurrents.
Quelques centaines de sessions par COP, quelques milliers de personnes.

---

## Constitution Check

*Porte à franchir avant la recherche, et à repasser après la conception.*

| Principe | Verdict | Comment il est tenu |
|---|---|---|
| **I — Le modèle fait autorité** | ✅ | `020_reference.sql`, `100_negotiations.sql`, `900_seed.sql` d'abord ; base locale **migrée** par `migration.sql` rejouable, jamais rechargée ; ligne dans `docs/progression/modele.md` (ADR-017) |
| **II — Frontières de modules** | ✅ | Tout dans `negotiation`, qui ne dépend que de `kernel` et `contracts` ; `tests/frontieres.rs` le garde. Le fuseau de l'édition se lit en SQL par la clé `xmod_fk` existante, sans appel au crate `event` |
| **III — `xmod_fk_*`** | ✅ | `agenda_items.event_id`, `official_imports.event_id`, `agenda_entries.person_id`, `group_subscriptions.person_id`, `theme_set_by`, `updated_by` |
| **IV — Outbox** | ✅ | L'import **n'émet rien** : `tg_meeting_status_event` exempte les lignes importées (FR-021) ; les notifications de changement sont de 3b, qui lira `meeting_changes` |
| **V — Permission et portée** | ✅ | Administration : `Requires<SpaceManage>` sur la portée globale (tranché le 21/09) ; application : lecture publique, agenda et groupes sur soi |
| **VI — SQLx vérifié** | ✅ | Requêtes statiques ; le comparateur travaille en Rust sur la forme pivot, pas en SQL dynamique |
| **VII — Contexte d'écriture** | ✅ | Routes : `Db::write(&ctx)` ; travaux : `job.context()` |
| **VIII — Invariants non réimplémentés** | ✅ | Vocabulaires gardés par `tg_check_term_taxonomy` ; coupure dans `import_is_serving()` ; le code traduit les refus en codes stables |
| **IX — Codes stables** | ✅ | Sept codes nouveaux au catalogue de `kernel` ([contrat](contracts/api-sessions.md)) |
| **X — Base réelle** | ✅ | SC-001 à SC-004 et SC-010 prouvés par des tests d'intégration sur le jeu archivé ; traducteur fixe |
| **XI — Hors connexion** | ✅ | Toute la COP gardée d'une lecture ; agenda et groupes par la file ; « lu à » partout |
| **XII — Confiance** | ⚠️ | Origine, lecture, lien vers l'original, coupure au seuil : tenus. **Écart décidé le 25/09** : les titres traduits par l'IA se publient sans relecture — voir *Complexity Tracking* |
| **XIII — Design borné** | ✅ | Composants nouveaux dans `components/guide-nego/`, sur `design/passation/`, ajoutés à la planche ; back-office en composants `ui/` du site |
| **XIV — Une seule porte** | ✅ | Aucun service ni route nouvelle vers l'IA ; le worker appelle OpenRouter, clé en environnement ([R11](research.md)) |
| **Trois agendas** | ✅ | Famille `/negotiation/sessions` ; « Sessions de négociation » à l'écran ; « Programme » seul nulle part — « Programme officiel de la CCNUCC » nomme la source |
| **1000 lignes** | ✅ | Client d'API dans `composables/api/negotiation-sessions.ts` ; import découpé en `source`, lecteurs, `comparaison`, `denominations`, `traduction` |

---

## Project Structure

### Documentation (this feature)

```text
specs/014-guide-nego-sessions-agenda/
├── plan.md · research.md · data-model.md · quickstart.md
├── contracts/ api-sessions.md · api-admin-import.md · client.md
├── migration.sql        # rejouable — la base se migre
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code

```text
docs/database/
├── 020_reference.sql          # + negotiation_meeting_type, negotiation_group (+ dénominations)
├── 100_negotiations.sql       # + colonnes de meetings, sept tables, import_is_serving()
└── 900_seed.sql               # + ai.drafting_model, import cop31 éteint

backend/crates/modules/negotiation/src/
├── import/                    # NOUVEAU
│   ├── source.rs              # trait SourceOfficielle, SessionLue, EchecLecture
│   ├── ccnucc.rs              # le format réel : JSON du calendrier, titre analysé, heure corrigée
│   ├── archive.rs · reel.rs   # les deux lecteurs
│   ├── archives/cop30/        # le JSON réel réduit — lecture-1.json, lecture-2.json, injoignable
│   ├── comparaison.rs         # pur : pivot × base → écarts ; tests unitaires
│   ├── denominations.rs       # normalisation, résolution type et groupe
│   └── traduction.rs          # trait Traducteur, client OpenRouter
├── jobs/import.rs · jobs/traduction.rs   # NOUVEAUX — replanifiés
├── domain/ · repo/ · service/ · routes/  # sessions.rs, agenda.rs, groups.rs, admin_import.rs
├── routes/openapi.rs · lib.rs            # + chemins, montage, job_handlers
└── tests/                     # import_*.rs, sessions_*.rs, agenda_*.rs, groupes_*.rs
backend/crates/kernel/src/error.rs        # + sept codes
backend/crates/worker/src/main.rs         # armer_les_recurrents : + les imports allumés

frontend/app/
├── pages/guide-nego/negociations/        # index.vue (remplace negociations.vue), [id].vue, agenda.vue
├── pages/guide-nego/thematiques.vue      # + bloc des groupes
├── pages/guide-nego/index.vue            # bloc prochaine-session seulement
├── pages/admin/negociations/import.vue · ordre-du-jour.vue
├── components/guide-nego/                # GnBandeJours, GnLigneSession, GnEtatSession, GnValeurChangee,
│                                         # GnBandeauRappel, GnLectureImpossible (+ planche)
├── components/admin/negotiation/         # ImportStatus.vue, AgendaItemRow.vue
├── composables/api/negotiation-sessions.ts · admin-negotiations.ts (+ import)
├── composables/guide-nego/useGnSessions.ts · useGnAgenda.ts · useGnGroupes.ts · useGnRappel.ts
├── utils/guide-nego/sessions.ts · agenda.ts · edition.ts (+ timezone)
├── types/negotiation-sessions.ts
└── i18n/locales/{fr,en}/…                # voir contracts/client.md
frontend/tests/guide-nego/sessions.test.ts · agenda.test.ts
```

**Structure Decision** — l'import vit dans un sous-dossier `import/` du crate : cinq fichiers qui ne
servent qu'à lui, dont un pur et testé seul. Le jeu archivé est **dans le crate**, embarqué, parce
qu'il est une source comme une autre et que la recette doit tourner sur un binaire construit.

---

## Séquencement

**Un commit par phase**, contrôles ciblés entre-temps ; `make check-safe` en clôture, avec le verrou.

| # | Phase | Ce qu'elle livre | Pourquoi là |
|---|---|---|---|
| 1 | **Le modèle** | SQL des trois fichiers, `migration.sql`, base `epavillon_dev2` migrée et schémas comparés à une base jetable, ligne dans `modele.md` | Rien ne s'écrit avant le SQL |
| 2 | **L'import** (US2) | Trait, lecteurs, jeu archivé, comparaison, dénominations, travail, `import_is_serving`, réarmement du worker ; tests SC-001 à SC-003 | Tout le reste lit ce qu'il écrit |
| 3 | **La traduction** | Trait, client OpenRouter, travail par lots, réglage ; tests avec traducteur fixe (SC-010) | Indépendante des écrans ; l'import la pose |
| 4 | **L'API de l'application** | `GET /negotiation/sessions`, groupes, agenda ; codes ; OpenAPI et `make openapi` ; tests (coupure SC-004, 409, idempotence, audit) | Le client a besoin d'un contrat servi |
| 5 | **Le back-office** | Routes d'administration, écrans « Import » et « Ordre du jour » ; tests de garde globale | Il allume l'import : la recette en dépend |
| 6 | **La plomberie client** | Types, client d'API, `useGnSessions`/`useGnAgenda`/`useGnGroupes`, règles pures et leurs tests, fuseau de l'édition | Conditionne les trois écrans |
| 7 | **US1 — La liste** | Écran 07 et ses quatre états, composants, groupes sur « Mes thématiques » | Le MVP de l'étape |
| 8 | **US3 — La fiche** | Écran 08, valeurs changées, lexique — sans documents liés, que la source ne donne pas | Sur 7 |
| 9 | **US4 — Mon agenda** | Écran 09, chevauchements, rappel et son bandeau, bloc de « Ma journée » | Sur 8 |
| 10 | **Recette** | Quickstart au navigateur, 360 px, clair et sombre ; écart 45 dans `05-design.md` ; planche | — |

### Ce qui ne se fait pas depuis un poste

Le rappel sur un téléphone réel, application ouverte ; la liste relue en mode avion sur Android et
iPhone — ajoutés au § 15 de `docs/DEPLOIEMENT.md`. **La source réelle** : le lecteur est écrit, il
n'est branché sur rien tant que l'accord du secrétariat manque.

## Complexity Tracking

| Écart | Pourquoi il est accepté | Ce qui le borne |
|---|---|---|
| **XII — une production d'IA publiée sans relecture** : les titres français des sessions | Décision du commanditaire du 25/09 : une COP compte des centaines de titres par jour, et une relecture humaine arriverait après la session | Titres de sessions officielles **seulement** ; l'anglais reste affiché et fait foi ; mention « Traduction automatique » sur chaque ligne ; modèle et date gardés ; inscrit dans `COMMENT ON TABLE negotiation.title_translations` et en écart 45 de `05-design.md` |
