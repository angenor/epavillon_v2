---

description: "Task list — Direct + Tableaux de bord (B9)"
---

# Tasks: Direct + Tableaux de bord (B9)

**Input**: Design documents from `/specs/007-direct-tableaux-de-bord/`

**Prerequisites**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: **OBLIGATOIRES.** Le principe X de la constitution impose des tests d'intégration sur base **réelle et jetable**, sans aucun mock de base. Les tâches de test ne sont pas optionnelles ici.

**Organization**: par histoire utilisateur, pour que chaque tranche soit implémentable, éprouvable et livrable seule.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichier différent, aucune dépendance en cours
- **[Story]** : `US1` … `US6`, correspondant aux histoires de [spec.md](spec.md)

## Conventions de chemin

Tout le Rust vit dans `backend/`. **Deux crates sont créés** : `backend/crates/modules/live` et `backend/crates/modules/analytics`. Trois exceptions hors `backend/` : `docs/database/130_analytics.sql` et `docs/database/030_identity.sql`, modifiés **une fois chacun** (T002, T003) ; et la bascule du site, dans `frontend/`, qui **fait partie du livrable**.

---

## ⚠️ Cinq choses à lire avant de commencer

Chacune coûterait cher découverte en chemin, et **quatre sur cinq produisent un défaut entièrement silencieux**.

### 1. Le schéma change — `down -v`, sans exception

**T002** ajoute une ligne de réglage à `130_analytics.sql`, **T003** une attribution de permission à `030_identity.sql`. Le schéma n'est chargé qu'au **premier** démarrage du conteneur : sans destruction du volume, la base garde l'ancien semis **sans le dire**, et deux choses passeraient au vert pour de mauvaises raisons — le seuil d'urgence serait lu sur sa valeur de repli, et le contrôle de permission serait vérifié avec un catalogue périmé. **T004** fait la destruction et vérifie les deux lignes en base.

### 2. Les deux fonctions de publication émettent DÉJÀ

`live.publish_incident()` émet `live.incident.published`, `live.unpublish_incident()` émet `live.incident.resolved` — **dans la transaction de l'appelant** (`080_live.sql` § 6).

C'est le piège de B1 (`anonymize_person()`), de B2 (`merge_organizations()`), de B4 (`tg_guard_proposal_status()`) et de B5 (les deux déclencheurs de séance et d'inscription). **Aucune tâche de ce découpage n'appelle `kernel::events::emit`.** **T071** compte les lignes d'outbox après une publication et exige **exactement une** ; **T138** est un `grep` qui vaut un test.

### 3. `published_by` est posé par la fonction, et une écriture hors du noyau le laisse NUL

`live.publish_incident()` lit `platform.current_actor_id()`. Une écriture qui contournerait `Db::write()` produirait un `published_by` **nul, sans erreur** — et le back-office afficherait « publié par — ». Le défaut ne se verrait qu'à la relecture d'un message publié.

**T072** le vérifie **sur la valeur de la colonne**, pas sur l'audit.

### 4. Un intervalle de rafraîchissement trop court ARRÊTE la chaîne, en silence

La clé d'anti-rebond d'`analytics.enqueue_refresh()` est `refresh_all:<tranche>`, et le conflit de `platform.jobs` porte sur `(tâche, clé)` **quel que soit l'état du travail**, `cancelled` excepté : **un travail déjà réussi bloque une nouvelle mise en file de la même tranche**. Si l'intervalle était plus court que la fenêtre d'anti-rebond, la chaîne se dédoublonnerait contre elle-même et **s'arrêterait sans erreur ni trace**.

**T007** refuse la configuration au démarrage ; **T119** le mesure.

### 5. Le filtre de périmètre NE PEUT PAS être un `WHERE event_id`

`live.incidents` n'a **aucune colonne d'édition** pour les portées `session`, `event_day` et `organization` : le rattachement est un **calcul**, et c'est `live.event_incidents()` qui le fait. Un filtre écrit à la main laisserait fuir **trois portées sur cinq**, et l'écran paraîtrait juste — il montrerait simplement moins que ce qui existe.

**T038** pose les cinq portées et exige les cinq. **T138** cherche mécaniquement tout `WHERE` sur `event_id` dans le crate `live`.

---

## Phase 1 : Amorçage — le modèle, les deux crates, le montage

**Objectif** : les deux crates existent, compilent, sont montés derrière `platform.modules`, et rendent 404 sur tout. Rien n'est encore servi.

- [X] T001 Lire intégralement `docs/database/080_live.sql` § 6 et `docs/database/130_analytics.sql` avant d'écrire une ligne — la règle d'or du dépôt, et la seule protection contre un nom de colonne inventé
- [X] T002 Ajouter à `docs/database/130_analytics.sql`, en fin de fichier, le réglage `platform.settings('analytics.review_alert_days', '21')` avec sa `description` en français disant **pourquoi** ce seuil existe (écart n° 43), et un `ON CONFLICT (key) DO NOTHING` — **dans ce fichier et non dans `900_seed.sql`**, qui porte la mise en garde datée sur les réglages de module écrasés en silence
- [X] T003 Ajouter `('programmer', 'analytics.dashboard.read'),` au bloc `INSERT INTO identity.role_permissions` de `docs/database/030_identity.sql` § 6, avec un commentaire disant que **cela n'accorde aucune élévation** — un programmateur lit déjà, écran par écran et pour sa seule édition, tout ce que le tableau de bord agrège
- [X] T004 `docker compose -f ops/docker-compose.dev.yml down -v && up -d && make wait-db`, puis vérifier en base que `analytics.review_alert_days` vaut `21` et que l'attribution existe — les deux requêtes sont dans [quickstart.md](quickstart.md) § 0
- [X] T005 [P] Créer `backend/crates/modules/live/Cargo.toml` — `kernel`, `contracts`, `actix-web`, `serde`, `serde_json`, `sqlx`, `time`, `tracing`, `utoipa`, `uuid`, plus les `dev-dependencies` du patron de `content`. **Aucune dépendance nouvelle au workspace**, et **aucune ligne vers un autre crate de module**
- [X] T006 [P] Créer `backend/crates/modules/analytics/Cargo.toml` — idem, plus `async-trait` pour le gestionnaire de travail différé
- [X] T007 Ajouter `AnalyticsConfig { refresh_interval, refresh_debounce }` à `backend/crates/kernel/src/config.rs` et le champ `analytics` à `Config`, avec **le contrôle qui refuse le démarrage** si l'intervalle est nul ou **inférieur ou égal** à la fenêtre d'anti-rebond — sur le patron de `EVENT_CALL_AUTOCLOSE_INTERVAL`, qui refuse déjà zéro. Défauts : 15 min et 5 min
- [X] T008 [P] Ajouter `ANALYTICS_REFRESH_INTERVAL` et `ANALYTICS_REFRESH_DEBOUNCE` à `.env.example`, avec un commentaire disant que le premier doit dépasser le second **et pourquoi**
- [X] T009 [P] Créer `backend/crates/modules/live/src/lib.rs` : `pub fn routes()`, `pub fn event_routes()`, `mod` déclarés, et un en-tête de fichier disant **ce que ce crate NE sert pas** — réunions, participants, webhooks, diffusions, soit les quatre cinquièmes du schéma
- [X] T010 [P] Créer `backend/crates/modules/live/src/state.rs` — `LiveState { db, config }`, `pool()`, sur le patron de `ContentState`
- [X] T011 [P] Créer `backend/crates/modules/analytics/src/lib.rs` : `pub fn routes()`, `pub fn job_handlers()`, `pub mod jobs`, et un en-tête disant que **la composition du tableau de bord lit quatre schémas métier en lecture seule, sous `repo/cross/`**, et pourquoi cela ne rompt pas la frontière (R2)
- [X] T012 [P] Créer `backend/crates/modules/analytics/src/state.rs` — `AnalyticsState { db, config }`
- [X] T013 Créer `backend/crates/modules/analytics/src/authz.rs` — `pub struct DashboardRead` implémentant `PermissionSpec` avec le code `analytics.dashboard.read`, **déplacé** depuis `backend/crates/api/src/routes/health.rs`
- [X] T014 Modifier `backend/crates/api/src/routes/health.rs` : réemployer `analytics::authz::DashboardRead` et **retirer le commentaire d'excuse** (« la permission appartient au module `analytics`, qui n'a pas de crate dans ce jalon »), devenu faux. **La route ne bouge pas** : elle fait paire avec `/ready`, et montée derrière `is_mounted` elle disparaîtrait le jour où le module serait éteint (R3)
- [X] T015 Monter les deux modules dans `backend/crates/api/src/lib.rs`, derrière `etat.modules.is_mounted("live")` et `is_mounted("analytics")`, avec un commentaire disant que **`/admin/incidents` et `/events/{id}/incidents` sont des chemins plats** : aucun `web::scope`, le préfixe `/admin` étant partagé et deux scopes du même préfixe ne se complétant pas
- [X] T016 Ajouter les deux états à `backend/crates/api/src/state.rs` et leurs `app_data` dans `lib.rs`
- [X] T017 [P] Créer `backend/crates/modules/live/src/routes/openapi.rs` — `LiveApi`, `ApiErrorBody`, tags « Back-office — messages d'incident » et « Direct », sur le patron de `ContentApi`. **Les chemins s'ajoutent au fil des histoires**, jamais d'avance
- [X] T018 [P] Créer `backend/crates/modules/analytics/src/routes/openapi.rs` — `AnalyticsApi`, tag « Back-office — tableau de bord »
- [X] T019 Fusionner les deux documents dans `backend/crates/api/src/openapi.rs`, chacun derrière son `is_mounted`
- [X] T020 Enregistrer `analytics::job_handlers(db.clone(), &config)` dans `backend/crates/worker/src/main.rs` — **c'est ce seul geste qui fait écouter la file « analytics »**, `JobRegistry::queues()` étant construite à partir des files que les gestionnaires déclarent
- [X] T021 `make check-back` au vert depuis `backend/` : les deux crates compilent, `cargo fmt --check`, `clippy -D warnings` sans un avertissement
- [X] T022 Écrire `backend/crates/modules/live/tests/frontieres.rs` et `backend/crates/modules/analytics/tests/frontieres.rs` : `cargo tree` ne porte **aucune arête** vers `modules/`, et le fichier `Cargo.toml` ne cite aucun crate de module

**Checkpoint** — les deux crates sont montés et muets. `platform.cross_module_fk_report` reste vide.

---

## Phase 2 : Fondations — tout ce que les six histoires partagent

**⚠️ CRITIQUE** : aucune histoire ne commence avant la fin de cette phase.

- [X] T023 [P] Créer `backend/crates/modules/live/src/domain/incident.rs` — `ManagedIncident`, `IncidentState`, `IncidentWriteStatus` (les **dix** issues), `IncidentWriteResult`. Les noms de champs sont **exactement** ceux de `frontend/app/types/admin-incidents.ts`
- [X] T024 [P] Créer `backend/crates/modules/live/src/domain/desk.rs` — `IncidentListScreen`, `LiveDesk`, `LiveDeskSession`, `IncidentTargets`, `IncidentTargetOption`, `IncidentStateCounts`, `OverrunTemplate`. **`hint` est un texte, `starts_at` un instant** : les mélanger avait fait apparaître un horodatage brut dans une liste déroulante
- [X] T025 [P] Créer `backend/crates/modules/live/src/domain/payload.rs` — `CreateIncidentPayload` (avec `from_event_id`), `UpdateIncidentPayload`, `UnpublishIncidentPayload`. **Aucun champ `granted`** : l'API lit sa propre session
- [X] T026 [P] Créer `backend/crates/modules/analytics/src/domain/dashboard.rs` — `AdminDashboard`, `DashboardFigures`, `EventIncident`
- [X] T027 [P] Créer `backend/crates/modules/analytics/src/domain/action.rs` — `AdminAction`, `AdminActionKind` (les cinq), `AdminActionSeverity` (deux niveaux, **pas trois**), `AdminActionExample`
- [X] T028 [P] Créer `backend/crates/modules/analytics/src/domain/figures.rs` — `DashboardKpi`, `DashboardKpiKey`, `DashboardKpiTone`, `TrendPoint`, `BreakdownSlice`. **`value`, `out_of` et `delta` sont des `Option`** : `null` n'est jamais zéro
- [X] T029 Ajouter au catalogue de `backend/crates/kernel/src/error.rs` les trois codes de [contracts/errors.md](contracts/errors.md) § 2 : `LIVE_INCIDENT_SCOPE_TARGET_MISMATCH`, `LIVE_INCIDENT_WINDOW_INVALID`, `LIVE_INCIDENT_NOT_PUBLISHED`, chacun en 422 avec son message français
- [X] T030 Ajouter à `backend/crates/kernel/src/pg_error.rs` la traduction de `ck_incidents_scope_target` (champ `scope`), `ck_incidents_window` (champ `display_until`) et `ck_incidents_unpublish_shape` (`CONFLICT`), cette dernière avec le commentaire disant qu'elle **ne doit jamais remonter** — les fonctions la rendent inatteignable, et la déclarer est le seul moyen de s'apercevoir qu'une écriture les a contournées
- [X] T031 [P] Créer `backend/crates/modules/live/src/repo/mod.rs` avec l'ouverture d'une **transaction de lecture** — `pool.begin()` puis `SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY`. **Pas `Db::write()`** : il prend une connexion d'écriture et pose un contexte d'audit pour une requête qui n'écrit rien
- [X] T032 [P] Créer `backend/crates/modules/analytics/src/repo/mod.rs` avec la même ouverture, et le commentaire qui dit **pourquoi elle règle le problème des neuf instants** : `now()` vaut `transaction_timestamp()` et reste constant, l'isolation ajoutant un instantané unique (R14)
- [X] T033 [P] Créer `backend/crates/modules/live/src/repo/kinds.rs` — les termes actifs de la taxonomie `incident_kind`, dans leur `sort_order`. **Hors `cross/`** : `reference` est le noyau partagé (principe III)
- [X] T034 [P] Créer `backend/crates/modules/live/src/service/authz.rs` — la **portée visée** par une écriture : l'édition pour les portées `event`, `event_day`, `session`, `organization` ; la **portée globale** pour un message `global` (D3)
- [X] T035 [P] Créer `backend/crates/modules/live/tests/commun/mod.rs` — harnais `kernel::testing`, fabriques d'édition, de journée, d'activité, d'organisation et d'incident dans les cinq portées et les cinq états. **Le semis ne fournit aucun incident** : chaque test pose les siens
- [X] T036 [P] Créer `backend/crates/modules/analytics/tests/commun/mod.rs` — harnais, plus l'appel à `analytics.refresh_all(false)` que **tout test lisant un chiffre doit faire d'abord** : les projections matérialisées sont **vides à la création**
- [X] T037 Écrire `backend/crates/modules/live/tests/aucune_ecriture_hors_schema.rs` et son jumeau dans `analytics` : parcourir `src/`, refuser toute occurrence d'`INSERT INTO`, `UPDATE ` ou `DELETE FROM` visant un schéma autre que le sien — **`platform` et `reference` compris**, qu'aucun des deux n'a à écrire

**Checkpoint** — les formes, les erreurs, les gardes et les harnais sont posés. Les histoires peuvent commencer.

---

## Phase 3 : US2 — L'équipe voit ce qui se joue, et ce qui est déjà dit (P1) 🎯 MVP

**Objectif** : l'écran des messages d'incident lit la plateforme réelle — la liste, le poste de direct, les cibles, le gabarit de débordement.

**Test indépendant** : ouvrir `/admin/incidents` sur une édition réelle, API configurée, et retrouver les activités du jour (ou le repli annoncé), les messages dans leur ordre d'action, les compteurs, les neuf natures et les cibles — **sans bandeau de données d'exemple**.

**Pourquoi avant US1**, contre l'ordre de la spécification : le tableau de bord affiche les incidents actifs de l'édition, et un incident n'existe que si quelque chose sait en poser un. Livrer US1 d'abord ferait vérifier sa cinquième famille d'alerte sur une liste vide.

### Tests

- [X] T038 [P] [US2] `tests/portees.rs` : six incidents posés (les cinq portées + un dont la fenêtre est close), `GET /admin/incidents` rend **les cinq portées**, la portée `organization` comprise dès lors que l'organisation anime une activité de l'édition
- [X] T039 [P] [US2] `tests/portee_globale.rs` : un message `global` apparaît dans la liste de **chaque** édition administrée, et un message de portée `session` d'une autre édition **n'y apparaît pas**
- [X] T040 [P] [US2] `tests/ordre_et_compteurs.rs` : l'ordre rendu est **celui de la fonction** — actifs, programmés, brouillons, historique, gravité décroissante à état égal — et les `counts` sont établis **avant tout filtrage**
- [X] T041 [P] [US2] `tests/cible_resolue.rs` : la cible est rendue par son **nom** ; une journée **sans titre** est désignée par sa **date** au format `JJ/MM/AAAA`
- [X] T042 [P] [US2] `tests/poste_jour_edition.rs` : `desk.day` vaut `(now() AT TIME ZONE events.timezone)::date` et non la date du serveur — éprouvé sur une édition dont le fuseau diffère de l'UTC de plusieurs heures
- [X] T043 [P] [US2] `tests/poste_repli.rs` : sans activité aujourd'hui, `is_fallback` est **vrai**, `desk.day` reste **aujourd'hui**, et `sessions` porte les **quatre** prochaines par début croissant
- [X] T044 [P] [US2] `tests/etat_temporel.rs` : pour une activité **publiée**, `temporal_state` rendu par le poste **est égal** à celui de `programme.v_public_schedule`, sur les **cinq** branches. C'est le test qui tient la duplication assumée (R22)
- [X] T045 [P] [US2] `tests/compteur_messages_actifs.rs` : `active_incident_count` compte les messages **actifs de portée `session`** visant l'activité, et rien d'autre
- [X] T046 [P] [US2] `tests/cibles_bornees.rs` : `targets` ne porte que les journées et activités de l'édition, et **seulement** les organisations qui y animent une activité
- [X] T047 [P] [US2] `tests/perimetre.rs` : périmètre vide → **403** ; édition hors périmètre, **URL forgée** → **404**, jamais 403, jamais une liste vide
- [X] T048 [P] [US2] `tests/lecture_sans_permission.rs` : un compte **sans** `live.incident.publish` mais administrant l'édition **voit la liste** — lire n'est pas un privilège
- [X] T049 [P] [US2] `tests/gabarit_debordement.rs` : `GET /admin/incidents/overrun-template?session_id=` rend l'activité, son titre **résolu**, son créneau et son édition ; une activité hors périmètre rend **404**
- [X] T050 [P] [US2] `tests/ordre_des_routes.rs` : `GET /admin/incidents/overrun-template` **n'est pas lu comme un identifiant** — il rend le gabarit, jamais « message introuvable »

### Implémentation

- [X] T051 [US2] Créer `backend/crates/modules/live/src/repo/incidents.rs` — lecture par `live.event_incidents($1, now())`, **jamais un `WHERE i.event_id = $1`**. Les colonnes de la fonction s'annotent **une à une** : une fonction qui rend une table ne porte aucune contrainte de nullité (leçon de B3)
- [X] T052 [P] [US2] Créer `backend/crates/modules/live/src/repo/cross/event.rs` — `event.events` (titre, fuseau, **ville**), `event.event_days` (cibles), `event.rooms` (nom de salle du poste)
- [X] T053 [P] [US2] Créer `backend/crates/modules/live/src/repo/cross/programme.rs` — `programme.sessions` pour le poste, les cibles et le gabarit. **Pas `v_public_schedule`** : elle écarte les activités non publiées, et une activité non publiée peut tomber en panne
- [X] T054 [P] [US2] Créer `backend/crates/modules/live/src/repo/cross/org.rs` — les organisations **animant** au moins une activité de l'édition, même critère que la portée `organization` du modèle
- [X] T055 [US2] Créer `backend/crates/modules/live/src/service/desk.rs` — le poste de direct, son repli à quatre, `is_fallback`, et l'expression d'état temporel **recopiée de `v_public_schedule`** avec un commentaire renvoyant à T044
- [X] T056 [US2] Créer `backend/crates/modules/live/src/service/list.rs` — la composition entière dans **une seule transaction de lecture** : édition, lignes, poste, compteurs, natures, cibles
- [X] T057 [US2] Créer `backend/crates/modules/live/src/routes/admin.rs` et y déclarer `chemins_litteraux` : `GET /admin/incidents` et `GET /admin/incidents/overrun-template`, **dans cet ordre et avant tout chemin paramétré**
- [X] T058 [US2] Ajouter `chemins_de_dossier` au même fichier : `GET /admin/incidents/{id}`, qui retrouve le message **par** `live.event_incidents()` sur les éditions du périmètre — ce qui rend le contrôle et la lecture indissociables
- [X] T059 [US2] Annoter les trois routes pour OpenAPI et les inscrire dans `LiveApi` — descriptions nommant les formes `IncidentListScreen`, `OverrunTemplate`, `ManagedIncident`

### Bascule du site

- [X] T060 [P] [US2] Ajouter `OverrunTemplate` à `frontend/app/types/admin-incidents.ts` — cinq champs, **aucun changé** : c'est nommer une forme aujourd'hui anonyme, ce qu'exige `check-api-contract`, pas une renégociation
- [X] T061 [US2] Dans `frontend/app/composables/api/admin-incidents.ts` : élargir `IncidentsApiDeps` à `call`, `callOrNull` et `send` ; basculer `list` en `call('/admin/incidents', …, { event_id })`, `byId` en `callOrNull`, `overrunTemplate` en `callOrNull('/admin/incidents/overrun-template', …, { session_id })`. **Les paramètres sont rétablis** — `pending` n'en prenait aucun
- [X] T062 [US2] `node frontend/scripts/check-api-contract.mjs --verbose` : **5 routes en attente**, contre 8. Aucune route « laissée en données d'exemple alors que l'API la sert »

**Checkpoint** — **US2 est livrée** : l'écran des messages d'incident lit la plateforme réelle. Le bandeau de données d'exemple a disparu de ses trois lectures.

---

## Phase 4 : US3 — Un message se rédige, se publie, se corrige et se retire (P1)

**Objectif** : les quatre écritures, leur autorisation sur la portée visée, et les dix issues du contrat.

**Test indépendant** : depuis le poste de direct, publier un message sur une activité, le voir passer « En ligne » et remonter en tête, le corriger, le retirer avec motif, et le retrouver à l'historique avec son auteur et son motif.

### Tests

- [X] T063 [P] [US3] `tests/ecriture_brouillon.rs` : enregistrer sans publier rend `created`, l'état est `draft`, et le message **n'apparaît dans aucune lecture active**
- [X] T064 [P] [US3] `tests/ecriture_et_publication.rs` : enregistrer avec `publish` rend `published`, horodaté et attribué, **dans la même transaction**
- [X] T065 [P] [US3] `tests/correction.rs` : `PUT` rend `updated` ; republier un message retiré **efface** `unpublished_at`, `unpublished_by` et `unpublish_reason`
- [X] T066 [P] [US3] `tests/depublication.rs` : `DELETE` rend `unpublished`, **la ligne demeure** avec son instant, son auteur et son motif, et reparaît à l'historique de la liste
- [X] T067 [P] [US3] `tests/depublication_impossible.rs` : retirer un message **jamais publié** rend `not_published` — la levée de `live.unpublish_incident()` est **traduite**, la condition n'est pas rejouée en amont
- [X] T068 [P] [US3] `tests/refus_de_validation.rs` : les trois issues `missing_target`, `missing_message`, `invalid_window`, **chacune en 200**, chacune nommant son champ
- [X] T069 [P] [US3] `tests/refus_de_permission.rs` : sans `live.incident.publish` sur la portée visée, les quatre écritures rendent **200 `{ status: 'forbidden' }`** — **pas 403** : le contrat le nomme et l'écran le traduit dans son formulaire
- [X] T070 [P] [US3] `tests/portee_globale_ecriture.rs` : un compte détaché sur une seule édition **ne peut pas** retirer un message de portée `global` (D3) ; un compte global le peut. La différence est portée par `has_permission()`, sans une ligne de code supplémentaire
- [X] T071 [P] [US3] `tests/outbox.rs` : compter `platform.outbox_events` avant et après une publication et exiger **exactement une** ligne, puis **exactement une** après un retrait. Le jour où un `emit_event` est ajouté « pour faire comme les autres », le compte double et ce test casse
- [X] T072 [P] [US3] `tests/acteur_pose.rs` : après une publication par l'API, `live.incidents.published_by` porte **l'identifiant de la personne**, jamais `NULL` — vérifié sur la valeur de la colonne, pas sur l'audit
- [X] T073 [P] [US3] `tests/perimetre_ecriture.rs` : `from_event_id` hors périmètre → **404** ; périmètre vide → **403**. Aucun message d'erreur ne **nomme** l'édition, l'activité ou l'organisation d'une cible hors périmètre
- [X] T074 [P] [US3] `tests/cible_hors_edition.rs` : viser une journée ou une activité **d'une autre édition** rend `missing_target` — `ck_incidents_scope_target` vérifie la cohérence portée/cible, **jamais l'appartenance à une édition**
- [X] T075 [P] [US3] `tests/traduction_invariant.rs` : forcer un refus de `ck_incidents_scope_target` par une écriture qui contourne la validation du service, et vérifier qu'il ressort en `LIVE_INCIDENT_SCOPE_TARGET_MISMATCH` sur le champ `scope` — jamais en `INTERNAL`, jamais avec le texte brut de PostgreSQL

### Implémentation

- [X] T076 [US3] Étendre `backend/crates/modules/live/src/repo/incidents.rs` : `INSERT` et `UPDATE` de `live.incidents`, **sans jamais toucher** `published_at`, `published_by`, `unpublished_at`, `unpublished_by` ni `unpublish_reason`
- [X] T077 [US3] Ajouter au même fichier les appels à `live.publish_incident($1)` et `live.unpublish_incident($1, $2)` — **jamais un `UPDATE` direct** : l'historique est le sujet, pas un effet de bord
- [X] T078 [US3] Créer `backend/crates/modules/live/src/service/write.rs` — l'ordre des contrôles de [contracts/routes.md](contracts/routes.md) § 4 : périmètre, appartenance de la cible, permission sur la portée visée, validation, écriture, publication. Le titre vide dans les deux langues est écrit **nul**
- [X] T079 [US3] Ajouter les quatre routes à `backend/crates/modules/live/src/routes/admin.rs` — `POST /admin/incidents`, `PUT /admin/incidents/{id}`, `POST /admin/incidents/{id}/publish`, `DELETE /admin/incidents/{id}/publish`. **Le `DELETE` porte un corps**, et c'est délibéré : le chemin est celui de la publication, le verbe dit qu'on la retire, le motif accompagne le geste
- [X] T080 [US3] Annoter les quatre routes pour OpenAPI et les inscrire dans `LiveApi` — descriptions nommant `CreateIncidentPayload → IncidentWriteResult` et ses jumelles, et **disant que les dix issues sortent en 200**

### Bascule du site

- [X] T081 [US3] Dans `frontend/app/composables/api/admin-incidents.ts`, basculer les quatre écritures vers `send` avec **leur verbe** — `send(path, body, mocks, 'POST' | 'PUT' | 'DELETE')` — et **retirer le paramètre `granted`** de leurs signatures
- [X] T082 [US3] Dans `pages/admin/incidents/index.vue`, `nouveau.vue` et `[id].vue`, retirer `granted` des **appels d'API** — et **le garder** là où il sert encore : `hasPermission(granted, 'live.incident.publish', eventId)` décide de l'affichage des boutons, et cet usage-là reste
- [X] T083 [US3] Corriger l'en-tête de `frontend/app/composables/api/admin-incidents.ts` : « aucune de ces **sept** routes n'existe encore » est faux **deux fois** — elles sont huit, et elles existent (écart n° 143)
- [X] T084 [US3] `node frontend/scripts/check-api-contract.mjs --verbose` : **1 route en attente**, le tableau de bord

**Checkpoint** — **JALON 1 (T001–T084)** : **l'écran des messages d'incident est entièrement branché.** On voit ce qui se joue, on publie, on corrige, on retire — et la trace reste.

---

## Phase 5 : US1 — Le tableau de bord dit la vérité de la plateforme (P1)

**Objectif** : `GET /admin/dashboard` rend tout l'écran en une réponse et un instant.

**Test indépendant** : ouvrir `/admin`, API configurée, changer d'édition dans le sélecteur, et retrouver les chiffres que la base porte réellement — **une seule requête** dans l'onglet réseau.

### Tests

- [X] T085 [P] [US1] `tests/composition.rs` : la réponse porte l'édition, le fuseau, l'appel, les actions, les chiffres, la santé et les incidents — **en une seule requête HTTP**
- [X] T086 [P] [US1] `tests/un_seul_instant.rs` : deux parties de la réponse qui dépendent de `now()` — les incidents actifs et l'échéance — parlent du **même instant**, la transaction de lecture le figeant
- [X] T087 [P] [US1] `tests/cinq_familles.rs` : les cinq familles se déclenchent sur une édition qui les réunit, chacune avec son décompte, **trois exemples nommés au plus** et son lien
- [X] T088 [P] [US1] `tests/famille_vide.rs` : une famille sans élément **n'émet aucune ligne** ; une édition où tout va bien rend `actions: []`
- [X] T089 [P] [US1] `tests/seuil_urgence.rs` : passer `analytics.review_alert_days` de 21 à 1 **change** le contenu de la famille « dossiers sans évaluation », sans redéploiement — et la clé supprimée fait retomber sur 21
- [X] T090 [P] [US1] `tests/echeance_applicable.rs` : un dossier confié porte `min(review_assignments.due_at)` sur ses affectations non déportées ; un dossier **sans aucune affectation** entre dans la famille quelle que soit l'échéance
- [X] T091 [P] [US1] `tests/familles_non_filtrees.rs` : les doublons et les messages de portée globale remontent pour un compte détaché, et **ne révèlent l'existence d'aucune autre édition**
- [X] T092 [P] [US1] `tests/indicateurs.rs` : les six indicateurs, chacun tracé à sa colonne ; **`null` n'est jamais zéro** — un taux d'acceptation sur zéro dossier tranché est absent, pas « 0 % » ; `scheduled` n'a **aucun** dénominateur
- [X] T093 [P] [US1] `tests/series_continues.rs` : les deux courbes portent les jours vides **avec zéro**, telles que les projections les rendent ; la variation hebdomadaire est **nulle** sous quatorze jours de série
- [X] T094 [P] [US1] `tests/fraicheur.rs` : `refreshed_at` vaut `max(finished_at)` **sur les succès** ; une exécution partielle plus récente **ne le fait pas avancer** ; aucun rafraîchissement jamais réussi le laisse **nul**
- [X] T095 [P] [US1] `tests/sante.rs` : `health` porte le **code** de chaque indicateur et ses deux seuils, **non recalculés**
- [X] T096 [P] [US1] `tests/entonnoir_absent.rs` : une édition sans appel ni dépôt rend `funnel: null`, **pas un entonnoir de zéros**
- [X] T097 [P] [US1] `tests/perimetre_tableau_de_bord.rs` : périmètre vide → **403** ; édition hors périmètre, URL forgée → **404** ; `analytics.dashboard.read` absente sur l'édition → **403**
- [X] T098 [P] [US1] `tests/programmateur.rs` : un compte `programmer` détaché sur une seule édition **obtient** son tableau de bord — c'est ce que l'attribution de T003 rend possible, et c'est le compte avec lequel la règle métier n° 8 a été vérifiée le 17/08

### Implémentation

- [X] T099 [P] [US1] Créer `backend/crates/modules/analytics/src/repo/projections.rs` — `mv_proposal_funnel`, `mv_daily_submissions`, `mv_daily_registrations`, `mv_reviewer_workload`. **Les quatre autres ne sont pas lues** et le fichier dit pourquoi
- [X] T100 [P] [US1] Créer `backend/crates/modules/analytics/src/repo/health.rs` — `v_operational_health` telle quelle, et `max(finished_at)` sur `analytics.refresh_log` **où `succeeded`**
- [X] T101 [P] [US1] Créer `backend/crates/modules/analytics/src/repo/settings.rs` — le seuil, avec repli sur 21. **Hors `cross/`** : `platform` est le noyau partagé
- [X] T102 [P] [US1] Créer `backend/crates/modules/analytics/src/repo/reference.rs` — `taxonomy_terms` (libellés et `color_hex`) et `countries`. **Hors `cross/`**, même raison
- [X] T103 [P] [US1] Créer `backend/crates/modules/analytics/src/repo/cross/event.rs` — `event.events`, `event.calls_for_proposals`, `event.effective_deadline(call)`
- [X] T104 [P] [US1] Créer `backend/crates/modules/analytics/src/repo/cross/programme.rs` — `v_proposal_dashboard`, `review_assignments`, `detect_conflicts(event)`, `proposals` et `sessions` pour les répartitions
- [X] T105 [P] [US1] Créer `backend/crates/modules/analytics/src/repo/cross/org.rs` — `duplicate_candidates` jointes à `organizations`, triées par score décroissant. **`v_platform_overview` n'est pas lue** : le seul chiffre qu'on lui aurait pris s'obtient ici, **avec ses trois exemples nommés** (R18)
- [X] T106 [P] [US1] Créer `backend/crates/modules/analytics/src/repo/cross/live.rs` — `live.active_incidents_for_event($1, now())`. **Une fonction SQL, jamais le crate `live`**
- [X] T107 [US1] Créer `backend/crates/modules/analytics/src/service/actions.rs` — les cinq familles, leur critère, leurs trois exemples, et le rangement : gravité, puis échéance la plus proche, puis décompte
- [X] T108 [US1] Créer `backend/crates/modules/analytics/src/service/figures.rs` — les six indicateurs, les deux courbes, les deux répartitions (huit parts, queue regroupée **si elle en compte au moins deux**), l'échéance et la fraîcheur
- [X] T109 [US1] Créer `backend/crates/modules/analytics/src/service/dashboard.rs` — la composition entière dans **une seule transaction de lecture**
- [X] T110 [US1] Créer `backend/crates/modules/analytics/src/routes/admin.rs` — `GET /admin/dashboard`, gardée par `Perimeter` **et** `analytics.dashboard.read` sur l'édition demandée
- [X] T111 [US1] Annoter la route pour OpenAPI et l'inscrire dans `AnalyticsApi` — description nommant `AdminDashboard`

### Bascule du site

- [X] T112 [US1] Dans `frontend/app/composables/useApi.ts`, basculer `admin.dashboard` de `pending` vers `callOrNull('/admin/dashboard', …, { event_id: eventId })`
- [X] T113 [US1] `node frontend/scripts/check-api-contract.mjs --verbose` : **0 route en attente**

**Checkpoint** — **US1 est livrée.** Les deux écrans du back-office lisent la plateforme réelle.

---

## Phase 6 : US4 — Les chiffres ne vieillissent pas en silence (P2)

**Objectif** : le rafraîchissement périodique des huit projections, par le worker.

**Test indépendant** : lancer le worker, attendre une période, constater une exécution complète réussie dans `analytics.refresh_log` — puis voir `refreshed_at` avancer à l'écran.

### Tests

- [X] T114 [P] [US4] `tests/file_ecoutee.rs` : au démarrage, le worker déclare **« analytics »** parmi ses files — conséquence du seul fait que le gestionnaire la nomme
- [X] T115 [P] [US4] `tests/chaine_armee_une_fois.rs` : appeler `planifier` **dix fois** dans la même tranche ne pose **qu'un** travail
- [X] T116 [P] [US4] `tests/rafraichissement.rs` : l'exécution rafraîchit les huit projections **en mode concurrent**, journalise chacune, et replanifie la chaîne à +intervalle
- [X] T117 [P] [US4] `tests/echec_isole.rs` : une projection rendue inaccessible n'empêche pas les sept autres — **7 succès, 1 échec** —, l'échec est journalisé, et **`refreshed_at` ne bouge pas**
- [X] T118 [P] [US4] `tests/anti_rebond.rs` : cent appels à `analytics.enqueue_refresh()` dans la même minute ne produisent **qu'un** travail
- [X] T119 [P] [US4] `tests/intervalle_refuse.rs` : une configuration dont l'intervalle est **inférieur ou égal** à la fenêtre d'anti-rebond **refuse le démarrage**, en nommant le réglage. C'est le seul défaut du jalon qui serait entièrement silencieux

### Implémentation

- [X] T120 [US4] Créer `backend/crates/modules/analytics/src/jobs/refresh.rs` — `RefreshAll` implémentant `JobHandler` : `task()` rend `analytics.refresh_all`, `queue()` rend `analytics`, `carries_secret()` rend **faux** (la charge utile est la seule matière de diagnostic d'un rafraîchissement mort)
- [X] T121 [US4] Dans le même fichier, `run()` : appeler `analytics.refresh_all(true)` **sur le pool, hors transaction d'écriture** (R8), journaliser les vues en échec **en avertissement sans rendre d'erreur**, puis replanifier
- [X] T122 [US4] Ajouter `planifier()` et `prochaine_occurrence()` au même fichier — `planifier` appelle **`analytics.enqueue_refresh()`**, la fonction du modèle, et **jamais `kernel::jobs::enqueue`** : c'est elle qui pose la file, la priorité, les tentatives et la clé d'anti-rebond
- [X] T123 [US4] Armer la chaîne dans `backend/crates/worker/src/main.rs`, dans `armer_les_recurrents`, sur le patron des six chaînes existantes

**Checkpoint** — **JALON 2 (T001–T123)** : **les deux écrans lisent la plateforme, et les chiffres restent frais.** C'est le point à partir duquel le jalon est utile. Les deux phases suivantes se reportent sans rien casser.

---

## Phase 7 : US5 — Un bandeau publié se voit du public (P3)

**Objectif** : la lecture publique des messages actifs d'une édition, et le bandeau sur la page des programmations.

**Test indépendant** : publier un message de portée `session` depuis le back-office, ouvrir `/programmations` sur cette édition **dans une session de navigateur neuve, sans cookie**, et voir le bandeau **nommer l'activité**.

> **Cette histoire a été amendée le 27/08 en écrivant le plan** : elle visait « la page publique d'une activité », **qui n'existe pas** (R26). L'exposition se fait à l'échelle de l'**édition**, par la fonction **descendante**.

### Tests

- [X] T124 [P] [US5] `tests/lecture_publique.rs` : `GET /events/{id}/incidents` **sans session** rend les messages actifs, le plus grave en tête, chacun avec son `target_label` **résolu**
- [X] T125 [P] [US5] `tests/lecture_publique_bornee.rs` : un message dépublié, hors fenêtre ou en brouillon **ne sort pas** ; une édition inconnue rend **une liste vide, jamais 404** — cette route ne dit pas si une édition existe
- [X] T126 [P] [US5] `tests/pas_de_capture_de_chemin.rs` : `GET /events/{slug}` du module `event` continue de répondre, et `/events/{id}/incidents` ne la capture pas — les motifs ne se recouvrent pas, vérifié plutôt que supposé

### Implémentation

- [X] T127 [US5] Créer `backend/crates/modules/live/src/repo/active.rs` — `live.active_incidents_for_event($1, now())`
- [X] T128 [US5] Créer `backend/crates/modules/live/src/routes/public.rs` — `GET /events/{event_id}/incidents`, **route plate, aucune garde**. Le module `event` déclare ses routes `/events/...` à plat : rien à composer côté API
- [X] T129 [US5] Annoter la route et l'inscrire dans `LiveApi` — description nommant `ActiveIncident[]`

### Bascule du site

- [X] T130 [US5] Ajouter la lecture à `frontend/app/composables/useApi.ts` — `incidents.forEvent(eventId)` par `call('/events/${eventId}/incidents', …)`, avec son repli sur `mocks/incidents.ts`
- [X] T131 [US5] Monter `UiIncidentBanner` dans `frontend/app/pages/programme.vue` pour l'édition ouverte : **trois au plus**, le plus grave en tête, le reste replié en « +N » — la règle des pastilles de la charte. Le bandeau **nomme son sujet** par `target_label`
- [X] T132 [US5] Vérifier au navigateur, session neuve, à 375 px et en thème sombre : le bandeau s'affiche, se referme s'il est refermable, et **disparaît** après dépublication

**Checkpoint** — **US5 est livrée** : un message publié atteint enfin quelqu'un.

---

## Phase 8 : US6 — Le dépôt cesse d'annoncer une dette qui n'existe plus (P3)

**Objectif** : zéro route en attente, et trois affirmations fausses corrigées.

**Test indépendant** : `make check-api-contract` compte **0 route en attente**, et aucun des trois fichiers d'en-tête n'annonce d'écran en données simulées.

- [X] T133 [P] [US6] Corriger l'en-tête de `pending()` dans `frontend/app/composables/useApi.ts` : « trois écrans du jalon lisent encore des données simulées » est faux — il n'y en a plus aucun. La primitive **reste**, pour le prochain écran qui en aura besoin
- [X] T134 [P] [US6] Corriger l'en-tête de `frontend/app/composables/useMockData.ts`, qui porte la même affirmation
- [X] T135 [P] [US6] Corriger `CLAUDE.md` § Périmètre actuel : le paragraphe « Trois écrans lisent encore des données d'exemple » disparaît. Dire à la place que **les jeux d'exemple restent** pour les tests et le travail hors ligne, ce qui est désormais leur seule raison d'être
- [X] T136 [US6] Vérifier que, `NUXT_PUBLIC_API_BASE` **vide**, `frontend/app/pages/admin/index.vue`, `pages/admin/incidents/index.vue` et `pages/programme.vue` fonctionnent toujours sur les jeux d'exemple de `frontend/app/mocks/`
- [X] T137 [US6] `make check-api-contract` (`frontend/scripts/check-api-contract.mjs`) : **0 route en attente**, **0 route laissée en données d'exemple alors que l'API la sert**

**Checkpoint** — **JALON 3 (T001–T137)** : **la dernière dette de données simulées du projet est fermée.**

---

## Phase 9 : Polissage et vérifications transverses

- [X] T138 Écrire `backend/crates/modules/live/tests/aucun_filtre_event_id.rs` : un `grep` qui vaut un test — aucun fichier de `src/` ne compare `incidents.event_id` à un paramètre d'édition, et aucun n'appelle `kernel::events::emit`
- [X] T139 [P] Relire les deux `repo/cross/` : `live` porte **event, programme, org** ; `analytics` porte **event, programme, org, live**. **Ni `platform.rs`, ni `reference.rs`** — le noyau partagé n'est pas une frontière (principe III), et les y ranger ferait perdre au dossier son sens
- [X] T140 [P] Vérifier par `wc -l` qu'aucun fichier de `backend/crates/` ni de `frontend/app/` ne dépasse **1000 lignes** ; découper par écran ou par entité si c'est le cas
- [X] T141 [P] `make openapi` puis relire `frontend/app/types/api.ts` : les **9 chemins** y figurent avec leurs paramètres et leurs codes. **Ne pas modifier ce fichier à la main**
- [X] T142 Reprendre `frontend/app/pages/admin/index.vue` et `pages/admin/incidents/index.vue` au navigateur, API configurée : quatre états (chargement, vide, erreur, **accès refusé**), thème sombre **basculé à chaud**, `/en/admin` sans clé brute, cibles tactiles à 44 px, et `scrollWidth === clientWidth === 375`
- [X] T143 Vérifier les trois comptes de [quickstart.md](quickstart.md) § 3 sur les neuf routes : globale, programmatrice détachée, sans droit — et l'URL forgée
- [X] T144 Dérouler [quickstart.md](quickstart.md) en entier, § 0 à § 12
- [X] T145 `make check` **depuis la racine du dépôt**, base détruite et rechargée de zéro (`Makefile`, cibles `check-db`, `check-front`, `check-back`)
- [X] T146 Mettre à jour la progression : journal du jour, [`ecrans/b9-direct-tableaux-de-bord.md`](../../docs/progression/ecrans/b9-direct-tableaux-de-bord.md), décisions du jour, [`modele.md`](../../docs/progression/modele.md) — **le SQL a bougé deux fois** — et la ligne de suivi de `docs/PROGRESSION.md`

---

## Dependencies & Execution Order

### Dépendances de phase

- **Phase 1 (Amorçage)** : aucune dépendance. **T002 et T003 avant T004**, et **T004 avant toute compilation** — SQLx vérifie ses requêtes contre la base réelle
- **Phase 2 (Fondations)** : dépend de la phase 1. **Bloque les six histoires**
- **Phase 3 (US2)** : dépend de la phase 2
- **Phase 4 (US3)** : dépend de **US2** — les écritures se relisent par la composition de la liste, et le poste de direct est le point d'entrée du geste
- **Phase 5 (US1)** : dépend de la phase 2 seulement. **Mais se livre après US3** : le tableau de bord affiche les incidents actifs, et sa cinquième famille se vérifierait sur une liste vide sans quelque chose qui sache en poser un
- **Phase 6 (US4)** : dépend de **US1** — la fraîcheur n'a de sens que si un écran l'affiche
- **Phase 7 (US5)** : dépend de **US3** — il faut savoir publier pour vérifier qu'un bandeau s'affiche
- **Phase 8 (US6)** : dépend de US1, US2, US3 et US5 — c'est la porte de sortie
- **Phase 9 (Polissage)** : dépend de tout

### Dépendances entre histoires

```
Fondations
    ├── US2 (P1) ─── US3 (P1) ─── US5 (P3) ──┐
    └── US1 (P1) ─── US4 (P2) ───────────────┤
                                             └── US6 (P3)
```

**US1 et US2 sont indépendantes** et peuvent être menées en parallèle par deux personnes. Tout le reste suit la chaîne.

### Parallélisme

- **Phase 1** : T005, T006, T008, T009, T010, T011, T012 en parallèle
- **Phase 2** : T023 à T028 en parallèle (six fichiers de formes), puis T031 à T036 en parallèle
- **Phase 3** : les treize tests T038–T050 en parallèle ; puis T052, T053, T054 en parallèle
- **Phase 4** : les treize tests T063–T075 en parallèle
- **Phase 5** : les quatorze tests T085–T098 en parallèle ; puis T099 à T106 en parallèle (huit fichiers de repo)
- **Phase 6** : les six tests T114–T119 en parallèle
- **Phase 9** : T139, T140, T141 en parallèle

---

## Parallel Example: US1

```bash
# Les quatorze tests du tableau de bord, ensemble :
Task: "tests/composition.rs — une seule requête"
Task: "tests/cinq_familles.rs — trois exemples nommés au plus"
Task: "tests/famille_vide.rs — aucune ligne à zéro"
Task: "tests/indicateurs.rs — null n'est jamais zéro"
Task: "tests/programmateur.rs — le compte détaché obtient son tableau de bord"
…

# Puis les huit fichiers de lecture, ensemble :
Task: "repo/projections.rs — les quatre projections lues"
Task: "repo/health.rs — la vue de santé et le journal"
Task: "repo/settings.rs — le seuil, avec repli"
Task: "repo/cross/event.rs · cross/programme.rs · cross/org.rs · cross/live.rs"
```

---

## Implementation Strategy

### MVP d'abord (US2 seule)

1. Phase 1 — Amorçage, **`down -v` compris**
2. Phase 2 — Fondations
3. Phase 3 — US2
4. **S'ARRÊTER ET VÉRIFIER** : l'écran des messages lit la plateforme réelle, sans bandeau de données d'exemple
5. Le contrat compte **5 routes en attente** au lieu de 8 — la dette recule, visiblement

### Livraison incrémentale

1. Amorçage + Fondations → les deux crates sont montés
2. US2 → l'écran lit → **jalon lisible**
3. US3 → l'écran écrit → **JALON 1**
4. US1 → le tableau de bord lit
5. US4 → les chiffres restent frais → **JALON 2, le jalon utile**
6. US5 → le bandeau atteint le public
7. US6 → la dette disparaît → **JALON 3**

### En équipe

Après les fondations : une personne prend **US2 puis US3 puis US5** (la chaîne du direct), une autre **US1 puis US4** (la chaîne de la mesure). Les deux chaînes ne partagent qu'une fonction SQL — `live.active_incidents_for_event()` —, jamais un fichier.

---

## Notes

- **[P]** = fichiers différents, aucune dépendance en cours
- **Les tests d'abord**, et ils doivent **échouer** avant l'implémentation
- **`down -v` avant la première compilation** : deux lignes de semis ont changé
- **Aucun appel à `kernel::events::emit`** dans ce jalon : les deux fonctions de publication émettent déjà
- **Aucune écriture hors du schéma du module**, `platform` et `reference` compris
- Commit après chaque tâche ou groupe logique
- S'arrêter à n'importe quel checkpoint pour vérifier une histoire seule
