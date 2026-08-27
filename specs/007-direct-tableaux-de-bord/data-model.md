# Phase 1 — Modèle de données

**Module** : Direct + Tableaux de bord (B9) · **Date** : 2026-08-27 · [spec.md](spec.md) · [plan.md](plan.md) · [research.md](research.md)

Ce fichier ne redécrit pas le modèle : il dit **ce que les deux crates lisent, ce qu'ils écrivent, et ce qu'ils ne touchent pas**. La source de vérité reste `docs/database/080_live.sql` § 6 et `docs/database/130_analytics.sql`.

---

## 1. Ce que le modèle ajoute — deux lignes, aucune structure

| Ajout | Fichier | Ligne |
|---|---|---|
| Le seuil d'urgence du tableau de bord | `130_analytics.sql`, fin de fichier | `INSERT INTO platform.settings (key, value, description, is_secret) VALUES ('analytics.review_alert_days', '21', 'Jours avant l''échéance applicable à partir desquels un dossier sans évaluation devient une alerte du tableau de bord.', false) ON CONFLICT (key) DO NOTHING;` |
| L'accès du programmateur au tableau de bord | `030_identity.sql` § 6, bloc `role_permissions` | `('programmer', 'analytics.dashboard.read'),` |

**Ni table, ni colonne, ni type, ni fonction, ni index.** `platform.cross_module_fk_report` reste vide sans qu'on y touche.

**Le réglage est déclaré par `130_analytics.sql` et non par `900_seed.sql`**, et ce n'est pas indifférent : le fichier de semis porte une mise en garde datée — deux réglages du module média y avaient été posés avec d'autres valeurs que celles de `050_media.sql`, et comme le semis se charge **après**, son `ON CONFLICT DO NOTHING` les écartait en silence. Un module déclare ses propres réglages.

**Conséquence d'exploitation** : `down -v` avant la première compilation. Le schéma n'est chargé qu'au **premier** démarrage du conteneur ; sans destruction du volume, la base garde l'ancien semis sans le dire, et le test du seuil passerait sur la valeur de repli.

---

## 2. Le schéma `live` — ce que le crate `live` touche

### 2.1 `live.incidents` — la seule table écrite du jalon

Les colonnes que le crate écrit, toutes issues du contrat du site (`types/admin-incidents.ts`, `IncidentPayload`) :

| Colonne | Type | Ce que le crate en fait |
|---|---|---|
| `scope` | `live.incident_scope` | écrite — l'une des cinq |
| `event_id` · `event_day_id` · `session_id` · `organization_id` | `uuid` | écrites, **exactement une renseignée** selon la portée, aucune pour `global` |
| `incident_kind_code` | `text` | écrite — un code de la taxonomie `incident_kind` |
| `severity` | `live.incident_severity` | écrite |
| `title` | `platform.i18n_text` | écrite, **nulle si vide dans les deux langues** — un `{"fr":"","en":""}` serait rendu par `platform.t()` comme un titre présent et vide |
| `message` | `platform.i18n_text` | écrite, **exigée dans les deux langues par l'API** |
| `action_url` | `platform.url` | écrite |
| `is_dismissible` | `boolean` | écrite |
| `display_from` · `display_until` | `timestamptz` | écrites ; la fin peut être **nulle**, et c'est légitime |
| `published_at` · `published_by` · `unpublished_at` · `unpublished_by` · `unpublish_reason` | — | **jamais écrites directement** — voir § 2.2 |
| `created_by` | `uuid` | écrite à la création, depuis la session |
| `created_at` · `updated_at` | `timestamptz` | posées par la base et son déclencheur |

**Les trois contraintes, et qui répond quand** :

| Contrainte | Ce qu'elle interdit | Qui refuse |
|---|---|---|
| `ck_incidents_scope_target` | une cible orpheline, ou deux cibles | le service **d'abord** (issue `missing_target`, en 200) ; la base **ensuite**, traduite si un refus échappe |
| `ck_incidents_window` | une fin d'affichage antérieure au début | idem (`invalid_window`) |
| `ck_incidents_unpublish_shape` | un retrait sans publication | **inatteignable par les fonctions** — traduite quand même, parce que le dire est le seul moyen de s'apercevoir qu'elle a remonté |

**Aucune suppression.** `live.incidents` n'a pas de suppression logique et le crate n'en fait pas de physique : `DELETE /admin/incidents/{id}/publish` **dépublie**. La ligne demeure, avec son auteur, son instant et son motif.

### 2.2 Les cinq fonctions — appelées, jamais réécrites

| Fonction | Usage dans le crate |
|---|---|
| `live.event_incidents(event, at)` | **la** lecture du back-office : les cinq états, la cible résolue, l'ordre d'action. C'est aussi le **filtre de périmètre** (R7) |
| `live.active_incidents_for_event(event, at)` | la lecture publique de l'édition (route 8), et ce que `analytics` lit pour le tableau de bord |
| `live.publish_incident(id)` | publier, ou rétablir un message retiré. Horodate, attribue, **émet** |
| `live.unpublish_incident(id, motif)` | dépublier. Horodate, attribue, garde le motif, **émet** |
| `live.active_incidents(session, at)` | **non appelée dans ce jalon** — elle viendra avec la page publique d'une activité (R26) |

**`published_by` est posé par la fonction**, à partir de `platform.current_actor_id()`. Le crate ne le passe pas — mais il doit avoir posé `app.actor_id`, ce que `Db::write()` fait. Une écriture qui contournerait la porte du noyau produirait un `published_by` **nul sans erreur**, et le back-office afficherait « publié par — ». Un test le vérifie sur la valeur.

### 2.3 Ce que le crate `live` NE touche pas

`live.meetings`, `live.meeting_participants`, `live.provider_webhook_events`, `live.streams`, `live.meetings_public`, `live.current_streams`, `live.build_embed_url()`, `live.requeue_failed_participants()`, `live.replay_webhook_event()` — les quatre cinquièmes du schéma. Aucun écran ne les demande (R4).

---

## 3. Les lectures hors schéma — ce que `repo/cross/` porte

**La règle** : un module lit hors de son schéma quand la question porte sur ses propres entités, il n'y écrit jamais, et il n'appelle jamais un autre module. `platform` et `reference` sont le **noyau partagé** que le principe III exempte nommément : les y ranger ferait perdre à `cross/` son sens, qui est de lister exactement ce qu'un découplage aurait à couper.

### 3.1 `live/repo/cross/`

| Fichier | Ce qui est lu | Pour quelle question du module |
|---|---|---|
| `event.rs` | `event.events` — titre, fuseau, ville · `event.event_days` — cibles · `event.rooms` — nom de salle du poste | « dans quel fuseau se lit la fenêtre d'affichage de mon message ? », « que puis-je viser ? » |
| `programme.rs` | `programme.sessions` — poste de direct, cibles, gabarit de débordement | « que se passe-t-il aujourd'hui, sur quoi puis-je publier ? » |
| `org.rs` | `org.organizations` — organisations **animant** une activité de l'édition | « quelles organisations puis-je viser ? » — même critère que la portée `organization` du modèle |

Hors `cross/` parce que noyau partagé : `reference.taxonomy_terms` (taxonomie `incident_kind`), lu par `repo/kinds.rs`.

### 3.2 `analytics/repo/cross/`

C'est la lecture hors schéma la plus large du projet, et elle est bornée à quatre fichiers.

| Fichier | Ce qui est lu | Pour quelle question du module |
|---|---|---|
| `event.rs` | `event.events` · `event.calls_for_proposals` · `event.effective_deadline(call)` | l'édition mesurée, son appel, l'échéance qui fait foi |
| `programme.rs` | `programme.v_proposal_dashboard` · `programme.review_assignments` · `programme.detect_conflicts(event)` · `programme.proposals` · `programme.sessions` | quatre des cinq familles d'alerte, et les deux répartitions |
| `org.rs` | `org.duplicate_candidates` · `org.organizations` | la cinquième famille, avec ses trois exemples nommés |
| `live.rs` | `live.active_incidents_for_event(event, at)` | les incidents actifs de l'édition — **une fonction SQL, jamais le crate `live`** |

Hors `cross/` parce que noyau partagé : `platform.settings` (`repo/settings.rs`), `reference.taxonomy_terms` et `reference.countries` (`repo/reference.rs`).

**Aucune écriture, dans aucun des deux crates, dans aucun schéma autre que le sien** — y compris `platform` et `reference`. Un contrôle mécanique le vérifie (R29).

---

## 4. Le schéma `analytics` — ce que le crate `analytics` touche

### 4.1 Les projections lues, et lesquelles ne le sont pas

| Projection | Lue ? | Ce qu'elle donne |
|---|---|---|
| `mv_proposal_funnel` | ✅ | l'entonnoir, les dépôts, le taux d'acceptation, les séances programmées |
| `mv_daily_submissions` | ✅ | la courbe des dépôts, sa moyenne mobile, son cumul |
| `mv_daily_registrations` | ✅ | la courbe des inscriptions aux activités |
| `mv_reviewer_workload` | ✅ | les revues en retard, l'avancement du comité, les trois exemples nommés |
| `mv_daily_signups` | ❌ | compte des **créations de compte** sur toute la plateforme : ne se ventile par aucune édition (écart n° 40) |
| `mv_organization_scorecard` | ❌ | aucun écran ne l'affiche |
| `mv_session_attendance` | ❌ | idem |
| `mv_content_popularity` | ❌ | idem |

Les quatre non lues sont **rafraîchies quand même** : `refresh_all()` les porte, et les retirer de la liste serait modifier le modèle pour un gain nul.

### 4.2 Les deux vues, et l'usage borné de la première

| Vue | Usage |
|---|---|
| `v_platform_overview` | **finalement pas lue** (R18). Elle compte la plateforme entière (écart n° 44) ; le seul chiffre qu'on lui aurait pris — les doublons à arbitrer — s'obtient de `org.duplicate_candidates`, qui porte en plus les **trois exemples nommés** que la vue n'a pas |
| `v_operational_health` | ✅ lue telle quelle. **Les seuils ne sont pas recalculés** : le modèle porte déjà la décision de ce qui mérite attention. Rendue par le **code** de l'indicateur, le libellé français restant un repli (écart n° 45) |

### 4.3 `analytics.refresh_log` — écrite par la fonction, lue par la composition

| Usage | Comment |
|---|---|
| Écriture | **jamais directe** : `analytics.refresh_all()` journalise elle-même chaque vue, avec sa durée, ses lignes et son erreur |
| Lecture | `max(finished_at) WHERE succeeded` — la fraîcheur affichée par l'écran. **Le maximum sur les succès, pas la dernière ligne** : une exécution partielle laisse des lignes en échec plus récentes que le dernier succès complet (R19) |

### 4.4 `platform.jobs` — par la fonction du modèle

La mise en file passe par `analytics.enqueue_refresh(concurrently, delay, debounce)`, jamais par `kernel::jobs::enqueue`. Elle pose la file `analytics`, la tâche `analytics.refresh_all`, la priorité 200 et la clé d'anti-rebond `refresh_all:<tranche>`.

**Le piège structurel** : le conflit porte sur `(task, idempotency_key)` **quel que soit l'état du travail**, `cancelled` excepté — un travail déjà réussi bloque une nouvelle mise en file de la même tranche. Si l'intervalle était plus court que la fenêtre d'anti-rebond, **la chaîne s'arrêterait en silence** (R9). D'où le contrôle au démarrage.

---

## 5. Les formes rendues — et la seule qui manque au contrat du site

Toutes viennent de `frontend/app/types/` et **ne se renégocient pas**.

| Forme | Fichier du site | Route |
|---|---|---|
| `IncidentListScreen` | `types/admin-incidents.ts` | 1 |
| `OverrunTemplate` | **à ajouter** — voir ci-dessous | 2 |
| `ManagedIncident` | `types/admin-incidents.ts` | 3 |
| `CreateIncidentPayload` → `IncidentWriteResult` | `types/admin-incidents.ts` | 4 |
| `UpdateIncidentPayload` → `IncidentWriteResult` | `types/admin-incidents.ts` | 5, 6 |
| `UnpublishIncidentPayload` → `IncidentWriteResult` | `types/admin-incidents.ts` | 7 |
| `ActiveIncident[]` | `types/live.ts` | 8 |
| `AdminDashboard` | `types/admin-dashboard.ts` | 9 |

**`OverrunTemplate` est la seule forme à ajouter**, et c'est une conséquence mécanique du contrôle de contrat : `mocks/admin-incidents.ts` rend aujourd'hui un objet **anonyme**, et `check-api-contract` refuse toute forme annoncée par l'API sans définition dans `app/types/`. Ce n'est pas une renégociation — c'est nommer une forme qui existe déjà, avec ses cinq champs inchangés :

```ts
/** Ce que le raccourci « Signaler un débordement » a besoin de savoir. */
export interface OverrunTemplate {
  session_id: SessionId
  /** Titre résolu — l'API le rend déjà dans la langue négociée. */
  title: string
  starts_at: IsoDateTime
  ends_at: IsoDateTime
  event_id: EventId
}
```

---

## 6. Les règles qui vivent dans le service, et qui ne sont pas des invariants de base

Le principe VIII interdit de redoubler une contrainte. Ces neuf règles-là ne sont **portées par la base d'aucune manière** : les écrire dans le service n'est pas une réimplémentation.

| # | Règle | Où la base est muette |
|---|---|---|
| 1 | Le message est exigé **dans les deux langues** | `platform.i18n_text` n'exige rien de plus qu'un document non nul — **et c'est voulu**, les données reprises de la v1 n'ayant qu'une langue |
| 2 | Un titre vide dans les deux langues est écrit **nul** | la base accepterait `{"fr":"","en":""}`, que `platform.t()` rendrait comme un titre présent et vide |
| 3 | La cible visée doit appartenir à l'**édition depuis laquelle on agit** | `ck_incidents_scope_target` vérifie la cohérence portée/cible, **jamais l'appartenance à une édition** |
| 4 | Les organisations visables sont celles qui **animent** une activité de l'édition | critère de lecture de `event_incidents()`, aucune contrainte d'écriture |
| 5 | Le poste de direct montre les **quatre** prochaines activités quand le jour est vide | aucune notion de repli en base |
| 6 | Au plus **trois** exemples nommés par ligne d'action | règle d'écran, portée par le contrat du site |
| 7 | Au plus **huit** parts par répartition, la queue regroupée si elle en compte au moins deux | idem |
| 8 | La variation hebdomadaire est **nulle** sous quatorze jours de série | idem — une comparaison sur une semaine tronquée est un artefact |
| 9 | Au plus **trois** bandeaux publics, le reste replié en « +N » | la règle des pastilles de la charte, appliquée à un cas qu'elle décrit (R26) |

**Deux règles ressemblent à des réimplémentations et n'en sont pas** : la validation de la cohérence portée/cible et celle de la fenêtre d'affichage sont faites **avant** l'écriture — non pour remplacer la contrainte, mais pour rendre l'issue que le contrat du site nomme (`missing_target`, `invalid_window`) et que l'écran pose sur le bon champ. Le refus de la base reste la barrière, et [`contracts/errors.md`](contracts/errors.md) dit lequel des deux chemins répond dans quel cas.

---

## 7. Ce qui reste vrai après ce jalon, et qu'il faudra reprendre

| Point | Pourquoi il reste |
|---|---|
| `live.active_incidents(session)` n'est appelée par rien | la page publique d'une activité n'existe pas ; elle viendra avec son écran (R26) |
| Deux indicateurs de santé portent sur la visioconférence | la vue les calcule, l'écran les affiche, **aucun écran ne les règle** — leur valeur est zéro tant qu'aucune réunion n'existe (R4) |
| Un bandeau sans fin d'affichage reste le vrai danger de la table | le modèle ne l'interdit pas, à juste titre ; l'interface le signale deux fois. Le rappel automatique (« en ligne depuis 3 jours ») demande une règle de notification qui n'est pas écrite |
| Aucun consommateur d'outbox ne déclenche de rafraîchissement | l'effet serait invisible et l'écart au plus d'un quart d'heure (R28). À reprendre si l'intervalle s'allonge au-delà de l'heure |
