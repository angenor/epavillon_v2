# B9 — Direct + Tableaux de bord

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de ce module est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅ **LIVRÉ le 27/08** — les 146 tâches, deux crates, neuf routes, `make check-api-contract` à **zéro route en attente**. [`specs/007-direct-tableaux-de-bord/`](../../../specs/007-direct-tableaux-de-bord/spec.md)

---

## Ce qui a été livré

**27/08 — le module.** Les 146 tâches. **Deux crates** — `backend/crates/modules/live` et
`backend/crates/modules/analytics` —, **9 routes**, **1 travail différé**, **0 événement émis par le
code**, **3 codes d'erreur**, **2 lignes de semis**, **1 section de configuration au noyau**, et
**aucune dépendance nouvelle au workspace**. La dernière dette de données simulées du projet est
fermée : `make check-api-contract` compte **0 route en attente**, contre 8 en ouvrant la session.

| Ce qui devient vrai | Où |
|---|---|
| L'écran des messages d'incident lit la plateforme réelle — liste, poste de direct, cibles, gabarit de débordement | `live/service/list.rs`, `live/routes/admin.rs` |
| Un message se rédige, se publie, se corrige et se retire — la trace reste | `live/service/write.rs` |
| Un bandeau publié se voit **du public**, sur la page des programmations | `live/routes/public.rs`, `frontend/app/pages/programme.vue` |
| Le tableau de bord dit la vérité de l'édition, **en une réponse et un instant** | `analytics/service/dashboard.rs` |
| Les chiffres ne vieillissent plus en silence | `analytics/jobs/refresh.rs`, `worker/src/main.rs` |


**27/08 — la spécification.** 6 histoires, 12 cas limites, **71 exigences**, 14 critères mesurables,
**4 décisions tranchées** et un périmètre hors-champ écrit. Aucun marqueur de clarification.
[Checklist de qualité](../../../specs/007-direct-tableaux-de-bord/checklists/requirements.md).

**27/08 — le plan.** [`plan.md`](../../../specs/007-direct-tableaux-de-bord/plan.md) — **29 décisions
techniques**, **9 routes**, **2 crates**, **1 travail différé**, **0 événement émis**, **3 codes
d'erreur**, contrôle constitutionnel **sans entorse au principe VIII** et **sans aucune dépendance
nouvelle**, ce qui n'était arrivé à aucun jalon de la phase B. Cinq artefacts :
[`research.md`](../../../specs/007-direct-tableaux-de-bord/research.md),
[`data-model.md`](../../../specs/007-direct-tableaux-de-bord/data-model.md),
[`contracts/`](../../../specs/007-direct-tableaux-de-bord/contracts/) (routes, erreurs, événements),
[`quickstart.md`](../../../specs/007-direct-tableaux-de-bord/quickstart.md).

**27/08 — le découpage.** [`tasks.md`](../../../specs/007-direct-tableaux-de-bord/tasks.md) — **146 tâches
en 9 phases**, dont **69 de test**, **88 parallélisables**, et **trois jalons** : l'écran des messages
entièrement branché (T001–T084), les deux écrans qui lisent la plateforme avec des chiffres frais
(T001–T123), la dette fermée (T001–T137). Répartition : US1 29, US2 25, US3 22, US4 10, US5 9, US6 5,
plus 46 tâches d'amorçage, de fondations et de polissage.

**US2 avant US1, contre l'ordre de la spécification** : le tableau de bord affiche les incidents actifs
de l'édition, et un incident n'existe que si quelque chose sait en poser un. Livrer US1 d'abord ferait
vérifier sa cinquième famille d'alerte sur une liste vide.

**Cinq avertissements en tête du découpage, dont quatre produisent un défaut entièrement silencieux** :
le schéma change et exige `down -v` ; les deux fonctions de publication **émettent déjà** ; une écriture
hors de la porte du noyau laisse `published_by` **nul sans erreur** ; un intervalle de rafraîchissement
plus court que la fenêtre d'anti-rebond **arrête la chaîne sans trace** ; et un filtre de périmètre écrit
à la main laisserait fuir **trois portées sur cinq**.

**Le plan a rouvert la spécification deux fois, et les deux amendements sont datés** — voir « Ce que
seul le plan pouvait révéler » plus bas.

**Deux modifications du modèle après le plan, aucune n'est un changement de schéma** : une ligne de
réglage (le seuil de l'écart n° 43) et une attribution de permission. Ni table, ni colonne, ni type, ni
fonction. Tout le reste est déjà servi par la base.

---

## Ce qui existait déjà, et qui n'est pas à refaire

C'est le premier constat de la session, et il change le contour du jalon : **le modèle sert tout, et la
moitié de l'outillage aussi.**

| Ce qui existe | Où | Conséquence |
|---|---|---|
| Le schéma `live` en entier | `080_live.sql` | Table `incidents`, `publish_incident()`, `unpublish_incident()`, `active_incidents(session)`, `event_incidents(event, at)`, `active_incidents_for_event(event, at)` |
| Le schéma `analytics` en entier | `130_analytics.sql` | 8 projections, `refresh_all()`, `enqueue_refresh()`, `refresh_log`, `v_platform_overview`, `v_operational_health` |
| Les deux modules déclarés en base | `010_platform.sql` § 7 | `('live','live',…,'{programme}')` et `('analytics','analytics',…,'{}')` — **rien à semer** |
| La permission `live.incident.publish`, portée par `admin` et `programmer` | `030_identity.sql` | **Rien à ajouter** — et rien à ajouter non plus pour la lecture (écart n° 1 d'A13) |
| La taxonomie `incident_kind`, neuf termes dont `overrun` | `080_live.sql` § 7 | **Rien à semer** |
| **Le worker écoute déjà les files `live` et `analytics`** | `worker/src/registry.rs` (`queues()`) et `jobs.rs` | Il ne manque **qu'un gestionnaire** pour `analytics.refresh_all` et sa chaîne récurrente — le patron d'armement existe pour six chaînes |
| **`GET /api/health` sert déjà `v_operational_health`** | Livré en **B1** | La zone 3 du tableau de bord a **déjà** sa route de rafraîchissement isolé. Elle n'entre pas dans les huit routes, et n'est pas à refaire |
| Les quatre contrats et les quatre jeux d'exemple du site | `types/`, `mocks/` | Spécification exécutable, composition de la zone d'alertes comprise |
| Les deux écrans, livrés et vérifiés au navigateur | A6 les 17-18/08, A13 le 18/08 | **Aucun écran n'est à réécrire** — seuls leurs appels basculent |
| Le patron de crate de module, de `repo/cross/`, de travail récurrent | `modules/programme/`, `worker/src/main.rs` | À suivre, pas à réinventer |

**Ce qui n'existe pas** : les crates `backend/crates/modules/live` et `backend/crates/modules/analytics`,
et les huit routes.

**Deux affirmations du prompt sont déjà périmées, et la spécification le dit** :

1. `composables/api/admin-incidents.ts` annonce « **sept** routes » ; le décompte réel est **huit** —
   les six écrites dans cette fabrique, plus le gabarit de débordement, plus le tableau de bord. La
   vérification du contrat les compte une par une.
2. Trois fichiers annoncent « **trois** écrans en données simulées ». La vitrine est basculée depuis
   B8 : ils sont donc **déjà faux d'un tiers**, et le seront de trois tiers à la fin du jalon.

**État mesuré du contrat au 27/08**, avant toute modification :

```
Contrat d'API : 139 appels sur 165 chemins, 130 formes annoncées — toutes définies.
8 route(s) en attente d'API.
```

Les huit sont exactement celles de ce jalon. **Aucune autre dette de données simulées n'existe dans le
dépôt** — c'est bien le dernier jalon qui la ferme.

---

## Les quatre décisions tranchées, et pourquoi

Le prompt laissait quatre points « à trancher dans la spécification, et non en chemin ». Le détail est
dans [`decisions/2026-08-27.md`](../decisions/2026-08-27.md) ; en résumé :

| N° | Le point | La décision |
|---|---|---|
| **D1** | Où vit la composition du tableau de bord, qui réunit `programme`, `org`, `live` et `analytics` ? | **Dans le crate `analytics`, sous `repo/cross/`.** L'entité propre de ce module **est** la mesure de la plateforme : ses huit projections lisent déjà six schémas en base, sa vue de santé en lit sept dans une seule requête. Le crate porte le code là où le modèle porte déjà ses vues. Trois autres emplacements écartés, dont le découpage en cinq lectures — que le contrat du site interdit explicitement |
| **D2** | Où vit le seuil qui rend un dossier « urgent » (écart n° 43, ouvert depuis le 17/08) ? | **Dans `platform.settings`**, semé à 21 jours. Pas une colonne d'appel : ce n'est pas une propriété de l'appel mais le réglage d'affichage d'un écran, et le porter par l'appel obligerait à le renseigner douze fois pour une valeur que personne ne veut faire varier. Une table qui porte déjà un seuil de même nature — `organization.duplicate_block_threshold` |
| **D3** | Une portée globale se retire-t-elle depuis une édition (point n° 3 d'A13) ? | **Non.** La permission d'écriture se vérifie **sur la portée réellement visée** : la portée globale pour un message global, qu'un compte détaché n'a pas. C'est la lecture stricte du principe V, et le sens **réversible** — ouvrir plus tard ne casse rien. Question au commanditaire inscrite aux points bloqués, non bloquante |
| **D4** | L'exposition publique du bandeau entre-t-elle dans ce jalon ? | **Oui, en dernière priorité** (histoire n° 5). Toutes les pièces existent — fonction montante en base, contrat, composant dessiné et vérifié : il manque **une lecture publique et un montage**. Bornée à la page d'une activité ; le bandeau sur la programmation complète et sur l'accueil reste au jalon suivant, la question « quel message s'affiche sur une page qui parle de trente activités » n'étant pas tranchée |

---

## Ce que seul le plan pouvait révéler (27/08)

Deux points de la spécification étaient **faux**, et aucun ne se voyait en lisant le modèle : il fallait
confronter la spécification au dépôt. C'est ce que la phase de plan existe pour produire.

### 1. Le tableau de bord aurait été refusé au rôle qui pilote une édition

La spécification gardait `GET /admin/dashboard` par le seul périmètre d'administration. Or le modèle
porte `analytics.dashboard.read` (`030_identity.sql` ligne 586), faite pour cet écran, et
**`GET /api/health` la teste depuis B1**.

Elle n'est attribuée qu'à `admin` et `reviewer`. **Pas à `programmer`** — qui a pourtant un périmètre
d'administration, `identity.administered_events()` étant adossée à `programme.proposal.read_all`. Et
c'est avec un tel compte — Fatou Nko Diop, `programmer` sur la seule COP31 — que la **règle métier n° 8 a
été vérifiée au navigateur sur cet écran le 17/08**.

Garder la permission telle quelle aurait donc refusé le tableau de bord au compte qui a servi à le
valider, et le défaut ne se serait vu qu'en se connectant. **Décision (R10)** : garder la route par la
permission — l'ignorer inventerait une règle contre le modèle —, et **ajouter une ligne** au catalogue
des rôles. Elle n'accorde **aucune élévation** : un programmateur lit déjà, écran par écran et pour sa
seule édition, tout ce que le tableau de bord agrège. C'est le raisonnement que le modèle a lui-même
écrit en accordant `identity.role.assign` à `admin`.

### 2. La page publique d'une activité n'existe pas

La décision D4 et les exigences FR-057 à FR-060 plaçaient le bandeau « sur la page publique de cette
activité ». **Cette page n'existe pas.** `frontend/app/pages/` n'en porte aucune — la programmation
publique est `pages/programme.vue`, une grille et un calendrier —, et `useApi.ts` le dit lui-même à
propos du détail public d'une séance : « aucun écran ne l'appelle encore : **la page publique d'une
séance n'est pas au jalon** ». C'est aussi la cause du constat de départ, `UiIncidentBanner` n'étant
monté que dans l'aperçu du formulaire et le guide de style.

L'histoire n° 5 aurait donc été « livrée » avec une route qu'aucun écran n'appelle — c'est-à-dire pas
livrée du tout, exactement le demi-livrable que D4 cherchait à éviter.

**Décision (R26)** : l'exposition se fait sur la **page des programmations**, à l'échelle de l'**édition
affichée**, par `GET /events/{event_id}/incidents` — donc par la fonction **descendante**. Et la question
que D4 reportait — « quel message s'affiche sur une page qui parle de trente activités » — se tranche
**par une règle qui existe déjà** : trois au plus, le plus grave en tête, le reste replié en « +N »,
comme les pastilles thématiques de la charte. Ce qui rend la réponse acceptable est que le modèle résout
**déjà** la cible : « Atelier de négociation — diffusion interrompue » informe, là où un bandeau anonyme
serait du bruit. Le cas d'usage du commanditaire est donc servi.

### 3. Et une mesure, faite parce que le contraire aurait coûté une journée

`REFRESH MATERIALIZED VIEW CONCURRENTLY` est réputé refusé dans une fonction et dans un bloc
transactionnel. Si c'était vrai, `analytics.refresh_all(true)` journaliserait **huit échecs sans lever**
— son `EXCEPTION WHEN OTHERS` les avale vue par vue — et le tableau de bord vieillirait en silence
pendant que le worker croirait travailler.

**Mesuré sur la base du dépôt, PostgreSQL 17** : `SELECT … FROM analytics.refresh_all(true)` rend huit
vues et huit succès, **et autant à l'intérieur d'un `BEGIN … COMMIT`**. Le gestionnaire peut donc
appeler la fonction telle quelle (R8).

---

## Écarts relevés en spécifiant B9 (27/08)

Numérotation continuée à partir de l'écart n° 142, le dernier numéroté (B7). Les « 152 écarts tranchés » de B7 sont un décompte, pas un numéro.

| N° | Écart | Où | Ce qu'il coûte | Suite donnée |
|---|---|---|---|---|
| **143** | **Le décompte de routes du site est faux d'une unité.** L'en-tête de `composables/api/admin-incidents.ts` annonce « aucune de ces **sept** routes n'existe encore » ; la fabrique en porte six, et le gabarit de débordement en est la septième — le tableau de bord, servi ailleurs, fait la huitième | `composables/api/admin-incidents.ts` | Un décompte lu comme un inventaire fait chercher une route de moins qu'il n'y en a | Le jalon en nomme **huit** ; l'en-tête est à corriger à la bascule, avec les trois autres |
| **144** | **Trois fichiers annoncent une dette d'un tiers plus grande qu'elle ne l'est.** « Trois écrans du jalon lisent encore des données simulées » — la vitrine est basculée depuis B8 | `CLAUDE.md` § Périmètre actuel, `composables/useApi.ts`, `composables/useMockData.ts` | Un fichier lu à chaque session qui affirme une chose fausse coûte plus qu'il ne coûte à corriger | **FR-064** : les trois sont corrigés à la bascule, et l'affirmation disparaît plutôt que d'être réduite à un |
| **145** | **La santé opérationnelle est déjà servie, et le prompt ne le sait pas.** `GET /api/health` rend les indicateurs de `v_operational_health` depuis B1 — c'est même le point de contrôle de la chaîne différée de ce jalon-là. Le site l'appelle déjà par `call`, pas par `pending` | `useApi.ts` (`admin.operationalHealth`), B1 | La refaire produirait deux routes pour une vue | **Écartée du périmètre**, et dite comme telle dans la spécification. La composition du tableau de bord porte la même vue : les deux servent une vue **non matérialisée**, donc le même instant |
| **146** | **Le worker écoute déjà les files du jalon, et c'était un défaut corrigé pour une autre raison.** `JobRegistry::queues()` liste les files des gestionnaires **déclarés** : « live » et « analytics » n'y sont donc **pas** aujourd'hui, faute de gestionnaire. Le commentaire de `jobs.rs` les nomme comme si elles l'étaient | `worker/src/registry.rs`, `jobs.rs` | Rien n'est cassé, mais le commentaire promet une écoute que le registre ne produit qu'une fois le gestionnaire écrit | **FR-051** : déclarer le gestionnaire de `analytics.refresh_all` sur la file « analytics » suffit à la faire écouter. Le point bloqué du 21/08 (« un travail différé DÉCLARE sa file ») se referme ici |
| **147** | **Aucune route ne sert les cinq autres projections, et c'est voulu.** Fiche de performance des organisations, participation par activité, charge du comité, inscriptions de plateforme, popularité des contenus : elles alimentent la composition du tableau de bord, aucun écran ne les affiche seules | `130_analytics.sql` § 5 à 8 | Les servir « parce qu'elles existent » produirait cinq routes sans appelant, que la vérification du contrat listerait indéfiniment comme non appelées | **Hors périmètre**, écrit. La charge du comité est lue **dans** la composition (revues en retard, avancement) sans route propre |
| **148** | **Le module `live` ne livre que sa part « messages d'incident », et le prompt ne le borne pas explicitement.** Le schéma porte aussi les réunions de visioconférence, les participants, le journal des webhooks et les diffusions — soit les quatre cinquièmes du fichier | `080_live.sql` § 2 à 5 | Un crate nommé d'après son schéma laisse croire qu'il le sert en entier | **Hors périmètre**, écrit : ni écran, ni contrat, ni prompt ne les demande. Le crate est créé au bon nom, il ne sera simplement pas complet |
| **149** | **Le point bloqué « le module `live` n'a aucun prompt d'API » se referme, et pas comme il était posé.** Le 21/08, la question était de le rattacher à un autre prompt ou de lui en donner un. B9 lui en donne un — **et un second au module analytique** | `points-bloques.md` (21/08) | — | Clos par ce jalon. La règle « un module = un schéma = un crate » est tenue : **deux crates, jamais un** |

---

## Ce qui a été vérifié le 27/08, et comment

Une spécification ne se prouve pas ; ce qui suit a été **mesuré ou lu dans la source de vérité**, jamais
supposé.

| Contrôle | Résultat |
|---|---|
| **Combien de routes restent réellement en attente ?** | `node frontend/scripts/check-api-contract.mjs --verbose` : **8**, nommées une par une, toutes de ce jalon. Aucune autre dette de données simulées dans le dépôt |
| **Le modèle sert-il vraiment tout ?** | Les six fonctions et la table du schéma `live` § 6 lues **en entier**, ligne à ligne ; les projections, `refresh_all`, `enqueue_refresh` et les deux vues du schéma `analytics` de même. **Aucun nom de champ n'est recopié de mémoire** dans la spécification |
| **Les deux modules sont-ils déjà déclarés en base ?** | Oui — `010_platform.sql` § 7, lignes 530 et 536. Rien à semer, et le montage conditionnel du binaire HTTP les reconnaîtra sans modification du semis |
| **La permission de lecture existe-t-elle ?** | Non : le catalogue ne porte que `live.incident.publish` (`030_identity.sql` ligne 578), attribuée à `admin` et `programmer`. L'écart n° 1 d'A13 est confirmé sur la source — **ne pas en ajouter** |
| **Le périmètre global couvre-t-il une portée d'édition ?** | Oui, et c'est ce qui rend D3 opérante : `has_permission()` accepte une attribution `global` pour **n'importe quelle** portée demandée, et une attribution `event` **uniquement** pour son édition. Vérifier sur la portée globale exclut donc bien un compte détaché, sans code supplémentaire |
| **Le poste de direct peut-il se composer sans inventer un champ ?** | Oui : `programme.sessions` porte `is_streamed`, `status`, `starts_at`, `ends_at`, `room_id`, et `v_public_schedule` calcule `temporal_state` dans l'ordre annulé → reporté → à venir → en cours → passé. Le poste doit **reprendre cet ordre**, pas en composer un autre |
| **Le seuil d'urgence a-t-il déjà un précédent en base ?** | Oui — `organization.duplicate_block_threshold` est semé dans `platform.settings` (`900_seed.sql`). D2 suit un patron existant, elle n'en invente pas un |
| **Le worker sait-il déjà armer une chaîne récurrente ?** | Oui, six fois : purge des jetons, balayage des doublons, clôture des appels échus, purge des objets, réconciliation des quotas, partitions du journal d'expédition. Chacune se replanifie et porte une clé d'unicité qui empêche dix redémarrages d'en armer dix |
| **Les exigences citent-elles une table ou une colonne ?** | Relevé mécaniquement sur les sections « Requirements » et « Success Criteria » : **aucune**. Les seuls identifiants sont les huit chemins de routes, deux noms de schéma, un chemin de dossier, un code de permission, trois valeurs de portée et deux cibles du `Makefile` — chacun justifié dans la checklist |
| **Le dépôt a-t-il bougé ?** | Non. Cette session **n'a modifié aucun fichier de `frontend/`, de `backend/` ni de `docs/database/`** : elle spécifie |

---

## Ce que l'implémentation a tranché, et que le plan n'avait pas prévu

| Point | Ce qui a été fait, et pourquoi |
|---|---|
| **Le bandeau public ne pouvait pas nommer son sujet** | `ActiveIncident` gagne `target_id` et `target_label`, **facultatifs**, et `UiIncidentBanner` une propriété `targetLabel`. Sans eux, « la diffusion est interrompue » ne dit pas laquelle sur une page qui parle de trente activités — et la portée seule ne suffit pas : « cette activité » ne nomme rien quand le bandeau coiffe tout un programme. Deux champs ajoutés, **aucun changé** |
| **Le périmètre et l'autorisation ont quitté les routes** | Ils vivent dans `service/`, et les gestionnaires Actix sont devenus des enveloppes. Les y laisser les rendait **inéprouvables** sans une dépendance de développement vers `api` — que `cargo tree` verrait, et que le contrôle de frontière du principe II refuse |
| **`unpublished_by_name` ne vient pas de la fonction** | `live.event_incidents()` rend l'instant du retrait et son motif, mais **pas le nom** de qui a retiré. Le dépôt le complète par une jointure ; sans elle, l'historique afficherait « retiré par — » alors que la colonne porte l'identifiant |
| **`ModuleRegistry::complet()` ignorait les deux modules** | Le binaire qui exporte l'OpenAPI s'en sert, et les neuf routes n'auraient donc **jamais figuré au contrat** — `make openapi` aurait rendu un client sans elles, sans rien signaler |

---

## Ce qui reste ouvert

- **Rien n'a été vérifié au navigateur contre l'API réelle.** La suite d'intégration tourne sur base
  jetable et le contrat est vert, mais les deux écrans du back-office et la page des programmations
  n'ont pas été rouverts.
- **Une question au commanditaire**, inscrite aux [points bloqués](../points-bloques.md) : quand un
  message d'entretien s'affiche sur tout le site, l'équipe d'une seule COP doit-elle pouvoir l'enlever
  elle-même ? Non bloquante — la position tenue est la réversible, et c'est celle qui est codée.
- **Aucun consommateur d'outbox ne déclenche de rafraîchissement**, et c'est une décision : l'effet
  serait invisible et l'écart au plus d'un quart d'heure. À reprendre si l'intervalle s'allonge
  au-delà de l'heure.
- **`live.active_incidents(session)` n'est appelée par rien** : la page publique d'une activité
  n'existe pas ; elle viendra avec son écran.

---

## Les trois pièges nommés d'avance, et ce que chacun coûterait

| Piège | Le symptôme si on le manque | Où il est traité |
|---|---|---|
| **L'intervalle de rafraîchissement plus court que la fenêtre d'anti-rebond** | la chaîne récurrente **se dédoublonne contre elle-même et s'arrête** — sans erreur, sans trace : le conflit de `platform.jobs` porte sur `(tâche, clé)` **quel que soit l'état du travail**, un travail réussi bloquant donc une nouvelle mise en file de la même tranche | R9 — 15 min contre 5, et le réglage est **refusé au démarrage** s'il casse la chaîne |
| **Le service qui émet « pour faire comme les autres »** | deux lignes d'outbox par publication, donc deux effets de bord le jour où quelqu'un branche un consommateur. Le piège n° 1 des six modules précédents | R5 — un test compte les lignes et exige **exactement une** |
| **Une écriture qui contourne la porte du noyau** | `live.publish_incident()` pose `published_by` depuis le contexte de transaction : sans `SET LOCAL`, la colonne est **nulle sans erreur** et le back-office affiche « publié par — » | R5 — un test le vérifie **sur la valeur de la colonne**, pas sur l'audit |
