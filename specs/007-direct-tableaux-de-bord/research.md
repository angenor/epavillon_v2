# Phase 0 — Décisions techniques

**Module** : Direct + Tableaux de bord (B9) · **Date** : 2026-08-27 · [spec.md](spec.md) · [plan.md](plan.md)

Vingt-neuf décisions. Chacune porte ce qui a été retenu, pourquoi, et ce qui a été écarté. Celles qui viennent d'une lecture du modèle citent le fichier et la section ; celles qui viennent d'un précédent livré citent le fichier de code ; celles qui viennent d'une mesure disent ce qui a été mesuré.

**Deux d'entre elles rouvrent la spécification** — R10 et R26 — et l'amendement est daté.

---

## A. Structure et frontières

### R1 — Deux crates, et ils ne se connaissent pas

**Décision** : `backend/crates/modules/live` et `backend/crates/modules/analytics`, chacun sur le patron des sept crates livrés — `domain/`, `repo/`, `routes/`, `service/`, `state.rs`, plus `jobs/` pour le second. Aucun `use` de l'un vers l'autre, aucune ligne de dépendance dans leurs `Cargo.toml`.

**Pourquoi** : le prompt l'impose, le principe II le vérifie mécaniquement, et le seul lien logique entre les deux — le tableau de bord affiche les incidents actifs — passe par une **fonction SQL** du schéma `live`, pas par le crate.

**Écarté** : *un crate « exploitation »* portant les deux schémas. Le précédent invoqué serait `programme`, qui porte `070` et `075` — mais il porte **un seul schéma en deux fichiers**. Ici il y a deux schémas, donc deux modules, donc deux crates. La règle ne souffre pas d'exception de commodité, et B6 avait déjà refusé la même.

### R2 — La composition du tableau de bord vit dans `analytics`, sous `repo/cross/`, un fichier par schéma lu

**Décision** — D1 de la spécification, rendue opérationnelle :

```
backend/crates/modules/analytics/src/repo/
├── mod.rs
├── projections.rs      analytics.mv_*            (schéma propre)
├── health.rs           analytics.v_operational_health, refresh_log
├── settings.rs         platform.settings         (noyau partagé, PAS cross/)
├── reference.rs        reference.taxonomy_terms, countries (noyau partagé)
└── cross/
    ├── mod.rs
    ├── event.rs        event.events, calls_for_proposals, effective_deadline()
    ├── programme.rs    v_proposal_dashboard, review_assignments, detect_conflicts(), proposals, sessions
    ├── org.rs          org.duplicate_candidates, organizations
    └── live.rs         live.active_incidents_for_event()
```

**`platform` et `reference` ne sont pas dans `cross/`, et ce n'est pas une facilité.** Le principe III le dit déjà pour les clés étrangères : « les schémas `platform` et `reference` forment le noyau partagé et sont exemptés — c'est la vue `platform.cross_module_fk_report` qui en décide, pas une appréciation ». La même frontière vaut pour les lectures : un module qui lit un réglage ou un terme de taxonomie ne traverse aucune frontière de module, il lit le noyau. Les ranger dans `cross/` ferait perdre au dossier son sens — il doit lister **exactement** ce qu'un découplage aurait à couper.

**Ce que `cross/` garantit** : lecture seule, aucune écriture, aucun appel de crate. Vérifiable mécaniquement — voir R29.

**Écarté** : *la composition dans `api`*. Ce n'est pas un module : il ne s'extrait pas, il n'a pas de schéma, et la frontière n'y a plus de prise. *La composition dans `programme`* : trois des cinq familles ne lui appartiennent pas. *Cinq lectures séparées* : neuf allers-retours et neuf instants de mesure, ce que le contrat du site interdit explicitement.

### R3 — `/ready` et `/health` restent dans `api` ; seule la déclaration de permission déménage

**Décision** : `backend/crates/api/src/routes/health.rs` n'est pas déplacé. Le `PermissionSpec` qu'il déclare aujourd'hui « faute de crate » (`DashboardRead`, code `analytics.dashboard.read`) est **déplacé dans le crate `analytics`** et réemployé par `api`, qui dépend déjà des modules.

**Pourquoi ne pas déplacer la route** : `/health` fait paire avec `/ready`, et les deux sont des routes d'**exploitation**, pas des routes de module. Montée derrière `is_mounted("analytics")`, elle disparaîtrait le jour où le module serait éteint — c'est-à-dire exactement le jour où l'on veut savoir ce qui se passe. Un orchestrateur qui perd sa sonde parce qu'un drapeau a bougé est un incident, pas une conséquence acceptable.

**Ce que le déménagement de la permission règle** : le commentaire d'excuse (« la permission appartient au module `analytics`, qui n'a pas de crate dans ce jalon ») cesse d'être vrai, et le code de permission est déclaré **une fois**, à côté des deux routes qui le testent.

### R4 — Le crate `live` ne sert que les messages d'incident

**Décision** : réunions de visioconférence, participants et leur synchronisation, journal des webhooks fournisseur, diffusions — les quatre cinquièmes de `080_live.sql` — restent hors périmètre. Le crate est créé au nom de son schéma sans le couvrir, et son `lib.rs` le dit en tête.

**Pourquoi** : ni écran, ni contrat du site, ni prompt ne les demande. Les livrer « parce qu'elles existent » produirait des routes sans appelant, que la vérification du contrat listerait indéfiniment.

**Conséquence à ne pas oublier** : deux indicateurs de `v_operational_health` portent sur la visioconférence (`visio_reunions_desynchronisees`, `visio_inscriptions_desynchronisees`). Ils continuent d'être **rendus** par le tableau de bord — la vue les calcule, on les affiche —, ils ne sont simplement **actionnables par aucun écran** de ce jalon. C'est un état de fait, pas un défaut : leur valeur est zéro tant qu'aucune réunion n'est créée.

---

## B. Ce que la base fait déjà, et que le code ne refait pas

### R5 — Le service n'émet aucun événement : les deux fonctions de publication le font

**Vérifié dans le corps des fonctions** (`080_live.sql` § 6), pas déduit d'un précédent :

| Ce qui est appelé | Ce que la base fait alors, seule |
|---|---|
| `live.publish_incident(id)` | pose `published_at`, `published_by` (par `platform.current_actor_id()`), efface la dépublication, **et émet** `live.incident.published` |
| `live.unpublish_incident(id, motif)` | pose `unpublished_at`, `unpublished_by`, `unpublish_reason`, **et émet** `live.incident.resolved` |

**Décision** : ni le service ni le repo n'appellent `platform.emit_event()`. C'est le piège n° 1 des six modules précédents — B3, B4 et B5 l'ont chacun rencontré —, et il est nommé ici avant d'être commis.

**Corollaire** : `published_by` est posé par la **fonction**, à partir du contexte de transaction. Le service n'a donc rien à passer — mais il doit avoir posé `app.actor_id`, ce que `Db::write()` fait déjà (principe VII). Une écriture qui oublierait la porte du noyau produirait un `published_by` nul, sans erreur.

### R6 — L'état d'un incident n'est jamais recomposé

**Décision** : `state` (`active`, `scheduled`, `draft`, `expired`, `unpublished`) est lu tel que `live.event_incidents()` le rend. Aucune expression équivalente en Rust, aucune dans le site — il le lit déjà.

**Pourquoi** : quatre conditions cumulées, que la v1 oubliait une par une, d'où ses bandeaux restés en ligne des mois. Le commentaire du modèle est explicite : « l'état est calculé ici, et nulle part ailleurs ».

### R7 — Le filtre de périmètre passe par la fonction, jamais par un `WHERE event_id`

**Décision** : toute lecture d'incidents d'une édition passe par `live.event_incidents(event, at)` ou par sa part active. Aucune requête du crate ne compare `live.incidents.event_id` à une édition.

**Pourquoi, vérifié sur la table** : `live.incidents` n'a **aucune colonne d'édition** pour les portées `session`, `event_day` et `organization` — le rattachement est un calcul, et la portée `organization` en particulier dépend de ce que l'organisation **anime effectivement** dans l'édition. Un filtre écrit à la main laisserait fuir trois portées sur cinq.

**Conséquence sur la forme des requêtes** : la liste du back-office **filtre et compte après** l'appel de fonction, pas avant. Les compteurs par état sont établis sur le résultat complet — « avant filtrage, comme partout », dit le contrat du site.

### R8 — `refresh_all(true)` fonctionne depuis une fonction ET dans un bloc transactionnel — mesuré, pas supposé

**Le doute était sérieux** : `REFRESH MATERIALIZED VIEW CONCURRENTLY` est réputé refusé à l'intérieur d'une fonction et d'un bloc transactionnel. Si c'était le cas, `analytics.refresh_all(true)` journaliserait **huit échecs** sans lever — son `EXCEPTION WHEN OTHERS` les avale vue par vue — et le tableau de bord vieillirait en silence pendant que le worker croirait travailler. C'est exactement le genre de défaut qui coûte une journée en implémentation.

**Mesuré sur la base du dépôt, PostgreSQL 17** :

```
SELECT vue, succes FROM analytics.refresh_all(true);           →  8 vues, 8 succès
BEGIN; SELECT vue, succes FROM analytics.refresh_all(true); COMMIT;  →  8 vues, 8 succès
```

**Décision** : le gestionnaire appelle `analytics.refresh_all(true)`. Le mode non concurrent reste réservé au premier peuplement, comme le dit le modèle.

**Mais il l'appelle sur le pool, pas dans une transaction d'écriture du noyau.** La fonction écrit son propre journal, n'a besoin d'aucun `app.actor_id` — aucune table auditée n'est touchée —, et l'envelopper dans `Db::write()` tiendrait une connexion d'écriture et ses verrous pendant toute la durée du rafraîchissement pour n'y rien gagner.

### R9 — La mise en file passe par `analytics.enqueue_refresh()`, et l'intervalle doit dépasser la fenêtre d'anti-rebond

**Décision** : la chaîne récurrente ne construit pas son travail avec `kernel::jobs::enqueue` — elle appelle `analytics.enqueue_refresh(true, intervalle, anti_rebond)`, la fonction du modèle, qui pose la file « analytics », la tâche `analytics.refresh_all`, la priorité basse (200) et la clé d'anti-rebond.

**Le piège, et il est structurel.** La clé d'unicité est `refresh_all:<tranche>`, où la tranche est calculée sur `clock_timestamp()` **au moment de l'insertion**, arrondie au pas d'anti-rebond (300 s par défaut). Et le conflit de `platform.jobs` porte sur `(task, idempotency_key)` **quel que soit l'état du travail, `cancelled` excepté** — un travail déjà réussi bloque donc une nouvelle mise en file de la même tranche.

Conséquence : **si l'intervalle était plus court que la fenêtre d'anti-rebond, la chaîne se dédoublonnerait contre elle-même et s'arrêterait**, sans erreur et sans trace. C'est le pire des silences.

**Décision chiffrée** : intervalle par défaut **15 minutes**, anti-rebond **5 minutes** — un rapport de trois, qui laisse de la marge. `ANALYTICS_REFRESH_INTERVAL` est refusée au démarrage si elle est nulle ou inférieure à la fenêtre d'anti-rebond, comme `EVENT_CALL_AUTOCLOSE_INTERVAL` l'est déjà pour zéro.

**Le plafond vient du modèle, pas d'un goût** : l'indicateur `analytique_perimee` de `v_operational_health` passe en `attention` à **120 minutes** et en `critique` à **1440**. Un intervalle de 15 minutes laisse la marge d'un rattrapage sans allumer l'alerte ; un intervalle de deux heures l'allumerait à chaque cycle.

---

## C. Autorisation

### R10 — Le tableau de bord se garde par `analytics.dashboard.read`, et le rôle `programmer` la reçoit

**Ce que le plan a trouvé, et que la spécification ne pouvait pas voir.** FR-018 dit « le tableau de bord DOIT être refusé à qui n'administre aucune édition ». Or le modèle porte une permission faite pour cet écran — `analytics.dashboard.read` (`030_identity.sql` ligne 586) — et `GET /api/health` la teste **déjà**, depuis B1.

**Et elle n'est pas attribuée au rôle qui en a le plus besoin.** Relevé dans le catalogue :

| Rôle | `analytics.dashboard.read` | Périmètre d'administration |
|---|---|---|
| `super_admin` | oui (trigger) | global |
| `admin` | **oui** | oui |
| `reviewer` | **oui** | oui |
| `programmer` | **non** | **oui** |

`identity.administered_events()` est adossée à `programme.proposal.read_all`, que `programmer` détient : un programmateur détaché **a** un périmètre d'administration, et c'est avec un tel compte — Fatou Nko Diop, `programmer` sur la seule COP31 — que la règle métier n° 8 a été **vérifiée au navigateur sur cet écran** le 17/08. Garder la permission telle quelle refuserait le tableau de bord au compte qui a servi à le valider.

**Décision** : garder la route par `analytics.dashboard.read`, testée **sur l'édition demandée**, et **ajouter une ligne** au catalogue des rôles :

```sql
('programmer', 'analytics.dashboard.read'),
```

**Pourquoi ce sens plutôt que d'ignorer la permission** : l'ignorer inventerait une règle contre le modèle, ce que le principe I interdit. Et la ligne **n'accorde aucune élévation** : un programmateur lit déjà, écran par écran et pour sa seule édition, tout ce que le tableau de bord agrège — les dossiers par `programme.proposal.read_all`, la programmation par `programme.session.schedule`, les incidents par `live.incident.publish`. Le tableau de bord lui fait gagner cinq écrans, pas un droit. C'est le raisonnement que le modèle a lui-même écrit en accordant `identity.role.assign` à `admin`.

**La portée reste contraignante** : `has_permission(p, 'analytics.dashboard.read', 'event', édition)` n'accepte qu'une attribution globale ou une attribution sur **cette** édition.

**Amendement de la spécification, daté du 27/08** : FR-018 se lit désormais « le tableau de bord DOIT exiger `analytics.dashboard.read` sur l'édition demandée, et être refusé à qui n'administre aucune édition ». Les deux conditions se cumulent — la permission ouvre l'écran, le périmètre borne ce qu'on y voit.

### R11 — Lire les incidents n'exige aucune permission ; écrire exige `live.incident.publish` sur la portée visée

**Décision**, reprise telle quelle de la spécification et vérifiée sur le catalogue :

| Route | Garde |
|---|---|
| Les quatre lectures du back-office | `Perimeter` non vide + l'édition dans le périmètre. **Aucune permission.** |
| Les quatre écritures | idem, **plus** `live.incident.publish` sur la **portée visée** |
| La lecture publique | aucune garde — un bandeau est public par nature |

**Aucune permission de lecture n'est ajoutée** : le catalogue ne porte que `live.incident.publish`, et une permission qui protégerait un texte déjà affiché au public ne protégerait rien.

**La portée visée, concrètement** :

| Portée du message | Portée testée |
|---|---|
| `event`, `event_day`, `session`, `organization` | l'**édition** à laquelle la cible se rattache |
| `global` | la portée **globale** — D3 |

`identity.has_permission()` accepte une attribution `global` pour n'importe quelle portée demandée et une attribution `event` pour sa seule édition : tester la portée globale exclut donc un compte détaché **sans une ligne de code de plus**. Vérifié dans le corps de la fonction (`030_identity.sql` lignes 344-347).

### R12 — La ligne de partage entre un refus HTTP et un refus en 200

**Le contrat du site range dix issues sous un seul discriminant**, `forbidden` et `not_found` compris, et l'écran les traduit par `admin.incident.form.error.<statut>` (`pages/admin/incidents/nouveau.vue`, ligne 148). Répondre 403 à ces deux-là ferait lever le client là où il attend un message de formulaire.

**Décision** :

| Cas | Réponse |
|---|---|
| Périmètre d'administration **vide** | **403**, jamais un corps de contrat |
| Édition **hors périmètre** | **404** — un identifiant hors périmètre se refuse comme un identifiant inexistant (principe IX, `Perimeter::ensure`) |
| `live.incident.publish` absente sur la portée visée | **200** + `{ status: 'forbidden', incident: null }` |
| Incident introuvable, sur une **écriture** | **200** + `{ status: 'not_found', incident: null }` |
| Incident introuvable, sur `GET /admin/incidents/{id}` | **404** — le site le lit par `callOrNull`, pour qui un 404 est une réponse |
| Cible manquante, message unilingue, fenêtre invalide, message jamais publié | **200** + le statut correspondant |

**La règle qui les sépare** : le **périmètre** est un contrôle d'accès qui ne figure pas au contrat du site et ne doit rien révéler ; tout le reste est une **issue prévue par le contrat**, qui s'affiche dans le formulaire. C'est la même ligne que celle tenue par la vitrine (B8) et par les événements (B3).

### R13 — Aucune addition au noyau pour l'autorisation

**Décision** : `Perimeter`, `Actor`, `has_permission`, `require_permission`, `require_permission_anywhere`, `PermissionSpec`, `Requires`, `RequiresAnyScope` suffisent — tous livrés depuis B1 et B2.

Le seul ajout au noyau de ce jalon est **une section de configuration** (R27), pas un mécanisme.

---

## D. La composition du tableau de bord

### R14 — Une transaction de lecture, un instant — et `now()` est déjà figé par la transaction

**Décision** : les lectures qui composent `GET /admin/dashboard` s'exécutent dans **une seule transaction en lecture seule**, ouverte en `REPEATABLE READ` sur le pool :

```
BEGIN;  SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY;
… une dizaine de lectures …
COMMIT;
```

**Deux propriétés, et la première est gratuite.** `now()` vaut `transaction_timestamp()` : il est **constant pour toute la transaction**. Les fonctions dont le second paramètre est `DEFAULT now()` — `live.event_incidents()`, `live.active_incidents_for_event()` — parlent donc du même instant que `v_operational_health`, que `detect_conflicts()` et que le seuil d'urgence, sans qu'on ait à passer un horodatage de main en main. La seconde propriété demande l'isolation : les projections et les tables vives sont lues sur **un seul instantané**, si bien que l'entonnoir et la liste des dossiers ne peuvent pas se contredire.

**Pourquoi pas `Db::write()`** : il prend une connexion d'écriture et pose un contexte d'audit pour une requête qui n'écrit rien. `pool.begin()` suffit, et l'absence de contexte d'acteur est ici **correcte** : rien n'est audité.

**Le même patron vaut pour `GET /admin/incidents`**, dont la composition lit six choses.

### R15 — Les cinq familles d'alerte, leur source et leur critère

Relevés sur `mocks/admin-dashboard.ts`, qui vaut spécification exécutable, et retracés à la colonne du modèle :

| Famille | Source | Ce qui fait entrer une ligne | Gravité |
|---|---|---|---|
| `proposals_unreviewed` | `v_proposal_dashboard` + `review_assignments` | statut `submitted` ou `under_review`, `review_count = 0`, **et** (échéance applicable à moins de N jours **ou** aucun révisionniste affecté, déports exclus) | `high` |
| `reviews_overdue` | `mv_reviewer_workload` | `revues_en_retard > 0` | `high` |
| `active_incidents` | `live.active_incidents_for_event()` | au moins un actif | `high` — un incident actif est vu du public |
| `schedule_conflicts` | `programme.detect_conflicts(event)` | au moins un chevauchement détecté | `high` si un `blocking`, sinon `medium` |
| `organization_duplicates` | `org.duplicate_candidates` | `reviewed_at IS NULL` | `medium` |

**L'échéance applicable n'est pas celle de l'appel.** Un dossier confié porte **sa** date : `min(review_assignments.due_at)` sur les affectations non déportées ; un dossier sans affectation n'a que `event.effective_deadline(call)` pour horizon. Le distinguo vient du jeu d'exemple et il est structurant — ne regarder que la clôture ferait découvrir un dossier vierge le jour où plus personne n'a le temps.

**Trois exemples au plus par ligne**, chacun avec son libellé, sa précision et sa destination ; les lignes rangées par gravité, puis par échéance la plus proche, puis par décompte. **Une famille sans élément n'émet pas de ligne.**

**Les doublons et les incidents de portée globale ne sont pas filtrés par édition**, et ne révèlent l'existence d'aucune autre : une paire d'organisations n'appartient à aucune COP, un bandeau global les couvre toutes.

### R16 — Le seuil d'urgence : un réglage, déclaré par le fichier du module qui le lit

**Décision** — D2 rendue opérationnelle. Clé `analytics.review_alert_days`, valeur `21`, **déclarée dans `130_analytics.sql`** et non dans `900_seed.sql`.

**Pourquoi ce fichier-là** : `900_seed.sql` porte une mise en garde explicite — deux réglages du module média y avaient été semés avec d'autres valeurs que celles de `050_media.sql`, et comme le semis se charge **après**, son `ON CONFLICT DO NOTHING` les écartait en silence. « Le module média déclare ses propres réglages. » Le module analytique fait de même.

**Lecture** : une requête, repli sur 21 si la clé a été supprimée. Le repli n'est pas une seconde source de vérité — c'est la garantie que l'écran reste juste si quelqu'un vide la table.

### R17 — Les six indicateurs de tête, colonne par colonne

Aucun n'est calculé à l'écran, et aucun n'est inventé ici :

| Indicateur | Valeur | Second membre | Variation | Étincelle |
|---|---|---|---|---|
| `submissions` | `mv_proposal_funnel.deposees` | — | 7 j face aux 7 précédents, sur `mv_daily_submissions` | 21 derniers jours |
| `deadline` | jours entiers jusqu'à `effective_deadline()` | durée de la fenêtre de dépôt | — | — |
| `review_progress` | somme de `revues_soumises` | somme de `propositions_assignees` | — | — |
| `acceptance_rate` | `taux_acceptation` | `decidees` | — | — |
| `scheduled` | `sessions_programmees` | **aucun** | — | — |
| `registrations` | dernier `inscriptions_cumulees` | — | idem, sur `mv_daily_registrations` | 21 derniers jours |

**`null` n'est pas zéro**, et c'est la distinction qui coûte le plus cher : un taux d'acceptation nul dit qu'aucun dossier n'a été tranché ; affiché « 0 % », il ferait passer un comité qui n'a pas commencé pour un comité qui a tout refusé. Même règle pour la variation, nulle quand la série compte moins de quatorze jours.

**`scheduled` n'a délibérément pas de dénominateur** : `sessions_programmees` compte les séances issues d'un dossier de l'appel **quel que soit le sort du dossier**, si bien que les deux ensembles ne sont pas emboîtés — le jeu d'exemple le montre, 18 séances pour 16 dossiers retenus. Écrire « 18 sur 16 » affirmerait un rapport qui n'existe pas.

### R18 — `v_platform_overview` n'est lue que pour ce qui n'appartient à aucune édition

**Décision** : de cette vue, la composition ne prend que `doublons_a_arbitrer`. Tout le reste vient des projections par événement.

**Pourquoi** : elle compte la plateforme entière (écart n° 44) ; en servir davantage sur un écran dont le sujet est une édition ferait dire à un chiffre ce qu'il ne dit pas.

**Et même ce chiffre-là est lu autrement** : la famille « doublons » a besoin de **trois exemples nommés**, que la vue ne porte pas. Elle est donc lue directement dans `org.duplicate_candidates`, jointe à `org.organizations`, triée par score décroissant — et le décompte en sort du même coup. La vue n'est finalement **pas lue du tout**, et c'est plus honnête que de la lire pour un nombre qu'on obtient déjà.

### R19 — `refreshed_at` vient du journal, pas d'une horloge

**Décision** : `max(finished_at)` sur `analytics.refresh_log` où `succeeded`. Nul quand aucun rafraîchissement n'a jamais abouti.

**Pourquoi le maximum sur les succès et non la dernière ligne** : une exécution partielle laisse des lignes en échec plus récentes que le dernier succès complet. Prendre la dernière ligne ferait annoncer une fraîcheur qu'aucune projection n'a.

### R20 — Les répartitions : huit parts et une queue

**Décision** : dossiers déposés par **pays de l'organisation porteuse** et par **thématique**, huit parts au plus, la queue regroupée sous un libellé multilingue composé — et **seulement si elle compte au moins deux éléments**, « 1 autres » étant une faute et la neuvième barre coûtant moins que la ligne qui la masque.

**Le pays est celui de l'organisation, pas de la personne** : une chargée de projet basée à Paris qui dépose pour une ONG sénégalaise dépose un dossier sénégalais. C'est déjà la règle de `v_public_schedule` et de la liste du back-office.

**Les couleurs viennent de `reference.taxonomy_terms.color_hex`**, jamais d'un jeton de style. Les figer dans la feuille de style est le défaut n° 1 de la v1.

---

## E. Le poste de direct

### R21 — Le jour est celui de l'édition, et il est calculé en base

**Décision** : le jour du poste est `(now() AT TIME ZONE events.timezone)::date`, calculé dans la requête. Les activités retenues sont celles dont `(starts_at AT TIME ZONE events.timezone)::date` vaut ce jour.

**Pourquoi en base** : le fuseau est une colonne de l'édition, la conversion est une opération que PostgreSQL fait exactement, et la refaire en Rust ferait deux implémentations de la même arithmétique. À Belém il est 06:00 quand il est 11:00 à Paris ; une équipe qui pilote depuis Québec ne doit pas voir la journée de la veille.

**Le repli** : si le jour est vide, les **quatre** prochaines activités par `starts_at` croissant, et `is_fallback` à vrai. Le jour rendu reste **celui d'aujourd'hui** — c'est ce que le site affiche, et l'écran dit alors « aucune activité aujourd'hui, voici les prochaines ».

**La lecture porte sur `programme.sessions`, pas sur `v_public_schedule`** : le poste est un écran de back-office, et une activité non publiée peut parfaitement tomber en panne. La vue filtre `published_at IS NOT NULL`.

### R22 — L'état temporel reprend l'expression de la programmation publique, et un test le tient

**Décision** : `temporal_state` est recomposé dans la requête du poste avec **exactement** l'expression de `v_public_schedule` — annulé, reporté, à venir, en cours, passé, dans cet ordre de décision.

**C'est une duplication assumée**, et elle est nommée : la vue ne peut pas servir, puisqu'elle écarte les activités non publiées. Le risque est que les deux expressions divergent un jour, en silence.

**Ce qui le tient** : un test compare, pour une activité **publiée**, l'état rendu par le poste et celui rendu par `v_public_schedule`, sur les cinq branches. Une divergence casse la construction plutôt que d'apparaître à l'écran.

**Écarté** : *ajouter une fonction au modèle* pour partager l'expression. Elle aurait deux appelants et l'un des deux est une vue — un `CASE` de cinq branches dans une fonction `STABLE` appelée par ligne coûterait plus qu'il ne rapporte, et le modèle n'a pas à bouger pour un test qu'on peut écrire.

### R23 — Les cibles offertes sont bornées à l'édition, et les organisations à celles qui y animent

**Décision** : journées de l'édition, activités de l'édition, et organisations **portant au moins une activité** de l'édition — le même critère que la portée `organization` de `live.event_incidents()`.

**Pourquoi ce critère précis** : en proposer d'autres offrirait une portée qui ne s'afficherait nulle part. Une ONG en panne sur une autre COP n'a rien à faire dans cette liste — c'est le commentaire du modèle, repris tel quel.

**Une journée sans titre est désignée par sa date**, comme le fait déjà `live.event_incidents()` : `to_char(day_date, 'DD/MM/YYYY')`. Le format est celui de la base, et il ne se recompose pas côté site.

**Une cible d'activité porte son début comme instant**, à part de toute précision textuelle : le contrat du site sépare `hint` (texte) de `starts_at` (instant) précisément parce que les mélanger avait fait apparaître un `2027-11-13T09:30:00-03:00` brut dans une liste déroulante.

---

## F. Routes et montage

### R24 — Neuf routes, toutes plates, et l'ordre littéral avant paramétré

**Décision** : aucun `web::scope("/admin")`, aucun `web::scope("/events")`. Des routes plates, comme la vitrine et pour la même raison — deux scopes du même préfixe ne se complètent pas, et Actix rend 404 sur les routes du second. C'est le défaut qui a coûté trois routes sur vingt et une en B2.

| # | Route | Crate |
|---|---|---|
| 1 | `GET /admin/incidents` | `live` |
| 2 | `GET /admin/incidents/overrun-template` | `live` |
| 3 | `GET /admin/incidents/{id}` | `live` |
| 4 | `POST /admin/incidents` | `live` |
| 5 | `PUT /admin/incidents/{id}` | `live` |
| 6 | `POST /admin/incidents/{id}/publish` | `live` |
| 7 | `DELETE /admin/incidents/{id}/publish` | `live` |
| 8 | `GET /events/{event_id}/incidents` | `live` |
| 9 | `GET /admin/dashboard` | `analytics` |

**Le risque de capture est réel ici, et sur une seule paire** : `/admin/incidents/overrun-template` et `/admin/incidents/{id}` sont **toutes deux en `GET`**. Déclarée après, la littérale serait lue comme un identifiant et rendrait « incident introuvable » sur un chemin qui existe. Le module les sépare comme `programme` le fait : `chemins_litteraux` avant `chemins_de_dossier`, pour que la règle soit tenue par la structure et non par la vigilance.

**Les deux verbes inhabituels sont voulus et viennent du contrat du site** : `PUT` pour la correction, `DELETE` pour la dépublication — qui n'est **pas** une suppression. La ligne demeure ; c'est le chemin qui est celui de la publication.

### R25 — `/events/{id}/incidents` ne compose aucun scope

**Vérifié dans le code** : le module `event` déclare ses routes `/events/...` **à plat** (`routes/public.rs`, lignes 24-34), sans scope. `live` peut donc y ajouter la sienne sans que rien ne se compose côté API.

**Aucune capture possible** : `/events/{slug}` porte un segment après `/events`, `/events/{id}/incidents` en porte deux. Les motifs ne se recouvrent pas. Le point est vérifié plutôt que supposé, `{slug}` étant justement le genre de motif qui capture.

**Écarté** : *`GET /sessions/{id}/incidents`*, qui aurait servi `live.active_incidents(session)`. Elle est juste, mais **aucun écran ne l'appellerait** (R26) : la livrer produirait une route sans appelant. Elle viendra avec la page publique d'une activité, qui viendra avec son écran.

---

## G. L'exposition publique

### R26 — La page publique d'une activité n'existe pas : l'exposition se fait sur les programmations, à l'échelle de l'édition

**Ce que le plan a trouvé, et que la spécification ne pouvait pas voir.** D4 et FR-057 à FR-060 placent le bandeau « sur la page publique de cette activité ». **Cette page n'existe pas.** Relevé :

- `frontend/app/pages/` ne porte aucune page d'activité — la programmation publique est `pages/programme.vue`, une grille et un calendrier ;
- `useApi.ts` le dit lui-même, à propos du détail public d'une séance : « **Aucun écran ne l'appelle encore : la page publique d'une séance n'est pas au jalon**, et le type de son écran se déclarera avec elle » ;
- `UiIncidentBanner` n'est monté que dans l'aperçu du formulaire et le guide de style — ce qui était le constat de départ, et dont on comprend maintenant la cause.

**Décision, et amendement daté de la spécification (27/08)** : l'exposition publique se fait sur **la page des programmations**, à l'échelle de **l'édition affichée**, par `GET /events/{event_id}/incidents` — donc par la fonction **descendante** `live.active_incidents_for_event()`, et non par la fonction montante.

FR-057 se lit désormais : « L'API DOIT servir une lecture publique des messages actifs d'une **édition**, par la fonction descendante du modèle. » FR-059 : « Le site DOIT afficher le bandeau **sur la page des programmations**, pour l'édition ouverte. »

**Ce que cela change pour le cas d'usage du commanditaire** — « signaler un dysfonctionnement lié à la diffusion d'une activité en direct », portée `session` : le message **s'affiche**, et il **nomme son activité**. `live.active_incidents_for_event()` rend `target_label` déjà résolu — « Atelier de négociation », « Journée finance », le nom légal de l'organisation. Un bandeau qui dit « Atelier de négociation — diffusion interrompue » sur la page du programme est exactement l'information attendue, et elle atteint le public le jour où elle sert.

**La question laissée ouverte par D4 se tranche donc, et par une règle qui existe déjà** : au plus **trois** bandeaux, le plus grave en tête, le reste replié en « +N ». C'est mot pour mot la règle des pastilles thématiques de la charte — « trois au plus sur une carte, les suivantes se replient en +N ; au-delà, elles cessent d'informer ». Aucune règle nouvelle n'est inventée ; une règle existante est appliquée à un cas qu'elle décrit.

**Ce que le jalon ne prend toujours pas** : le bandeau sur l'accueil, et la page publique d'une activité. Toutes deux appartiennent au jalon suivant, avec leurs écrans.

---

## H. Le worker

### R27 — Une chaîne récurrente, sur le patron des six existantes

**Décision** : `analytics::job_handlers(db, &config)` rend **un** gestionnaire, `RefreshAll`, de tâche `analytics.refresh_all` et de file `analytics`. `analytics::jobs::refresh::planifier(&mut tx, …)` est appelée par `armer_les_recurrents` au démarrage, comme les six chaînes déjà en place.

**Ce qui change dans le worker**, et rien d'autre : deux lignes dans `main.rs` — l'enregistrement du gestionnaire et l'armement. La file « analytics » devient écoutée **du seul fait** que le gestionnaire la déclare, `JobRegistry::queues()` étant construite ainsi. Le commentaire de `jobs.rs` qui la nomme déjà cesse d'être une promesse.

**Le travail se replanifie lui-même**, comme les six autres : rien dans le noyau ne porte de récurrence, et une boucle de plus dans le worker serait un second ordonnanceur à surveiller.

**Un point d'attention repris de `PurgeAssets`** : la replanification est faite **après** le rafraîchissement, dans la même exécution. Si le rafraîchissement échoue et que le gestionnaire rend une erreur, `platform.fail_job()` rejoue le **même** travail avec un délai croissant — la chaîne n'est donc pas rompue. Mais si les tentatives s'épuisent, elle l'est : c'est le rôle de l'armement au démarrage, et c'est pourquoi il existe.

### R28 — Aucun consommateur d'outbox, et c'est délibéré

**Décision** : ce jalon ne branche **aucun** consommateur d'événements de domaine sur `analytics.enqueue_refresh()`, bien que le commentaire du modèle le prévoie (« appelée par un abonné de l'outbox — après une décision de comité, une fin de session… »).

**Pourquoi** : l'effet d'un tel consommateur serait **invisible** — les chiffres se rafraîchiraient un peu plus tôt. Un mécanisme dont on ne peut pas voir qu'il fonctionne est un mécanisme qu'on ne peut pas éprouver, et l'anti-rebond du modèle rend l'écart au plus égal à l'intervalle, soit un quart d'heure.

**Ce qui rendrait la décision fausse**, et qu'il faudra alors reprendre : un intervalle allongé au-delà de l'heure, ou une demande explicite de fraîcheur immédiate après une décision de comité. Le point est inscrit au fichier du module.

---

## I. Tests

### R29 — Base réelle et jetable, et deux contrôles mécaniques

**Décision** : le harnais `kernel::testing` de B1, sur une base chargée depuis `docs/database/` dans l'ordre. **Aucun mock de base.** Chaque test pose ses données — le semis ne fournit aucun incident.

Chaque crate livre les quatre minimums du principe X : chemin nominal de chaque route, refus par périmètre **URL forgée comprise**, traduction d'au moins un invariant de la base, et écriture des événements attendus dans l'outbox.

**Deux contrôles mécaniques s'ajoutent**, et ils vérifient ce qu'une relecture laisse passer :

1. **Aucune écriture hors du schéma du module.** Un test parcourt les fichiers de `src/` de chaque crate et refuse toute occurrence de `INSERT INTO`, `UPDATE` ou `DELETE FROM` visant un schéma autre que le sien — `platform` et `reference` compris, qu'aucun des deux n'a à écrire.
2. **Aucune arête entre modules.** `cargo tree -p live` et `cargo tree -p analytics` ne portent aucun crate de `modules/`. Déjà couvert par `make check`, redit ici parce que c'est la vérification qui rend la frontière réelle.
