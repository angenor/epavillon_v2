# Phase 0 — Décisions techniques

**Fonctionnalité** : Événements (B3) · **Date** : 2026-08-20 · **Spécification** : [spec.md](spec.md)

Vingt décisions. Chacune porte ce qui a été retenu, pourquoi, et ce qui a été écarté. Les décisions de B1 et de B2 qui tiennent encore — forme du jeton d'accès, transport de la session, politique de statut, traduction des erreurs, relais d'outbox, file de travaux, harnais de test, unique porte d'écriture — **ne sont pas rejouées** : elles vivent dans [`001-socle-identite/research.md`](../001-socle-identite/research.md) et [`002-organisations/research.md`](../002-organisations/research.md) et s'appliquent telles quelles.

---

## R1 — La règle du sigle : elle vit dans le service, et elle propose une valeur

**Décision.** Une fonction pure de `domain/acronym.rs` porte les deux moitiés de la règle :

```
exiger(has_pavilion, acronym)   →  Ok | Manquant { propose } | Format
proposer(titre_fr)              →  Option<String>
```

Elle est appelée **à la création et à la modification**, sur l'**état résultant** de la charge utile — jamais sur la ligne existante. Retirer le sigle d'une édition à pavillon, ou basculer en « pavillon tenu » sans en fournir, produisent donc exactement le même refus.

**Le format** : 2 à 12 caractères, `[A-Za-z0-9-]`. Le sigle est conservé **tel qu'il a été saisi** ; il est mis en majuscules par le déclencheur d'affectation du numéro de dossier, pas par nous.

**La valeur proposée** se dérive du libellé français : accents dépliés, tout ce qui n'est ni lettre ni chiffre retiré, majuscules, tronqué à douze. « COP31 — Conférence des Parties » donne `COP31CONFER` ; « Rendez-vous de l'IFDD » donne `RENDEZVOUSD`. Quand il en sort moins de deux caractères, **on ne propose rien** plutôt qu'une valeur absurde : le refus reste, sans suggestion.

**Pourquoi pas en base.** Le modèle *pourrait* l'exprimer : `CHECK (NOT has_pavilion OR acronym IS NOT NULL)`. On s'en abstient exprès. L'arbitrage retenu (question n° 9, option A) dit que **la base continue d'accepter une édition sans sigle** — parce que la reprise des données de la v1 devrait sinon inventer un sigle pour chaque édition passée, et parce qu'une édition sans pavillon n'a aucun numéro de dossier à préfixer. Une règle de saisie n'est pas un invariant de données ; l'inscrire en base interdirait un cas d'usage existant.

**Alternatives écartées.**
- *Rendre `acronym` non nul* — casse les webinaires du cycle PACO et la migration. C'est l'option B de la question, explicitement rejetée.
- *Laisser la règle au front seul* — une requête forgée passerait, et le prompt demande la contrainte côté service.
- *Générer le sigle d'office sans le demander* — la valeur serait moche et personne ne l'aurait choisie ; or elle finit imprimée sur chaque numéro de dossier communiqué. On **propose**, on n'impose pas.

---

## R2 — Le périmètre : remonter à l'édition avant de vérifier, sans divulguer ce qu'on a lu

**Décision.** B1 a livré tout ce qu'il faut : `kernel::auth::{AdminScope, Perimeter, require_perimeter, administered_events}`, les trois cas distincts et le refus explicite sur périmètre vide. **Ce module n'en réécrit rien.**

Il ajoute une seule pièce, dans `repo/cross.rs` : la **résolution d'ascendance**.

```
event_id_of_track(track_id)    event_id_of_room(room_id)
event_id_of_venue(venue_id)    event_id_of_channel(channel_id)
event_id_of_call(call_id)      event_id_of_day(day_id)
```

Six routes sont paramétrées par un identifiant d'enfant (`/admin/tracks/{id}`, `/admin/venues/{id}`, `/admin/rooms/{id}`, `/admin/channels/{id}`, `/admin/calls/{id}`, `/admin/calls/{id}/reviewers`). L'ordre est **imposé** : résoudre, puis `Perimeter::ensure(event_id)`, puis agir.

**La lecture d'ascendance ne divulgue rien** : elle ne rend qu'un identifiant d'édition, jamais exposé, et son absence produit **le même refus** que l'échec du périmètre. Un identifiant inexistant et un identifiant hors périmètre sont indiscernables par la forme de la réponse — principe IX.

**Une subtilité de charge utile.** Le front envoie `event_id` dans le corps des routes de suppression (`{ event_id: eventId }`) et dans les charges utiles d'écriture. **Il est ignoré** : c'est un droit déclaré par le client. L'édition vient toujours de l'ascendance en base. Même motif que les paramètres `personId` / `actorId` de B1 et B2.

**Alternatives écartées.**
- *Faire confiance à l'`event_id` du corps* — c'est le contrôle d'accès délégué au client, que le principe V interdit en une phrase.
- *Vérifier le périmètre après l'écriture, en transaction annulable* — coûte une transaction pour un refus, et laisse la porte ouverte à un effet de bord (déclencheur d'audit) sur une édition qu'on n'a pas le droit de toucher.
- *Une seule route générique `/admin/events/{id}/…` pour tous les enfants* — renégocierait les chemins du front, ce que la constitution interdit.

---

## R3 — Le détail en une réponse : une connexion, une transaction en lecture seule, douze lectures

**Décision.** `EditionDetail` porte douze collections (l'édition, ses textes, sa période, ses trois images, journées, fils, lieux et salles, canaux, appel et grille, comité, responsables assignables, candidats, thématiques). Elles sont lues **séquentiellement sur une seule connexion**, dans une transaction `REPEATABLE READ READ ONLY`.

**Pourquoi pas un éventail concurrent.** Douze requêtes lancées de front prennent douze connexions du pool pour un seul écran d'administration. B2 a payé exactement cette monnaie : cent créations concurrentes sortaient en « service indisponible » parce qu'une transaction perdante retenait **deux** connexions avant d'être rendue. Un back-office ouvert à la main n'a aucun besoin de gagner cinquante millisecondes au prix d'un pool saturé.

**Pourquoi une transaction et pas douze lectures libres.** La réponse mêle des décomptes venus de trois schémas. Sans instantané commun, l'onglet des journées pourrait annoncer trois séances quand celui des salles en compte quatre, pour la même édition et au même instant. `REPEATABLE READ` rend les douze lectures cohérentes entre elles, et `READ ONLY` le dit à PostgreSQL.

**Conséquence assumée** : environ douze allers-retours. Sur une base locale, quelques millisecondes ; sur un réseau, quelques dizaines. C'est un écran d'administration, pas une recherche interactive.

**Alternatives écartées.**
- *Une seule requête à douze agrégats JSON* — illisible, impossible à vérifier à la compilation champ par champ, et un décompte faux y serait invisible.
- *Un appel par onglet* — le contrat du front dit l'inverse, et pour une bonne raison : l'équipe passe d'un onglet à l'autre sans arrêt en préparant une COP.
- *Un cache* — six onglets dont les décomptes doivent être justes après chaque écriture. Un cache serait faux la moitié du temps (R20).

---

## R4 — Le plan de génération : une fonction pure, recalculée dans la transaction

**Décision.** `domain/calendar.rs` porte une fonction pure :

```
plan(premier_jour, dernier_jour, journees_existantes) → DayGenerationPlan
```

`GET /admin/events/{id}/days/plan` la nourrit d'une lecture et rend son résultat. `POST /admin/events/{id}/days` la **recalcule dans sa propre transaction d'écriture**, à partir des lignes qu'elle vient de lire, et n'utilise du client que le drapeau `remove_outside_period`.

**Pourquoi.** Entre l'affichage du plan et le clic, quelqu'un peut avoir modifié la période ou créé une journée. Faire confiance au plan renvoyé, c'est écrire d'après un état périmé — et ici, écrire d'après un état périmé veut dire **supprimer une journée qui vient d'entrer dans la période**, avec les séances qu'elle porte.

**La même fonction sert l'enregistrement d'une édition** (FR-033), qui crée les journées manquantes et n'en supprime aucune : c'est le même plan, dont on n'exécute que la moitié « créer ».

**Alternatives écartées.**
- *Passer le plan en charge utile* — l'état périmé ci-dessus, plus un contrôle d'accès délégué au client sur la liste des identifiants à supprimer.
- *Un jeton d'optimisme sur la période* — beaucoup d'appareillage pour un écran que deux personnes ouvrent au plus, et qui n'empêcherait pas le cas courant : la même personne, deux onglets.

---

## R5 — Les jours civils se calculent EN BASE, dans le fuseau de l'édition

**Décision.** La liste des dates d'une édition ne se calcule jamais en Rust :

```sql
SELECT d::date
FROM generate_series(
        (e.starts_at AT TIME ZONE e.timezone)::date,
        (e.ends_at   AT TIME ZONE e.timezone)::date,
        interval '1 day') AS d
```

La même expression sert `EditionDetail.period` et le marquage `is_outside_period` d'une journée.

**Pourquoi.** Une édition porte un instant de début, un instant de fin et un **fuseau de référence**. Le premier jour civil d'une COP à Belém n'est pas celui du serveur : à trois heures de décalage, « 2027-11-09 00:30 UTC » est encore le 8 novembre sur place. Calculer les dates ailleurs que dans le fuseau de l'édition décale le calendrier d'un jour — pas toujours, ce qui est pire.

**Et pourquoi pas avec une bibliothèque de fuseaux en Rust.** Parce que la base de fuseaux qui fait foi est celle de PostgreSQL : c'est elle qui valide le domaine `platform.timezone_name`, donc elle qui décide quels identifiants existent. Deux bases de fuseaux dans une même chaîne, c'est exactement le défaut qui a fait tomber le formulaire du front : `Europe/Geneva` est accepté par PostgreSQL et refusé par certaines exécutions de Node, et **une seule exception emportait la liste entière des fuseaux**.

**Alternatives écartées.**
- *`chrono-tz` côté Rust* — une seconde base de fuseaux, qui dérivera de celle du serveur au premier changement de règle d'heure d'été.
- *Stocker les dates civiles* — donnée dérivée dupliquée, que le modèle ne porte pas et qu'il faudrait maintenir.

---

## R6 — Le canal par défaut : retirer avant de poser, dans la même transaction

**Décision.** Poser `is_default` sur un canal se fait en deux ordres, dans cet ordre, dans une seule transaction :

```sql
UPDATE event.broadcast_channels SET is_default = false
 WHERE COALESCE(event_id, '00000000-…'::uuid) = COALESCE($1, '00000000-…'::uuid)
   AND is_default;
-- puis l'insertion ou la mise à jour du canal visé
```

**Pourquoi cet ordre.** `ux_broadcast_channels_default` est un index unique **partiel** sur `COALESCE(event_id, …)`, restreint à `is_default AND is_active`. Il n'est pas différable : poser d'abord violerait l'unicité au milieu de la transaction. Retirer d'abord est la seule séquence qui passe.

**Deux pièges de cet index, tous deux vérifiés dans le SQL :**

1. Il ne porte que sur les canaux **actifs**. Un canal par défaut désactivé libère la place — et c'est cohérent : un canal inactif n'occupe rien.
2. Les canaux **généraux de la plateforme** (sans édition) forment leur propre groupe, sous un identifiant de substitution. Le semis en pose déjà un, actif et par défaut (R19). Un canal par défaut d'édition **ne le déloge pas**, et c'est voulu : le canal général sert les diffusions dont l'événement n'a pas le sien.

**Le code d'un canal a la même asymétrie** : `ux_broadcast_channels_code` est `NULLS NOT DISTINCT`, donc deux canaux généraux ne peuvent pas partager un code, alors qu'un canal général et un canal d'édition le peuvent.

**Alternatives écartées.**
- *Poser puis retirer* — viole l'index, échoue à l'exécution.
- *Laisser l'écran décocher le précédent* — deux écritures, dont une que l'équipe peut oublier ; et entre les deux, l'état est refusé par la base.
- *Ne rien faire et traduire l'erreur d'unicité* — le contrat du front dit que poser un défaut retire le précédent. Traduire un refus serait renégocier le contrat.

---

## R7 — Retirer un canal : désactiver s'il a servi, supprimer sinon

**Décision.** `DELETE /admin/channels/{id}` compte d'abord les séances qui référencent le canal (lecture hors schéma, R14) :

- **au moins une** → `UPDATE … SET is_active = false`, réponse `{ ok: true, error_code: 'deactivated' }` ;
- **aucune** → `DELETE`, réponse `{ ok: true, error_code: null }`.

**Pourquoi ne pas simplement supprimer.** La clé étrangère est `ON DELETE SET NULL` : aucune séance ne serait perdue. Ce qui serait perdu, c'est **la trace du canal sur lequel une activité passée a été diffusée** — et c'est précisément ce qu'un bilan d'édition va chercher.

**`error_code: 'deactivated'` avec `ok: true` est un succès**, pas un refus. Le contrat du front le dit ainsi, et l'écran affiche un message d'information. C'est le seul endroit du module où ce champ ne signale pas une erreur ; l'annotation OpenAPI le dit à l'endroit où on serait tenté de croire l'inverse.

**Alternatives écartées.**
- *Toujours désactiver* — laisse s'accumuler des canaux jamais utilisés, créés par erreur, dont le code reste pris.
- *Toujours supprimer* — perd la trace, et l'écran ne peut plus expliquer une diffusion passée.

---

## R8 — Le décompte de détachement se prend AVANT de détacher

**Décision.** Toute suppression qui détache des séances compte **dans la même transaction, avant l'ordre de suppression** :

| Suppression | Ce qui est compté |
|---|---|
| une journée du calendrier | séances dont le jour est cette journée |
| un fil de programmation | lignes de rattachement séance–fil |
| une salle | séances installées dans cette salle |
| un lieu | séances installées dans **l'une quelconque** de ses salles |

**Pourquoi avant.** Les clés sont `ON DELETE SET NULL` (journée, salle) ou `CASCADE` (rattachement à un fil). Après l'ordre, le lien n'existe plus : le décompte rendrait **zéro**, et l'écran annoncerait sereinement qu'il n'a rien détaché.

**La leçon de B2 vaut ici en creux.** Là-bas, un décompte annoncé divergeait de la réalité parce qu'un déclencheur déplaçait une ligne avant que la boucle n'y arrive. Ici, **aucun déclencheur ne touche à ces liens** — vérifié : les seuls déclencheurs du module sont deux audits et cinq horodatages. Le chiffre pris avant est donc exact, et le test peut l'exiger au chiffre près.

**Alternatives écartées.**
- *Compter après, sur la table cible* — rend zéro, silencieusement.
- *Faire rendre le compte par un `UPDATE … RETURNING`* — ne marche que pour les `SET NULL` explicites ; ici c'est la base qui les fait, en cascade sur la suppression du parent. Le faire nous-mêmes serait réimplémenter la cascade.

---

## R9 — L'appel et sa grille : une transaction, un diff, et un critère qu'on refuse de supprimer

**Décision.** `saveCall` est **une seule transaction** : l'appel d'abord, puis le diff de la grille.

Le diff se fait par **code**, qui est unique par appel :

| Cas | Action |
|---|---|
| code présent dans la charge, absent en base | `INSERT` |
| code présent des deux côtés | `UPDATE` |
| code absent de la charge, présent en base | **compter les notes, puis supprimer ou refuser** |

**Le refus est l'entorse assumée du plan.** `xmod_fk_review_scores_criterion` est `ON DELETE CASCADE` : supprimer un critère **détruit les notes qui s'y rapportent**, sans erreur et sans trace. Or ces notes sont l'argumentaire d'une décision de sélection — ce que la v1 n'avait pas, et qui rendait un refus inexplicable à l'organisation qui le contestait. Le service compte donc, et refuse en nommant le critère et son nombre de notes (`EVENT_CRITERION_HAS_SCORES`, 422).

**`scores_affected`** vaut vrai lorsqu'un critère **conservé** voit son barème ou son poids changer **et** porte déjà des notes. Les notes ne bougent pas ; les moyennes, si — c'est `refresh_proposal_score()` qui les repondère, et un classement qui se déplace sans explication est une conversation difficile avec le comité.

**Une grille vide est refusée** (`criteria_empty`, dans le contrat, 200) : un appel sans critère ne peut évaluer aucun dossier. **Deux codes identiques** sont refusés en désignant le **rang** de la ligne fautive, ce que le contrat du front prévoit par `criterion_index`.

**Alternatives écartées.**
- *Deux appels, l'un pour la campagne, l'autre pour la grille* — laisse exister un appel sans grille le temps d'un oubli, et le contrat du front les envoie ensemble.
- *Diff par identifiant* — une ligne ajoutée à l'écran n'en a pas encore ; le contrat le dit (`id: CriterionId | null`). Le code est la clé stable.
- *Laisser la cascade faire son travail* — perte silencieuse d'un argumentaire opposable.
- *Changer la clé en `RESTRICT`* — modification du modèle non justifiée ; et la cascade est **juste** quand c'est l'appel entier qui disparaît, son cas d'origine.

---

## R10 — Publier la programmation : contrôler, estampiller, annoncer

**Décision.** `POST /admin/planner/publish` fait trois choses, dans une seule transaction :

1. **Contrôler** — `programme.publication_readiness(event_id)`, lecture hors schéma. S'il subsiste un point de gravité `blocking`, rien n'est écrit et la réponse rend `{ blocked: true, published_count: 0, published_at: null, issues }`.
2. **Estampiller** — `UPDATE event.events SET programme_published_at = now() WHERE id = $1 AND programme_published_at IS NULL`. La clause finale rend la republication inoffensive : la date d'origine ne s'écrase pas.
3. **Annoncer** — `platform.emit_event('event.programme.published', …)`, dans la même transaction, avec l'édition et **le prédicat exact** des séances à rendre publiques.

Le module Programmation consommera cet événement en B5 et posera `published_at` sur les séances désignées, avec garde de rejeu par `platform.inbox_events`.

**Pourquoi l'annonce et non l'écriture.** La vue de la programmation publique filtre sur `s.published_at IS NOT NULL` — **la date de chaque séance**, pas celle de l'édition. Rendre le programme public exige donc deux écritures dans deux schémas. Écrire dans `programme` depuis ce module romprait la frontière posée en B2 ; confier toute la publication à B5 la romprait dans l'autre sens, B5 écrivant alors dans `event.events`. L'outbox est la troisième voie, et c'est **exactement** ce que le principe IV décrit : « les effets de bord inter-modules passent par `platform.emit_event()`, appelée dans la même transaction que le changement d'état ».

**`published_count` est un décompte de désignation, pas d'exécution.** Il vaut le nombre de séances que le prédicat désigne, compté sous l'instantané de la transaction : séances de l'édition, statut `planned` ou `scheduled`, non encore publiées. Le consommateur publie **ce prédicat et pas un autre** ; le chiffre annoncé et l'effet obtenu viennent donc du même raisonnement — la garantie que B2 a payé cher sur son décompte de fusion.

**La limite, nommée** : entre l'annonce et la consommation, une écriture concurrente sur les séances ferait diverger le chiffre. Seul le module Programmation peut en produire, et il n'est pas livré. **À revérifier en B5**, où le test de bout en bout comparera l'annonce et l'effet.

**Une édition sans aucune séance publie**, avec zéro séance et une liste de contrôle vide. Ce n'est pas un conflit.

**Alternatives écartées.**
- *Écrire les deux tables depuis ce module* — rompt la frontière ; et le jour où `programme` devient un service autonome, la ligne est invisible.
- *Rendre `published_count: 0` et laisser l'écran sonder* — le contrat du front attend le chiffre, et sonder demanderait une route que personne n'a spécifiée.
- *Ne pas estampiller l'édition et tout laisser aux séances* — la vue des éditions publiques expose `programme_published_at`, la frise d'accueil s'en sert, et la colonne cesserait d'être renseignée par qui que ce soit.

---

## R11 — Le préfixe du planificateur se compose dans `api`, une seule fois

**Décision.** `lib.rs` du module expose **deux** fonctions de montage :

```rust
pub fn routes(cfg: &mut ServiceConfig)          // /events, /event-series, /admin/events, /admin/tracks…
pub fn planner_routes(cfg: &mut ServiceConfig)  // readiness, publish — SANS leur préfixe
```

`api` compose `web::scope("/admin/planner")` **une seule fois** et y verse les contributions de chaque module monté — aujourd'hui `event`, demain `programme`.

**Pourquoi maintenant, alors que B5 n'existe pas.** Parce que le défaut est connu, daté et coûteux : en B2, **trois routes sur vingt et une étaient muettes** — deux `web::scope` du même préfixe ne se complètent pas, Actix retient le premier et rend 404. Le correctif a été posé pour `/people`, où deux modules déposent des routes. `/admin/planner` est le prochain préfixe partagé du projet ; l'écrire correctement du premier coup coûte cinq lignes, le découvrir en B5 coûte une enquête.

**L'ordre d'enregistrement compte aussi, et pour une autre raison.** Trois chemins littéraux seraient capturés par un chemin paramétré s'ils étaient déclarés après lui :

| À déclarer avant | Sinon capturé par |
|---|---|
| `GET /events/public` | `GET /events/{slug}` |
| `GET /admin/events/form-options` | `GET /admin/events/{id}` |
| `GET /admin/calls/default-criteria` | *(aucun conflit aujourd'hui, déclaré d'abord par principe)* |

**Alternatives écartées.**
- *Monter les deux routes sous `/admin/events/{id}/…`* — renégocie les chemins du front.
- *Attendre B5 pour composer le préfixe* — c'est exactement ce qui a produit le défaut de B2.

---

## R12 — La publication est gardée par la permission de planifier, pas par celle de gérer les événements

**Décision.** `GET /admin/planner/readiness` et `POST /admin/planner/publish` exigent `programme.session.schedule` sur la portée de l'édition ou sur la portée globale.

**Pourquoi une permission d'un autre module.** Le modèle décrit le rôle chargé de la programmation comme celui qui « planifie les créneaux **et publie la programmation** », et lui attribue cette permission — pas une permission de ce module. Garder la route par la gestion des événements empêcherait un chargé de programmation de publier ce que son rôle dit qu'il publie, alors qu'il ne détient **aucune** permission de ce module (écart n° 88).

**Cela ne crée aucune dépendance entre crates.** Le garde d'autorisation vit dans `kernel` depuis B1 — c'était la décision d'architecture qui a rendu B2 à B6 possibles —, et les permissions sont des **chaînes de caractères lues en base**, pas des symboles d'un autre crate. `cargo tree -p event` reste sans arête.

**Alternatives écartées.**
- *Garder par `event.event.manage`* — contredit le rôle décrit par le modèle.
- *Exiger les deux* — un administrateur global les a toutes ; un chargé de programmation n'a que la première : exiger les deux le bloquerait sans raison.
- *Ajouter une permission `event.programme.publish` au modèle* — modification du semis, donc du modèle, pour une permission qui existe déjà sous un autre nom.

---

## R13 — Une écriture d'édition est totale, jamais partielle

**Décision.** `EditionFormPayload` porte **tous** les champs modifiables de l'édition. La mise à jour est donc un `UPDATE` complet, écrit une fois, vérifié à la compilation. **Aucun SQL n'est composé dynamiquement dans ce module.**

**Pourquoi c'est un choix et pas une évidence.** Une mise à jour partielle demanderait soit un `UPDATE` par combinaison de champs, soit du SQL composé — la seule entorse au principe VI que B2 ait dû s'accorder. Ici, le contrat du front envoie le formulaire entier : l'entorse n'a pas lieu d'être, et on se garde de l'introduire par confort.

**Deux champs échappent à cette règle et c'est normal** : `status` et `programme_published_at`. Le premier est dans la charge utile et suit la même règle. Le second n'y est pas — il est posé par la publication seule (R10), et une écriture d'édition ne doit jamais le toucher.

**Alternatives écartées.**
- *Charge utile partielle avec `Option<Option<T>>`* — trois états par champ (absent, nul, valeur) pour dix-huit champs, et un `UPDATE` composé.
- *Un `UPDATE … SET x = COALESCE($1, x)`* — rend impossible d'effacer un champ facultatif, ce qui casse la mise à nul du sigle, de la ville ou des coordonnées.

---

## R14 — Les lectures hors schéma sont réunies dans un fichier, et énumérées

**Décision.** Toutes les requêtes de ce module qui lisent hors de son schéma vivent dans `repo/cross.rs`. Il y en a **neuf**, et il n'y en a pas d'autres :

| Lecture | Sert |
|---|---|
| dossiers déposés par édition (brouillons exclus) | la liste et l'appel |
| séances par édition, et séances placées en salle | la liste |
| séances par journée, par salle, par lieu, par canal | les décomptes et les détachements (R8) |
| rattachements séance–fil par fil | le décompte d'un fil |
| notes posées par critère | le refus de suppression (R9) |
| contrôle avant publication d'une édition | la publication (R10) |
| séances à publier (prédicat) | le décompte annoncé (R10) |
| dossiers confiés et revues rendues par membre du comité | l'onglet du comité |
| personnes assignables : responsables de fil, candidats au comité | les deux listes du détail |
| thématiques d'un fil, et thématiques disponibles | les fils |

**Pourquoi les réunir.** La règle de frontière — *un module lit hors de son schéma quand la question porte sur ses propres entités, il n'y écrit jamais* — est facile à énoncer et facile à enfreindre par accroissement. Dispersées dans huit dépôts, ces requêtes deviennent invisibles ; réunies, elles se relisent en un fichier, et **c'est là qu'un ajout se discute**. Chacune porte en commentaire la question de ce module à laquelle elle répond.

**Aucune de ces lectures n'écrit.** Le fichier ne contient aucun `INSERT`, `UPDATE` ni `DELETE`, et c'est vérifiable d'un coup d'œil — c'est le second intérêt du regroupement.

**Alternatives écartées.**
- *Les répartir par agrégat* — plus « propre » en apparence, et la frontière devient impossible à auditer.
- *Passer par des vues du modèle* — il n'en existe pas pour ces décomptes, et en créer serait modifier le SQL.

---

## R15 — Un seul travail différé : la clôture d'un appel échu

**Décision.** Un travail récurrent, `event.call.autoclose`, qui **se replanifie lui-même** — le patron de la purge des jetons de B1, où le démarrage du worker ne fait que **réarmer** la chaîne au cas où sa dernière occurrence serait morte avant d'avoir posé la suivante.

Il passe en `closed` tout appel dont le statut est `open` et dont l'échéance effective — prolongation comprise, par la fonction du modèle — est passée, et émet `event.call.closed` pour chacun.

**Pourquoi il faut ce travail.** Sans lui, un appel resterait « ouvert » après son échéance jusqu'à ce que quelqu'un y pense. La fonction `is_call_open()` protège la soumission, qui tient compte de la fenêtre — mais le **statut** affiché reste faux, sur la page publique comme dans la liste, et c'est ce que lit une organisation qui se demande si elle peut encore déposer.

**Ce qui n'est PAS livré ici** : le rappel d'échéance aux organisations. Les règles de rappel et les modèles de message multilingues vivent dans le module Engagement (B6) ; les recopier ici produirait un second dispositif de rappel.

**Alternatives écartées.**
- *Un déclencheur en base* — un déclencheur ne se déclenche pas au passage du temps ; il faudrait un ordonnanceur en base, que le modèle ne porte pas.
- *Fermer à la lecture* — écrire pendant une lecture publique, non authentifiée. Non.
- *Ne rien faire, `is_call_open()` suffit* — vrai pour la recevabilité, faux pour l'affichage, et c'est l'affichage que l'organisation lit.

---

## R16 — La page publique d'une édition : une requête, deux vues

**Décision.** `GET /events/{slug}` et `GET /events/public` lisent `event.v_public_editions`, jointe **par la gauche** à `programme.v_edition_stats` sur l'identifiant d'édition. Une requête, aucune recomposition de jointure.

**Ce que cela referme, gratuitement** : l'écart n° 25. La vue rend déjà les **trois** déclinaisons d'image, résolues par `media.attached_image()` pour les rôles `banner`, `cover` et `thumbnail`. La lecture d'image séparée du front disparaît sans qu'on écrive une ligne pour cela.

**Et l'écart n° 26** : le critère de publicité — ni brouillon, ni annulée — est **dans la vue**, donc dans le modèle. Il cesse d'être recopié dans chaque écran, et une édition **annoncée** dont le programme n'est pas publié y figure, comme le fichier le dit en toutes lettres.

**La jointure est par la gauche, et c'est important.** `v_edition_stats` ne porte que les éditions ayant au moins une séance publiée. Une jointure stricte ferait disparaître de l'historique toute édition annoncée — c'est-à-dire précisément celle sur laquelle on dépose un dossier. C'est la leçon de B2, où une liste jointe par l'intérieur à une projection était **vide sur une base neuve**.

**Alternatives écartées.**
- *Deux appels, l'un pour l'édition, l'autre pour les statistiques* — un aller-retour de plus pour cinq colonnes de la même page.
- *Ajouter les statistiques à la vue des éditions* — impossible : `event` se charge **avant** `programme`, et le fichier explique pourquoi la dépendance va dans ce sens.

---

## R17 — Les images sont lues, jamais posées

**Décision.** `EditionFormPayload.images` porte trois identifiants d'objet. Ce module les **accepte et ne les pose pas** : le rattachement d'un fichier est une écriture dans `media.attachments`, schéma d'un autre module. La réponse rend les images **déjà rattachées**, résolues.

**Pourquoi ce n'est pas un manque mais une frontière.** L'écran A10 le dit déjà — « la bannière d'une édition ne se téléverse pas encore », et il l'affiche plutôt que d'offrir un bouton inerte. Ce qui change ici, c'est la formulation : ce n'est pas « pas encore fait », c'est « pas à nous ». Le téléversement, la vérification du format exigé par rôle (`media.attachable_roles`) et les variantes appartiennent à B6.

**Ce que le module fait quand même** : il ignore les trois identifiants sans les refuser. Refuser casserait l'enregistrement d'une édition depuis un écran qui les envoie déjà ; les poser romprait la frontière. **Inscrit comme obligation de B6** : le jour où le module Média expose le rattachement, l'écran l'appellera avant d'enregistrer l'édition.

**Alternatives écartées.**
- *Écrire dans `media.attachments`* — rompt la frontière pour trois colonnes.
- *Refuser la charge utile* — casse un écran livré.
- *Émettre un événement pour que Média rattache* — un événement de domaine annonce un **changement d'état**, pas une intention ; et personne ne le consommerait avant B6.

---

## R18 — Ce que les tests doivent tenir, et le test qui frappe les trente-sept routes

**Décision.** Les quatre obligations du principe X ont chacune leur test nommé, plus quatre que la conception impose :

| Test | Ce qu'il tient |
|---|---|
| `toutes_les_routes_repondent.rs` | les **37 routes** sur la vraie application, intergiciels compris — la leçon de B2, où trois routes sur vingt et une étaient muettes |
| `perimetre_edition_url_forgee.rs` | refus par périmètre sur **chaque** route paramétrée, y compris les six qui remontent par un enfant (R2) |
| `perimetre_vide_refuse.rs` | aucun droit → refus explicite, **jamais** une liste vide |
| `sigle_obligatoire_avec_pavillon.rs` | les quatre chemins d'écriture de R1, y compris la bascule en pavillon et le retrait du sigle |
| `contraintes_edition_traduites.rs` | les six contraintes nommées d'une édition, chacune sur son champ |
| `contraintes_appel_traduites.rs` | les six contraintes nommées d'un appel, chacune sur son champ |
| `critere_porteur_de_notes.rs` | la suppression refusée, et **les notes toujours là après le refus** (R9) |
| `canal_par_defaut_unique.rs` | poser un second défaut retire le premier, y compris sur deux écritures concurrentes (R6) |
| `detachement_compte_avant.rs` | le chiffre annoncé égale le chiffre réel, pour la journée, le fil, la salle et le lieu (R8) |
| `jours_civils_dans_le_fuseau.rs` | une édition à Belém commence le bon jour — le décalage de R5 |
| `publication_bloquee_puis_publiee.rs` | refus tant qu'un point bloquant subsiste, puis publication, **un seul** événement, republication inoffensive (R10) |
| `outbox_evenements_du_module.rs` | les six événements attendus, et **aucun** pour ce qui n'en émet pas |

**Le semis de chaque test crée son édition** (R19), sa série étant déjà là.

**Alternatives écartées.**
- *Un test par route* — trente-sept fichiers pour une garantie que le test de couverture donne en un.
- *Se fier au typage pour les contraintes* — les contraintes vivent dans PostgreSQL ; seul un aller-retour réel prouve la traduction.

---

## R19 — Le semis donne quatre séries et un canal, mais aucune édition

**Décision.** Les tests créent leur propre édition ; ils **ne créent ni série ni canal général**.

`900_seed.sql` § 4 sème quatre séries — climat, biodiversité, désertification, rendez-vous de l'IFDD —, et § 4 bis un **canal général de la plateforme**, sans édition, actif et par défaut. Aucune édition n'est semée.

**Deux conséquences à ne pas manquer :**

1. Un test qui poserait un canal par défaut d'édition et vérifierait « il n'y en a qu'un » doit compter **dans le bon groupe** : le canal général est le défaut du groupe « sans édition » et n'entre pas en concurrence (R6).
2. Un test qui compte les séries doit s'attendre à quatre, pas à zéro. C'est le rappel de l'écart n° 86 de B2 : *le semis donne plus que prévu, et l'ignorer coûte une demi-heure.*

---

## R20 — Ce que ce module ne fait pas : ni cache, ni pagination, ni Valkey

**Décision.** Aucune pagination, aucun cache, aucun usage de Valkey.

**Pourquoi c'est défendable ici et ne le sera pas partout.** La volumétrie est connue et petite : quelques dizaines d'éditions au total sur la vie de la plateforme, une douzaine de journées par édition, une poignée de fils, de lieux, de salles et de canaux, un appel et six critères. La liste du back-office tient sur un écran ; la page publique lit une vue prête à l'emploi. Ajouter une pagination que le contrat du front ne demande pas, c'est ajouter une surface et un cas limite pour rien.

**Un cache serait faux la moitié du temps** : le détail porte six onglets dont les décomptes doivent être justes après **chaque** écriture — c'est la raison même pour laquelle une écriture d'onglet rend la composition entière (FR-024).

**Valkey reste inutilisé**, comme en B1 et B2. Il est déclaré dans l'environnement local et attend un usage qui le justifie.
