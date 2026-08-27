# Implementation Plan: Direct + Tableaux de bord (B9)

**Branch**: `007-direct-tableaux-de-bord` | **Date**: 2026-08-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/007-direct-tableaux-de-bord/spec.md`

---

## Summary

Créer **deux** crates — `backend/crates/modules/live` et `backend/crates/modules/analytics` — pour fermer la dernière dette de données simulées du dépôt : les **huit** routes que `make check-api-contract` compte encore en attente, plus **une** lecture publique qui fait qu'un message publié atteigne enfin quelqu'un. Neuf routes, deux crates, un travail différé.

L'approche tient en dix points, tous déduits du modèle, du contrat du site, d'un précédent livré, ou d'une mesure :

1. **La composition du tableau de bord vit dans `analytics`, sous `repo/cross/`** — et le dossier ne porte que les **quatre schémas métier** lus, `platform` et `reference` étant le noyau partagé que le principe III exempte déjà (R2).
2. **Une transaction de lecture, un instant.** `now()` vaut `transaction_timestamp()` : les dix lectures d'une composition parlent du même instant **sans qu'on passe un horodatage de main en main**, et `REPEATABLE READ` y ajoute un instantané unique. C'est la réponse exacte au « neuf instants de mesure » que le contrat du site interdit (R14).
3. **`refresh_all(true)` fonctionne depuis une fonction et dans un bloc transactionnel — mesuré sur la base du dépôt**, parce que le contraire aurait journalisé huit échecs sans lever et fait vieillir le tableau de bord en silence (R8).
4. **L'intervalle de rafraîchissement doit dépasser la fenêtre d'anti-rebond**, sans quoi la chaîne récurrente se dédoublonne contre elle-même et s'arrête, sans erreur et sans trace. 15 minutes contre 5 (R9).
5. **Le tableau de bord se garde par `analytics.dashboard.read`, et le rôle `programmer` la reçoit** — une ligne au catalogue, aucune élévation, et sans elle le compte qui a servi à valider l'écran au navigateur en serait refusé (R10). **La spécification est amendée, et l'amendement est daté.**
6. **La page publique d'une activité n'existe pas.** L'exposition publique se fait donc sur **la page des programmations**, à l'échelle de l'édition, par la fonction **descendante** — et le message y **nomme son activité**, `target_label` étant déjà résolu par le modèle (R26). **Second amendement, daté.**
7. **Le service n'émet aucun événement** : les deux fonctions de publication le font déjà. C'est le piège n° 1 des six modules précédents, nommé avant d'être commis (R5).
8. **Le filtre de périmètre passe par la fonction, jamais par un `WHERE event_id`** : trois portées sur cinq n'ont aucune colonne d'édition (R7).
9. **La ligne de partage entre un refus HTTP et un refus en 200 est écrite** : le périmètre est un contrôle d'accès qui ne figure pas au contrat et ne révèle rien ; tout le reste est une issue prévue, qui s'affiche dans le formulaire (R12).
10. **Aucune écriture hors du schéma du module**, dans aucun des deux, et un contrôle mécanique le vérifie — comme en B6, et plus strictement qu'en B3, B4 et B5 (R29).

---

## Technical Context

**Language/Version** : Rust stable, édition 2021, chaîne épinglée par `backend/rust-toolchain.toml` — inchangée depuis B1.

**Primary Dependencies** : **aucune déclaration nouvelle.** Les deux crates ne prennent que ce que `content` prend déjà — `kernel`, `contracts`, `actix-web`, `serde`, `serde_json`, `sqlx`, `time`, `tracing`, `utoipa`, `uuid` —, plus `async-trait` pour le seul gestionnaire de travail différé, déjà au workspace. C'est le premier jalon de la phase B qui n'ajoute aucune dépendance, et c'est normal : tout le calcul vit en base.

**Storage** : PostgreSQL 17 + pgvector. **Deux lignes de semis ajoutées au modèle, aucune table, aucune colonne, aucun type, aucune fonction** :

| Ajout | Fichier | Pourquoi |
|---|---|---|
| `platform.settings('analytics.review_alert_days', 21)` | `130_analytics.sql` | Écart n° 43, ouvert depuis le 17/08 : le seuil est écrit dans le code du site, ce que le principe I interdit (D2, R16) |
| `role_permissions('programmer', 'analytics.dashboard.read')` | `030_identity.sql` § 6 | Sans elle, le tableau de bord est refusé au rôle qui pilote une édition — et au compte qui a servi à le vérifier (R10) |

Ni Garage ni Valkey ne servent ici.

**Testing** : `cargo test` sur base **réelle et jetable**, harnais `kernel::testing` de B1. Aucun mock de base. Le semis ne fournit **aucun incident** : chaque test pose les siens. Les projections matérialisées sont **vides à la création** — un test qui lit un chiffre commence par `analytics.refresh_all(false)`, et c'est une contrainte de rédaction, pas un détail.

**Target Platform** : serveur Linux ; développement macOS et Linux via `ops/docker-compose.dev.yml`.

**Project Type** : service web (API) adossé à un front Nuxt existant. `api` monte les routes des deux modules derrière `platform.modules` ; `worker` enregistre **un** travail différé et **aucun** consommateur d'outbox (R28).

**Performance Goals** : **aucune cible chiffrée, et trois garanties de comportement.** Le tableau de bord répond en **un aller-retour** et **un instant de mesure** — c'est une propriété du code, vérifiable, pas un débit. Le rafraîchissement des huit projections est **concurrent**, donc ne bloque aucune lecture. Et l'anti-rebond du modèle garantit qu'une rafale d'événements ne produit **qu'un** travail. Un point à surveiller : la composition du tableau de bord tient une connexion de lecture pendant une dizaine de requêtes ; elle est en lecture seule et son isolation ne bloque rien, mais elle n'a pas à s'allonger.

**Constraints** : `DATABASE_URL` renseignée et base démarrée pour compiler · **`down -v` obligatoire au premier lancement**, deux lignes de semis ayant changé et le schéma n'étant chargé qu'au premier démarrage du conteneur · aucun fichier de `backend/` ni de `frontend/` au-dessus de 1000 lignes · aucun avertissement Clippy · **aucune arête entre les deux crates ni vers un autre module** · **aucune écriture hors du schéma du module** · les noms de champs sont **exactement** ceux de `frontend/app/types/admin-dashboard.ts`, `types/analytics.ts`, `types/live.ts` et `types/admin-incidents.ts`.

**Scale/Scope** : **9 routes HTTP**, **2 crates créés**, **1 travail différé**, **0 consommateur d'outbox**, **0 événement émis** (deux le sont par la base), **3 codes d'erreur** ajoutés au catalogue, **2 lignes de semis** ajoutées au modèle, **1 section de configuration** ajoutée au noyau, **1 forme ajoutée** au contrat du site (`OverrunTemplate`), **1 déclaration de permission déplacée** de `api` vers `analytics`, **8 appels du site basculés** de la primitive d'attente aux primitives réelles, **3 en-têtes corrigés**. Volumétrie de référence : quelques dizaines de messages par édition, huit projections de quelques milliers de lignes.

---

## Constitution Check

*GATE : à passer avant la phase 0, à repasser après la phase 1.*

| # | Principe | Évaluation avant conception | Évaluation après conception |
|---|----------|------------------------------|------------------------------|
| I | Le modèle de données fait autorité | ✅ `080_live.sql` § 6 et `130_analytics.sql` relus intégralement, plus `075` § 1, 6 et 7, `070` § 5, `060` § 5, `040` § 5, `030` § 3 et 6, `010` § 1 et 5. Aucune modification annoncée hors le seuil de l'écart n° 43 | ⚠️ **Deux lignes de semis, et rien d'autre** : le seuil d'urgence (R16) et une attribution de permission (R10). Ni table, ni colonne, ni type, ni fonction — **un plan de moins que B6**, qui ajoutait une fonction. Justifiées en « Complexity Tracking ». **Et la conception a plié devant le modèle huit fois** : le service n'émet rien (R5) ; l'état d'un incident n'est jamais recomposé (R6) ; le filtre de périmètre passe par la fonction (R7) ; la mise en file passe par `enqueue_refresh()` (R9) ; le rafraîchissement passe par `refresh_all()` (R8) ; la fraîcheur vient du journal (R19) ; `v_platform_overview` n'est finalement **pas lue** (R18) ; et l'expression d'état temporel est **recopiée** de la vue plutôt qu'améliorée, avec un test qui l'y attache (R22) |
| II | Frontières de modules | ✅ Deux crates, un par schéma. Aucun lien nécessaire entre eux : le seul rapport — le tableau de bord affiche les incidents actifs — passe par une **fonction SQL** | ✅ **Tenu, et c'est le jalon qui lit le plus loin hors de son schéma.** `analytics/repo/cross/` porte **quatre** fichiers (event, programme, org, live) et **rien d'autre** : `platform` et `reference` sont le noyau partagé que le principe III exempte nommément, et les ranger dans `cross/` ferait perdre au dossier son sens — il doit lister exactement ce qu'un découplage aurait à couper. **Aucune écriture hors schéma**, vérifiée mécaniquement (R29). `cargo tree -p live` et `-p analytics` sans arête vers `modules/`. **Le noyau gagne une section de configuration**, pas un mécanisme |
| III | Frontières vérifiables en base (`xmod_fk_*`) | ✅ Aucune clé étrangère créée | ✅ `platform.cross_module_fk_report` reste vide sans qu'on y touche : les deux ajouts sont des lignes de données |
| IV | Effets de bord par l'outbox transactionnel | ✅ `platform.emit_event()` dans la transaction, jamais d'insertion à la main | ✅ **Le module émet zéro événement, et c'est le résultat d'une lecture, pas d'un oubli** : `live.publish_incident()` émet `live.incident.published`, `live.unpublish_incident()` émet `live.incident.resolved` — les deux **dans la transaction de l'appelant**. Un test compte les lignes d'outbox pour que le jour où quelqu'un ajoute un `emit_event` « pour faire comme les autres », le compte double et le test casse. **Aucun consommateur**, et [`contracts/events.md`](contracts/events.md) dit pourquoi (R28). Le travail différé passe par `platform.jobs` et l'anti-rebond du modèle |
| V | Autorisation par permission et par portée | ⚠️ **Un écart trouvé avant d'écrire une ligne** : FR-018 gardait le tableau de bord par le seul périmètre, alors que le modèle porte `analytics.dashboard.read` et que `/health` la teste depuis B1 | ⚠️ **Une ligne ajoutée au catalogue des rôles, et elle est le contraire d'une élévation** (R10) : un programmateur lit déjà, écran par écran et pour sa seule édition, tout ce que le tableau de bord agrège. **Les quatre écritures se vérifient sur la portée VISÉE**, portée globale comprise pour un message global (D3) — et `has_permission()` l'exclut d'un compte détaché **sans une ligne de code**. **Les quatre lectures d'incidents n'exigent aucune permission**, et aucune n'est ajoutée : elle protégerait un texte déjà public. **Les trois cas du périmètre restent distincts**, aucun droit → refus explicite |
| VI | SQLx vérifié à la compilation, pas d'ORM | ✅ Macros `query!`/`query_as!` | ✅ **Aucune exception, aucun SQL dynamique.** Trois traversées explicites : `platform.i18n_text` en `jsonb` (le patron de `content`), les énumérations `live.incident_scope` et `live.incident_severity` en texte avec cast (le patron de `identity` et de `programme`), et **les colonnes des fonctions qui rendent une table s'annotent une à une** — une fonction ne porte aucune contrainte de nullité, leçon de B3 payée deux fois |
| VII | Contexte d'écriture systématique | ✅ Unique porte d'écriture du noyau | ✅ Tenu, et **c'est ici une condition de justesse et non une trace** : `live.publish_incident()` pose `published_by` à partir de `platform.current_actor_id()`. Une écriture qui contournerait `Db::write()` produirait un `published_by` **nul**, sans erreur — le back-office afficherait « publié par — ». Un test le vérifie sur la valeur, pas sur l'audit. Le rafraîchissement, lui, est **écrit hors transaction d'audit** : il n'écrit dans aucune table auditée (R8) |
| VIII | Les invariants de la base ne sont pas réimplémentés | ✅ Trois contraintes identifiées, toutes destinées à la traduction | ✅ **Aucune entorse.** La validation faite en amont par le service **n'en est pas une** : elle sert à rendre l'issue que le contrat du site nomme (`missing_target`, `invalid_window`), et le refus de la base reste la barrière — les deux chemins portent la même règle et [`contracts/errors.md`](contracts/errors.md) dit lequel répond quand. **`ck_incidents_unpublish_shape` est inatteignable par les fonctions** et se traduit quand même : le dire est le seul moyen de s'apercevoir qu'elle a remonté |
| IX | Erreurs d'API : code stable, message français | ✅ Type d'erreur unique du noyau | ✅ **Trois codes ajoutés**, et pas plus. **Dix issues sortent en 200** avec leur discriminant, parce que le contrat du site les nomme et que l'écran les traduit champ par champ ; [`contracts/errors.md`](contracts/errors.md) trace la ligne. **Un identifiant hors périmètre se refuse comme un identifiant inexistant** — `Perimeter::ensure` rend 404, jamais 403, et la forme ne les distingue pas |
| X | Tests d'intégration sur base réelle | ✅ Harnais de B1 | ✅ Les quatre obligations ont chacune leur test dans chacun des deux crates, plus **neuf qui ne se déduisent d'aucune** et que le [quickstart](quickstart.md) énumère : les cinq portées qui remontent ; l'incident global visible de toute édition ; le poste de direct et son repli ; l'état temporel comparé à celui de la vue ; le compte d'outbox ; `published_by` non nul ; la chaîne récurrente qui ne s'arme qu'une fois ; l'échec d'une projection qui n'arrête pas les sept autres ; et **les neuf routes frappées sur la vraie application** |

**Verdict** : **aucune entorse au principe VIII**, comme en B6. **Aucune écriture hors schéma.** **Aucune dépendance nouvelle**, ce qui n'était arrivé à aucun jalon de la phase B. **Deux lignes de semis**, justifiées ci-dessous — et c'est tout ce que ce plan demande au modèle.

**Deux points ont été rouverts sur la spécification** plutôt que découverts à l'implémentation, parce qu'ils auraient produit une faute **à l'exécution** :

- **R10** — la permission du tableau de bord : la garder telle quelle refusait l'écran au rôle qui pilote une édition, et le défaut n'aurait été vu qu'en se connectant avec un tel compte ;
- **R26** — la page publique d'une activité **n'existe pas** : l'histoire n° 5 aurait été « livrée » avec une route qu'aucun écran n'appelle, c'est-à-dire pas livrée du tout.

**Trois autres ont été tranchés dans [`research.md`](research.md)** pour la même raison :

- **R8** — le mode concurrent depuis une fonction : mesuré, parce que le contraire journalise huit échecs **sans lever** ;
- **R9** — l'intervalle contre l'anti-rebond : une chaîne récurrente qui se dédoublonne contre elle-même s'arrête **en silence** ;
- **R12** — la ligne entre 403 et 200 : rendre 403 là où le contrat attend un statut ferait lever le client sur un message de formulaire.

---

## Project Structure

### Documentation (this feature)

```text
specs/007-direct-tableaux-de-bord/
├── plan.md              # Ce fichier
├── research.md          # Phase 0 — 29 décisions
├── data-model.md        # Phase 1 — ce qui est lu, ce qui est écrit, ce qui ne l'est pas
├── quickstart.md        # Phase 1 — comment prouver que ça marche
├── contracts/
│   ├── routes.md        # Les 9 routes, leurs gardes, leurs corps
│   ├── errors.md        # 3 codes ajoutés, 10 issues en 200, et la ligne entre les deux
│   └── events.md        # 2 événements émis par la base, 0 par le code, 0 consommateur
├── checklists/
│   └── requirements.md  # Qualité de la spécification
└── tasks.md             # Phase 2 — produit par /speckit-tasks, PAS par cette commande
```

### Source Code (repository root)

```text
backend/crates/
├── kernel/src/
│   └── config.rs                       + AnalyticsConfig { refresh_interval, refresh_debounce }
│
├── modules/live/                       ← CRÉÉ
│   ├── Cargo.toml                      kernel, contracts, actix-web, serde, sqlx, time, utoipa, uuid
│   ├── src/
│   │   ├── lib.rs                      routes() · event_routes() · ce que le crate NE sert PAS
│   │   ├── state.rs                    LiveState { db, config }
│   │   ├── domain/
│   │   │   ├── mod.rs
│   │   │   ├── incident.rs             ManagedIncident · IncidentState · IncidentWriteResult
│   │   │   ├── desk.rs                 LiveDesk · LiveDeskSession · IncidentTargets · OverrunTemplate
│   │   │   └── payload.rs              CreateIncidentPayload · UpdateIncidentPayload · UnpublishIncidentPayload
│   │   ├── repo/
│   │   │   ├── mod.rs
│   │   │   ├── incidents.rs            live.event_incidents() · publish_incident() · unpublish_incident() · INSERT/UPDATE
│   │   │   ├── active.rs               live.active_incidents_for_event()
│   │   │   ├── kinds.rs                reference.taxonomy_terms — noyau partagé, PAS cross/
│   │   │   └── cross/
│   │   │       ├── mod.rs
│   │   │       ├── event.rs            event.events (titre, fuseau, ville) · event.event_days · event.rooms
│   │   │       ├── programme.rs        programme.sessions — poste de direct, cibles, gabarit
│   │   │       └── org.rs              org.organizations — cibles
│   │   ├── service/
│   │   │   ├── mod.rs
│   │   │   ├── list.rs                 la composition de l'écran, en UNE transaction de lecture
│   │   │   ├── desk.rs                 le poste de direct et son repli
│   │   │   ├── write.rs                les quatre écritures, leur validation, leur autorisation
│   │   │   └── authz.rs                la portée visée par une écriture
│   │   └── routes/
│   │       ├── mod.rs
│   │       ├── admin.rs                7 routes · littérales AVANT paramétrées
│   │       ├── public.rs               GET /events/{id}/incidents
│   │       └── openapi.rs              LiveApi
│   └── tests/                          ~14 fichiers
│
├── modules/analytics/                  ← CRÉÉ
│   ├── Cargo.toml                      + async-trait
│   ├── src/
│   │   ├── lib.rs                      routes() · job_handlers() · jobs::refresh::planifier
│   │   ├── state.rs                    AnalyticsState { db, config }
│   │   ├── authz.rs                    PermissionSpec DashboardRead — déplacé depuis api
│   │   ├── domain/
│   │   │   ├── mod.rs
│   │   │   ├── dashboard.rs            AdminDashboard · DashboardFigures · EventIncident
│   │   │   ├── action.rs               AdminAction · AdminActionKind · AdminActionExample
│   │   │   └── figures.rs              DashboardKpi · TrendPoint · BreakdownSlice
│   │   ├── repo/
│   │   │   ├── mod.rs
│   │   │   ├── projections.rs          analytics.mv_*
│   │   │   ├── health.rs               v_operational_health · refresh_log
│   │   │   ├── settings.rs             platform.settings — noyau partagé
│   │   │   ├── reference.rs            taxonomy_terms · countries — noyau partagé
│   │   │   └── cross/
│   │   │       ├── mod.rs
│   │   │       ├── event.rs            events · calls_for_proposals · effective_deadline()
│   │   │       ├── programme.rs        v_proposal_dashboard · review_assignments · detect_conflicts() · proposals · sessions
│   │   │       ├── org.rs              duplicate_candidates · organizations
│   │   │       └── live.rs             live.active_incidents_for_event()
│   │   ├── service/
│   │   │   ├── mod.rs
│   │   │   ├── dashboard.rs            la composition, en UNE transaction de lecture
│   │   │   ├── actions.rs              les cinq familles et leur rangement
│   │   │   └── figures.rs              les six indicateurs, les courbes, les répartitions
│   │   ├── jobs/
│   │   │   ├── mod.rs
│   │   │   └── refresh.rs              RefreshAll · planifier() · prochaine_occurrence()
│   │   └── routes/
│   │       ├── mod.rs
│   │       ├── admin.rs                GET /admin/dashboard
│   │       └── openapi.rs              AnalyticsApi
│   └── tests/                          ~10 fichiers
│
├── api/src/
│   ├── lib.rs                          + montage des deux modules
│   ├── openapi.rs                      + fusion des deux documents
│   └── routes/health.rs                réemploie analytics::authz::DashboardRead
└── worker/src/main.rs                  + 1 gestionnaire, + 1 chaîne armée

docs/database/
├── 030_identity.sql                    + ('programmer', 'analytics.dashboard.read')
└── 130_analytics.sql                   + platform.settings('analytics.review_alert_days', 21)

frontend/app/
├── types/admin-incidents.ts            + OverrunTemplate
├── composables/useApi.ts               dashboard : pending → callOrNull · en-tête corrigé
├── composables/api/admin-incidents.ts  7 appels basculés · `granted` retiré · en-tête corrigé
├── composables/useMockData.ts          en-tête corrigé
└── pages/programme.vue                 + le bandeau d'incident de l'édition ouverte

CLAUDE.md                               § Périmètre actuel — corrigé
```

**Structure Decision** : deux crates de module, sur le patron exact des sept livrés. `live` porte la moitié « métier » du jalon — sept routes de back-office et une lecture publique —, `analytics` la moitié « mesure » — une route et un travail différé. Le dossier `repo/cross/` de chacun liste **exactement** ce qu'un découplage aurait à couper, ce qui rend la frontière lisible plutôt que promise.

---

## Les neuf routes, et où elles vivent

| # | Route | Crate | Garde | Rend |
|---|---|---|---|---|
| 1 | `GET /admin/incidents?event_id=` | `live` | périmètre + édition | `IncidentListScreen` |
| 2 | `GET /admin/incidents/overrun-template?session_id=` | `live` | périmètre + édition de l'activité | `OverrunTemplate` · 404 |
| 3 | `GET /admin/incidents/{id}` | `live` | périmètre + édition du message | `ManagedIncident` · 404 |
| 4 | `POST /admin/incidents` | `live` | périmètre + `live.incident.publish` sur la portée visée | `IncidentWriteResult` — **200 toujours** |
| 5 | `PUT /admin/incidents/{id}` | `live` | idem | `IncidentWriteResult` |
| 6 | `POST /admin/incidents/{id}/publish` | `live` | idem | `IncidentWriteResult` |
| 7 | `DELETE /admin/incidents/{id}/publish` | `live` | idem | `IncidentWriteResult` |
| 8 | `GET /events/{event_id}/incidents` | `live` | **aucune** | `ActiveIncident[]` |
| 9 | `GET /admin/dashboard?event_id=` | `analytics` | périmètre + `analytics.dashboard.read` sur l'édition | `AdminDashboard` · 404 |

**Deux pièges d'ordre, tous deux vérifiés et non supposés** :

- `/admin/incidents/overrun-template` et `/admin/incidents/{id}` sont **toutes deux en `GET`** : déclarée après, la littérale serait lue comme un identifiant. Le module les sépare en `chemins_litteraux` et `chemins_de_dossier`, comme `programme`, pour que la règle soit tenue par la structure.
- `/events/{id}/incidents` **ne compose aucun scope** : le module `event` déclare ses routes `/events/...` à plat, et `/events/{slug}` porte un segment là où celle-ci en porte deux. Aucune capture possible.

**Aucun `web::scope("/admin")`, aucun `web::scope("/events")`** : deux scopes du même préfixe ne se complètent pas, et c'est le défaut qui a coûté trois routes sur vingt et une en B2.

---

## Les six histoires, et l'ordre dans lequel elles se livrent

| Phase | Histoire | Ce qui devient vrai | Dépend de |
|---|---|---|---|
| 1 | — | Les deux crates existent, compilent, sont montés et rendent 404 sur tout | — |
| 2 | **US2** (P1) | L'écran des messages d'incident lit la plateforme réelle — liste, poste de direct, cibles, gabarit | 1 |
| 3 | **US3** (P1) | Un message se rédige, se publie, se corrige et se retire | 2 |
| 4 | **US1** (P1) | Le tableau de bord dit la vérité de la plateforme | 1 |
| 5 | **US4** (P2) | Les chiffres ne vieillissent plus en silence | 4 |
| 6 | **US5** (P3) | Un bandeau publié se voit du public | 3 |
| 7 | **US6** (P3) | Le dépôt cesse d'annoncer une dette qui n'existe plus | 2, 3, 4, 6 |

**US2 avant US1**, contre l'ordre de la spécification, et pour une raison de dépendance de données : le tableau de bord affiche les incidents actifs de l'édition, et un incident n'existe que si quelque chose sait en poser un. Livrer US1 d'abord ferait vérifier sa cinquième famille d'alerte sur une liste vide.

**Le jalon qui rend le jalon utile est la phase 5** : à partir de là, les deux écrans lisent la plateforme, et les chiffres restent frais. Les phases 6 et 7 se reportent sans casser quoi que ce soit — c'est pour cela qu'elles sont dernières.

---

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| **Une ligne de réglage ajoutée au modèle** (`analytics.review_alert_days`) | L'écart n° 43 est ouvert depuis le 17/08 : le seuil de vingt et un jours est écrit dans le code du site, ce que le principe I qualifie de « dette immédiate ». La composition du tableau de bord ne peut pas le lire ailleurs qu'en base sans le réintroduire côté serveur | *Une colonne sur `calls_for_proposals`* : ce n'est pas une propriété de l'appel mais un réglage d'affichage ; il faudrait le renseigner douze fois pour une valeur que personne ne veut faire varier, et il resterait vide sur les appels déjà créés — donc à replier sur une constante qu'il faudrait bien écrire quelque part. *Une constante en Rust* : déplace la dette de trois lignes, sans la régler |
| **Une attribution de permission ajoutée au modèle** (`programmer` → `analytics.dashboard.read`) | La permission existe pour cet écran et `/health` la teste depuis B1 ; l'ignorer inventerait une règle contre le modèle. Mais elle n'est pas attribuée au rôle qui pilote une édition, et le compte qui a servi à vérifier la règle métier n° 8 sur cet écran le 17/08 en serait refusé | *Garder la route sur le seul périmètre* : la permission deviendrait décorative sur le tableau de bord tout en étant testée sur `/health` — deux règles pour un même écran. *Refuser le programmateur* : il lit déjà tout ce que l'écran agrège, écran par écran ; on lui retirerait un raccourci, pas un droit |
| **Une section de configuration ajoutée au noyau** (`AnalyticsConfig`) | La période de rafraîchissement doit être réglable sans redéploiement, et refusée au démarrage si elle est plus courte que la fenêtre d'anti-rebond — sinon la chaîne s'arrête en silence (R9) | *Une constante* : la valeur juste dépend de la taille de la plateforme, et le contrôle de cohérence n'aurait nulle part où vivre. C'est exactement le patron des quatre sections déjà présentes (`event`, `media`, `engagement`, `programme`) |
| **Une expression SQL recopiée** (l'état temporel de `v_public_schedule`) | La vue écarte les activités non publiées, et le poste de direct est un écran de back-office : une activité non publiée peut parfaitement tomber en panne | *Ajouter une fonction au modèle pour partager l'expression* : deux appelants, dont une vue, et un `CASE` de cinq branches dans une fonction `STABLE` appelée par ligne coûterait plus qu'il ne rapporte. La duplication est **tenue par un test** qui compare les deux sur les cinq branches, pour une activité publiée (R22) |
