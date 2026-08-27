# Contrat — les neuf routes

**Module** : Direct + Tableaux de bord (B9) · **Date** : 2026-08-27 · [plan.md](../plan.md) · [research.md](../research.md)

Toutes les routes sont préfixées `/api`. Les formes de réponse sont **désignées par leur nom TypeScript** : leur source unique est `frontend/app/types/`, et `make check-api-contract` refuse une forme annoncée sans définition.

**Huit de ces routes sont déjà écrites côté site**, en attente d'API. La neuvième — la lecture publique — est nouvelle et son appelant est livré dans le même jalon (R26).

---

## Ce qui vaut pour toutes

| Règle | Détail |
|---|---|
| **Session** | Les huit routes de back-office exigent une session valide, par cookie. La lecture publique n'exige rien |
| **Périmètre** | Périmètre vide → **403**. Édition hors périmètre → **404**, jamais 403 : un identifiant hors périmètre se refuse comme un identifiant inexistant (principe IX) |
| **Langue** | `Accept-Language` négociée contre `reference.locales`, repli sur le français. Les textes multilingues sont rendus **bruts** (`platform.i18n_text`), sauf mention contraire : le site les résout |
| **Fuseau** | Toute date est rendue en `timestamptz` RFC 3339. Le **fuseau de l'édition** est rendu à part, avec la ville, pour que l'écran affiche « heure de Belém » |
| **Instant** | Chaque composition s'exécute dans **une transaction de lecture** : `now()` y est constant, et toutes les parties de la réponse parlent du même instant (R14) |

---

## 1. `GET /admin/incidents`

**Ce que l'écran demande** : tout l'écran des messages d'incident d'une édition, en une réponse.

| | |
|---|---|
| Paramètre | `event_id` — obligatoire |
| Garde | périmètre non vide, édition dans le périmètre. **Aucune permission** (R11) |
| Rend | **200** `IncidentListScreen` · **403** périmètre vide · **404** édition inconnue ou hors périmètre |

**Ce que la réponse porte, et d'où chaque part vient** :

| Part | Source |
|---|---|
| `event_id`, `event_title`, `timezone`, `zone_label` | `event.events` — `zone_label` est la **ville**, pas l'identifiant IANA |
| `rows` | `live.event_incidents(event, now())`, **dans l'ordre où elle les rend** : actifs, programmés, brouillons, historique, gravité décroissante à état égal. L'API ne réordonne pas |
| `desk` | le poste de direct — § 1.1 |
| `counts` | établis sur `rows` **avant tout filtrage**, un compte par état |
| `kinds` | `reference.taxonomy_terms` de la taxonomie `incident_kind`, actifs, dans leur `sort_order` |
| `targets` | § 1.2 |

### 1.1 Le poste de direct

`desk.day` est le **jour de l'édition** — `(now() AT TIME ZONE events.timezone)::date`, calculé en base (R21).

- `desk.sessions` porte les activités dont le début tombe ce jour-là, par `starts_at` croissant.
- Si ce jour est **vide** : les **quatre** prochaines activités par `starts_at` croissant, et `desk.is_fallback` vaut vrai. `desk.day` **reste aujourd'hui** — l'écran dit alors « aucune activité aujourd'hui, voici les prochaines ».

Chaque activité porte `session_id`, `title` (brut), `starts_at`, `ends_at`, `room_name` (brut, nul si aucune salle), `is_streamed`, `status`, `temporal_state` et `active_incident_count`.

- `temporal_state` reprend **exactement** l'expression de `programme.v_public_schedule` : annulé, reporté, à venir, en cours, passé (R22).
- `active_incident_count` compte les messages **actifs de portée `session`** visant cette activité — ce qui est déjà dit, pour ne pas publier deux fois la même panne.
- La lecture porte sur `programme.sessions` et **non** sur la vue publique : une activité non publiée peut tomber en panne.

### 1.2 Les cibles

`targets.event` — l'édition, avec son sigle en précision.
`targets.days` — les journées de l'édition, par date ; **une journée sans titre est désignée par sa date** au format `JJ/MM/AAAA`, comme le fait le modèle.
`targets.sessions` — les activités de l'édition, par début croissant ; `starts_at` est rendu **comme instant**, à part de toute précision textuelle.
`targets.organizations` — **seulement** celles qui animent au moins une activité de l'édition, par nom.

Aucune cible d'une autre édition, y compris en forgeant la requête (règle métier n° 8).

---

## 2. `GET /admin/incidents/overrun-template`

**Ce que l'écran demande** : de quoi pré-remplir le formulaire depuis le raccourci « Signaler un débordement » du planificateur, sans une saisie pendant que la salle attend.

| | |
|---|---|
| Paramètre | `session_id` — obligatoire |
| Garde | périmètre non vide, **édition de l'activité** dans le périmètre |
| Rend | **200** `OverrunTemplate` · **403** périmètre vide · **404** activité inconnue ou hors périmètre |

`OverrunTemplate` porte `session_id`, `title` (**résolu**, l'API rendant la langue négociée), `starts_at`, `ends_at`, `event_id`.

**`title` est ici résolu et non brut**, à la différence du reste : c'est une valeur de pré-remplissage de champ, pas une donnée à afficher — le site la pose telle quelle dans le formulaire.

**Le site la lit par `callOrNull`** : un 404 est une réponse, pas une panne.

**Cette route est déclarée AVANT `/admin/incidents/{id}`**, toutes deux étant en `GET` : déclarée après, elle serait lue comme un identifiant (R24).

---

## 3. `GET /admin/incidents/{id}`

**Ce que l'écran demande** : un message, pour le relire et le corriger.

| | |
|---|---|
| Garde | périmètre non vide, **édition du message** dans le périmètre |
| Rend | **200** `ManagedIncident` · **403** périmètre vide · **404** message inconnu ou hors périmètre |

**L'édition d'un message se calcule, elle ne se lit pas** : pour les portées `session`, `event_day` et `organization`, la ligne ne porte aucune colonne d'édition (R7). La route retrouve le message **par** `live.event_incidents()` sur les éditions du périmètre, ce qui rend le contrôle et la lecture indissociables.

**Un message de portée `global` est visible de toute édition administrée** — c'est voulu : une équipe qui pilote un pavillon doit savoir qu'un bandeau d'entretien le couvre.

Le site la lit par `callOrNull`.

---

## 4. `POST /admin/incidents`

**Ce que l'écran fait** : rédiger, et publier dans le même geste si c'est demandé.

| | |
|---|---|
| Corps | `CreateIncidentPayload` |
| Garde | périmètre + `live.incident.publish` **sur la portée visée** |
| Rend | **200** `IncidentWriteResult`, **toujours** · **403** périmètre vide · **404** `from_event_id` hors périmètre |

**`granted` n'existe pas.** Le site l'envoyait pour rejouer l'autorisation sur données d'exemple ; l'API lit sa propre session, et un client qui déclare ses droits n'est pas un contrôle d'accès.

**`from_event_id` reste**, et il est le seul champ du corps qui ne soit pas une colonne : c'est l'édition **depuis laquelle** on agit, et donc l'ancre du contrôle de périmètre.

**La portée visée, testée pour l'autorisation** :

| `scope` du message | Portée testée |
|---|---|
| `event` | l'édition visée |
| `event_day`, `session`, `organization` | l'édition à laquelle la cible se rattache |
| `global` | la **portée globale** (D3) |

**L'ordre des contrôles**, et il compte :

1. périmètre non vide, `from_event_id` dans le périmètre → sinon 403 / 404 ;
2. la cible visée appartient bien à cette édition → sinon `missing_target` ;
3. `live.incident.publish` sur la portée visée → sinon `forbidden` ;
4. cohérence portée/cible, message bilingue, fenêtre → sinon `missing_target`, `missing_message`, `invalid_window` ;
5. écriture, puis publication par `live.publish_incident()` si `publish` est vrai — **dans la même transaction**.

**Issue** : `created` si `publish` est faux, `published` sinon. `incident` porte la ligne de gestion complète, relue par `live.event_incidents()` pour que l'état soit celui de la base et non un état recomposé.

---

## 5. `PUT /admin/incidents/{id}`

**Ce que l'écran fait** : corriger.

| | |
|---|---|
| Corps | `UpdateIncidentPayload` |
| Garde | idem route 4, **sur la portée visée par le message tel qu'il devient** |
| Rend | **200** `IncidentWriteResult` · **403** · **404** |

**Republier efface la dépublication** — instant, auteur, motif —, exactement comme le fait `live.publish_incident()`. Le comportement n'est pas recomposé : la fonction est appelée.

**La portée peut changer.** L'autorisation est alors vérifiée sur la portée **d'arrivée** : déplacer un message d'une édition vers la portée globale exige la permission globale.

**Issue** : `updated`, ou `published` si `publish` est vrai et que le message ne l'était pas.

---

## 6. `POST /admin/incidents/{id}/publish`

**Ce que l'écran fait** : publier un brouillon depuis la ligne de liste, ou rétablir un message retiré.

| | |
|---|---|
| Corps | aucun |
| Garde | périmètre + `live.incident.publish` sur la portée du message |
| Rend | **200** `IncidentWriteResult` (`published`) · **403** · **404** |

Appelle `live.publish_incident(id)`. La fonction horodate, attribue, efface le retrait **et émet** `live.incident.published`. Le service n'émet rien (R5).

---

## 7. `DELETE /admin/incidents/{id}/publish`

**Ce que l'écran fait** : retirer un bandeau, avec un motif. **Ce n'est pas une suppression.**

| | |
|---|---|
| Corps | `UnpublishIncidentPayload` — `{ incident_id, reason }`, `reason` facultatif |
| Garde | idem route 6 |
| Rend | **200** `IncidentWriteResult` (`unpublished`) · **200** `not_published` si le message n'a jamais été publié · **403** · **404** |

**Un `DELETE` porteur d'un corps**, et c'est délibéré : le chemin est celui de la publication, le verbe dit qu'on la retire, et le motif accompagne le geste. La ligne demeure — `unpublished_at`, `unpublished_by`, `unpublish_reason` — et reparaît à l'historique de la liste.

Appelle `live.unpublish_incident(id, motif)`, qui lève `no_data_found` sur un message jamais publié ; le service la traduit en issue `not_published` plutôt que de rejouer la condition.

---

## 8. `GET /events/{event_id}/incidents`

**Ce que le public voit** : les messages actifs de l'édition affichée, sur la page des programmations.

| | |
|---|---|
| Garde | **aucune** — un bandeau d'incident est public par nature |
| Rend | **200** `ActiveIncident[]` — vide si rien n'est actif |

Sert `live.active_incidents_for_event(event, now())` : les portées `event`, `event_day`, `session`, `organization` **qui anime**, et les messages `global`. **Le plus grave en tête**, dans l'ordre où la fonction les rend.

**Chaque ligne porte `target_label` déjà résolu** par le modèle — « Atelier de négociation », « Journée finance », le nom légal d'une organisation : le bandeau nomme son sujet, et un message de portée `session` reste lisible sur une page qui parle de trente activités (R26).

**Le site en affiche au plus trois**, le reste replié en « +N ». C'est la règle des pastilles de la charte, appliquée à un cas qu'elle décrit — l'API, elle, rend tout.

**Une édition inconnue rend une liste vide, jamais 404** : cette route ne dit pas si une édition existe.

---

## 9. `GET /admin/dashboard`

**Ce que l'écran demande** : tout le tableau de bord d'une édition, en une réponse et un instant.

| | |
|---|---|
| Paramètre | `event_id` — obligatoire |
| Garde | périmètre non vide, édition dans le périmètre, **et `analytics.dashboard.read` sur cette édition** (R10) |
| Rend | **200** `AdminDashboard` · **403** périmètre vide ou permission absente · **404** édition inconnue ou hors périmètre |

**Ce que la réponse porte, et d'où chaque part vient** :

| Part | Source |
|---|---|
| `edition`, `timezone` | `event.events` |
| `call` | `event.calls_for_proposals` — zéro ou un, jamais deux |
| `actions` | les cinq familles, § 9.1 |
| `figures` | § 9.2 |
| `health` | `analytics.v_operational_health`, **seuils non recalculés**, rendue par le **code** de l'indicateur |
| `incidents` | `live.active_incidents_for_event(event, now())` |

### 9.1 Les cinq familles

**Une ligne par famille, jamais une par élément.** Une famille sans élément **n'émet aucune ligne**.

| `kind` | Ce qui la déclenche | `severity` | `target` |
|---|---|---|---|
| `proposals_unreviewed` | dossier `submitted` ou `under_review`, sans aucune revue, **et** (échéance applicable à moins de `analytics.review_alert_days` **ou** aucun révisionniste affecté, déports exclus) | `high` | `/admin/propositions?filtre=non-evaluees` |
| `reviews_overdue` | `mv_reviewer_workload.revues_en_retard > 0` | `high` | `/admin/evaluations?filtre=en-retard` |
| `active_incidents` | au moins un message actif | `high` — il est vu du public | `/admin/incidents` |
| `schedule_conflicts` | `programme.detect_conflicts(event)` non vide | `high` si un conflit `blocking`, sinon `medium` | `/admin/programmation?filtre=conflits` |
| `organization_duplicates` | `org.duplicate_candidates` non arbitrées | `medium` | `/admin/organisations/doublons` |

**L'échéance applicable n'est pas celle de l'appel** : un dossier confié porte `min(review_assignments.due_at)` sur ses affectations non déportées ; un dossier sans affectation n'a que `event.effective_deadline(call)`.

**Trois exemples nommés au plus** par ligne, chacun avec son libellé, sa précision et sa destination. **Rangement** : gravité, puis échéance la plus proche, puis décompte.

**Deux familles ne sont pas filtrées par édition** — les doublons et les messages de portée globale — et ne révèlent l'existence d'aucune autre édition.

### 9.2 Les chiffres

| Part | Source |
|---|---|
| `kpis` | six indicateurs, chacun tracé à une colonne — voir R17. **`null` n'est jamais zéro** |
| `funnel` | la ligne de `mv_proposal_funnel` de l'appel de l'édition. **Nulle** si l'édition n'a ni appel ni dépôt |
| `submissions`, `registrations` | `mv_daily_submissions` et `mv_daily_registrations`, **séries continues**, jours vides compris. Aucun trou n'est rebouché |
| `deadline`, `call_opens_at` | `event.effective_deadline(call)` — la prolongation prime — et l'ouverture |
| `by_country`, `by_theme` | huit parts au plus, la queue regroupée si elle en compte au moins deux ; libellés **multilingues bruts**, couleurs de `reference.taxonomy_terms.color_hex` |
| `refreshed_at` | `max(finished_at)` sur `analytics.refresh_log` où `succeeded`. **Nul** si aucun rafraîchissement n'a jamais abouti |

**De `v_platform_overview`, rien n'est servi** : elle compte la plateforme entière, et le seul chiffre qu'on lui aurait pris s'obtient — avec ses exemples nommés — de `org.duplicate_candidates` (R18).

---

## Ce que le site change en même temps

| Fichier | Changement |
|---|---|
| `types/admin-incidents.ts` | **+ `OverrunTemplate`** — nommer une forme aujourd'hui anonyme, sans en changer un champ |
| `composables/api/admin-incidents.ts` | 7 appels : `pending` → `call` / `callOrNull` / `send` · **paramètres rétablis** (`event_id`, `session_id`) · **corps rétablis** · **verbes rétablis** (`PUT`, `DELETE`) · **`granted` retiré des quatre écritures** · en-tête corrigé (« sept routes » → huit, et la dette levée) |
| `composables/useApi.ts` | `admin.dashboard` : `pending` → `callOrNull` · en-tête de `pending` corrigé |
| `composables/useMockData.ts` | en-tête corrigé |
| `pages/programme.vue` | **+ le bandeau d'incident** de l'édition ouverte, trois au plus, le plus grave en tête |
| `CLAUDE.md` | § Périmètre actuel — l'affirmation « trois écrans en données simulées » disparaît |

**`make check-api-contract` doit alors compter zéro route en attente**, et zéro route laissée en données d'exemple alors que l'API la sert.

**Les jeux d'exemple restent en place** : sans `NUXT_PUBLIC_API_BASE`, les deux écrans fonctionnent toujours hors ligne.
