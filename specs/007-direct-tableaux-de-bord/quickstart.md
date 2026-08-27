# Quickstart — comment prouver que B9 marche

**Module** : Direct + Tableaux de bord (B9) · **Date** : 2026-08-27 · [plan.md](plan.md) · [contracts/routes.md](contracts/routes.md)

Ce fichier ne contient aucun code d'implémentation : il dit **ce qu'on lance et ce qu'on doit voir**. Un écran de pilotage et un bandeau public se prouvent sur ce qu'ils **affichent**, pas sur ce qu'ils compilent.

---

## 0. Avant tout — `down -v`, sans exception

Deux lignes de semis ont changé (`analytics.review_alert_days`, l'attribution au rôle `programmer`). **Le schéma n'est chargé qu'au premier démarrage du conteneur** : sans destruction du volume, la base garde l'ancien semis **sans le dire**, et deux choses passeraient au vert pour de mauvaises raisons — le seuil serait lu sur sa valeur de repli, et le contrôle de permission serait vérifié avec un catalogue périmé.

```bash
docker compose -f ops/docker-compose.dev.yml down -v
docker compose -f ops/docker-compose.dev.yml up -d
make wait-db
```

Vérification en une ligne :

```bash
docker exec -i epavillon-postgres psql -U postgres -d epavillon -c \
  "SELECT value FROM platform.settings WHERE key = 'analytics.review_alert_days';
   SELECT count(*) FROM identity.role_permissions
    WHERE role_code = 'programmer' AND permission_code = 'analytics.dashboard.read';"
```

Attendu : `21` et `1`.

---

## 1. La porte de qualité

```bash
make check          # check-db · check-front · check-back
```

Ce qui doit être vrai :

| Contrôle | Attendu |
|---|---|
| `check-db` | 16 schémas, `cross_module_fk_report` **sans ligne non conforme**, `analytics.refresh_all(true)` — **8 vues, 8 succès** |
| `check-back` | `cargo fmt --check`, `clippy -D warnings` **sans un avertissement**, `cargo test --workspace` au vert |
| `check-front` | typecheck, build, **et `check-api-contract` à zéro route en attente** |
| Frontières | `cargo tree -p live` et `cargo tree -p analytics` **sans aucune arête** vers `modules/` |
| Taille | aucun fichier de `backend/` ni de `frontend/` au-dessus de 1000 lignes |

**Le chiffre qui ferme le jalon** :

```bash
node frontend/scripts/check-api-contract.mjs --verbose | tail -3
```

Attendu : `0 route(s) en attente d'API` — contre 8 avant le jalon.

---

## 2. Les neuf routes, sur la vraie application

```bash
cd backend && cargo run -p api      # dans un terminal
cd backend && cargo run -p worker   # dans un autre
```

Les tests d'intégration frappent chacune des neuf routes ; ce point de contrôle-ci sert à voir ce que les tests ne montrent pas — les corps réels, dans la langue négociée.

| # | À vérifier à la main | Ce qu'on doit voir |
|---|---|---|
| 1 | `GET /api/admin/incidents?event_id=<COP31>` | l'édition, son fuseau, **sa ville** (« Belém », accentuée), les lignes dans l'ordre d'action, le poste, les compteurs, les neuf natures, les cibles |
| 2 | `GET /api/admin/incidents/overrun-template?session_id=<X>` | l'activité, son titre **résolu**, son créneau, son édition |
| 3 | `GET /api/admin/incidents/<id>` | la ligne complète, `state` calculé, `target_label` résolu |
| 4-7 | les quatre écritures | **200 dans tous les cas**, y compris les refus — voir § 4 |
| 8 | `GET /api/events/<COP31>/incidents`, **sans cookie** | la liste des actifs, le plus grave en tête, chacun avec son `target_label` |
| 9 | `GET /api/admin/dashboard?event_id=<COP31>` | cinq lignes d'action, l'entonnoir, deux courbes, la santé, les incidents, `refreshed_at` |

---

## 3. La règle métier n° 8, éprouvée et non supposée

**Trois comptes, trois résultats différents.** C'est le contrôle qui compte le plus de ce jalon.

| Compte | Sur les neuf routes | Attendu |
|---|---|---|
| Administratrice **globale** (Bakayoko) | toutes | 200 partout, cinq éditions atteignables |
| **Programmatrice détachée** sur la COP31 (Nko Diop) | `?event_id=<COP31>` | **200** — et c'est exactement ce que l'attribution ajoutée au catalogue rend possible (R10) |
| Le même compte, **URL forgée** vers la COP30 | `?event_id=<COP30>` | **404**, jamais 403, jamais une réponse vide |
| Compte **sans aucun droit d'administration** | n'importe laquelle | **403**, jamais une liste vide |
| Membre du comité **sans `live.incident.publish`** | routes 1 à 3 | **200** — lire n'est pas un privilège |
| Le même | routes 4 à 7 | **200 `{ status: 'forbidden' }`** — pas un 403 : l'écran l'affiche dans son formulaire |

**Avant l'ajout de l'attribution, la deuxième ligne rendait 403.** C'est ce que ce point de contrôle existe pour montrer : refaire tourner ce cas avec le catalogue périmé (§ 0) est la démonstration de l'écart.

**Un compte de démonstration ne peut pas se connecter par l'interface** : Claire Perret, administratrice détachée sur la COP31, porte `mfa_enabled_at` et le second facteur n'est pas implémenté. Le contrôle du périmètre restreint se fait donc avec Fatou Nko Diop — comme le 17/08, et pour la même raison.

---

## 4. Les dix issues d'écriture, une par une

Chacune se produit à la main depuis le formulaire, et **chacune rend 200**.

| Geste | Issue |
|---|---|
| Enregistrer sans publier | `created`, état `draft`, **rien ne s'affiche nulle part** |
| Enregistrer en publiant | `published`, remonte en tête de liste |
| Corriger | `updated` |
| Publier un brouillon depuis la ligne | `published` |
| Retirer avec un motif | `unpublished` — **la ligne reste**, avec son auteur, son instant et son motif |
| Publier à nouveau un message retiré | `published`, et **le retrait est effacé** — instant, auteur, motif |
| Retirer un message jamais publié | `not_published` |
| Message rédigé dans une seule langue | `missing_message`, posé sur le champ |
| Portée changée sans vider la cible | `missing_target` |
| Fin d'affichage avant le début | `invalid_window` |

**Et un onzième, qui n'est pas une issue** : un compte détaché sur la COP31 qui tente de retirer un message de **portée globale** reçoit `forbidden` (D3). Le message reste visible dans sa liste — il doit savoir qu'un bandeau d'entretien couvre son pavillon.

---

## 5. Les cinq portées, sur une base réelle

Le semis ne fournit **aucun incident** : ce contrôle pose les siens, en transaction annulée, comme le 18/08.

```sql
BEGIN;
-- un incident par portée : global, event, event_day, session, organization
-- plus un sixième, publié mais dont la fenêtre est close
SELECT scope, state, target_label FROM live.event_incidents('<COP31>', now());
ROLLBACK;
```

Attendu : **les cinq portées remontent**, la portée `organization` comprise si l'organisation anime une activité de l'édition ; l'ordre est actifs, programmés, brouillons, historique ; la cible est résolue par son **nom** — et une journée sans titre par sa **date**.

Puis, par l'API : `GET /api/admin/incidents?event_id=<COP31>` doit rendre **exactement les mêmes lignes, dans le même ordre**. Une divergence signifierait que le crate a recomposé le balayage au lieu d'appeler la fonction.

**Et sur une seconde édition** : le message de portée `global` doit apparaître **aussi**, celui de portée `session` **non**.

---

## 6. Le poste de direct et son repli

L'édition de démonstration se tenant en novembre 2027, le repli est le cas **normal** aujourd'hui — et c'est ce qui le rend vérifiable.

| Contrôle | Attendu |
|---|---|
| `desk.is_fallback` | **vrai**, et `desk.day` reste **aujourd'hui** dans le fuseau de l'édition |
| `desk.sessions` | les **quatre** prochaines activités, par début croissant |
| Le jour est-il celui de l'édition ? | comparer `desk.day` à `(now() AT TIME ZONE 'America/Belem')::date`. À Belém il est 06:00 quand il est 11:00 à Paris : ce contrôle se fait entre 00:00 et 03:00 heure de Paris pour être concluant, ou en déplaçant l'horloge de la base |
| `active_incident_count` | publier un message de portée `session` sur une activité du poste : le compteur passe à 1 **sans recharger la page ?** non — au rechargement. C'est un compteur, pas un direct |
| `is_streamed` | trois gestes sur une activité non diffusée, quatre sur une activité diffusée |
| `temporal_state` | **comparé à `v_public_schedule`** pour une activité publiée, sur les cinq branches (R22). C'est le test qui tient la duplication assumée |

---

## 7. Le rafraîchissement, et sa chaîne

| Contrôle | Comment | Attendu |
|---|---|---|
| La file est écoutée | démarrer le worker, lire son premier journal | `files = ["analytics", "default", "email", "media"]` — **« analytics » y est**, du seul fait que le gestionnaire la déclare |
| La chaîne s'arme | redémarrer le worker **dix fois** dans la même tranche | `SELECT count(*) FROM platform.jobs WHERE task = 'analytics.refresh_all'` → **1** |
| Le travail s'exécute | attendre l'intervalle, ou poser `run_at = now()` à la main | 8 lignes de plus dans `analytics.refresh_log`, **toutes en succès** |
| Il se replanifie | après l'exécution | un travail `queued` de plus, `run_at` à +15 min |
| L'écran voit la fraîcheur avancer | `GET /api/admin/dashboard` avant et après | `refreshed_at` a avancé |
| **Un échec n'arrête pas les autres** | `ALTER MATERIALIZED VIEW analytics.mv_content_popularity RENAME TO …` puis exécuter | **7 succès, 1 échec**, l'échec journalisé — et `refreshed_at` **ne bouge pas**, puisqu'il prend le maximum sur les succès complets |
| **Le réglage qui casse la chaîne est refusé** | `ANALYTICS_REFRESH_INTERVAL=60s` avec un anti-rebond de 300 s | l'API et le worker **refusent de démarrer**, en nommant le réglage |

Le dernier contrôle est le plus important du paragraphe : c'est le seul défaut du jalon qui serait **entièrement silencieux**.

---

## 8. Le tableau de bord dit-il la vérité ?

| Contrôle | Attendu |
|---|---|
| **Un seul aller-retour** | l'onglet réseau du navigateur sur `/admin` : **une** requête `/admin/dashboard`, plus `/health` si l'écran rafraîchit sa zone 3 |
| **Un seul instant** | l'entonnoir et les lignes d'action s'accordent — 40 ouverts, 35 déposés, 16 retenus, et les décomptes d'alerte cohérents avec la liste des propositions |
| Les cinq familles se déclenchent | sur la COP31 : dossiers sans évaluation, revues en retard, chevauchements, messages actifs, doublons — chacune avec ses **trois exemples nommés** et son lien |
| **Une famille vide n'émet pas de ligne** | sur une édition sans appel : deux lignes seulement — l'incident global et les doublons, les deux qui n'appartiennent à aucune édition |
| **Le premier bloc reste lisible vide** | neutraliser temporairement la composition : encart en retrait, coche verte, « Rien n'attend l'équipe ». Ni bordure rouge, ni zone béante |
| `null` n'est pas zéro | sur une édition dont aucun dossier n'est tranché : le taux d'acceptation est **absent**, pas « 0 % » |
| Le seuil vient de la base | passer `analytics.review_alert_days` à `1` : la famille « dossiers sans évaluation » **maigrit**, sans redéploiement |
| Les séries sont continues | les jours vides sont **présents** avec zéro ; aucun trou n'est rebouché par l'écran |
| La santé est rendue par son **code** | `/en/admin` : les libellés d'indicateur sont traduits par leur code, jamais « Courriels en rebond ou signalés (7 jours) » en anglais |
| Les couleurs viennent de la base | chaque barre de thématique porte le `color_hex` de son terme |

---

## 9. Le bandeau atteint le public

C'est ce qui fait passer le jalon de « publié » à « lu ».

1. Publier, depuis le back-office, un message de portée `session` sur une activité de la COP31.
2. Ouvrir `/programmations?edition=cop31-belem-2027` **dans une session de navigateur neuve, sans cookie**.

| Attendu | |
|---|---|
| Le bandeau s'affiche | et **nomme son activité** — « Atelier de négociation », par `target_label` |
| Le plus grave en tête | ordre rendu par la fonction, non recomposé |
| Trois au plus | le reste replié en « +N » |
| Un message refermable | ne réapparaît pas pendant la visite ; un non refermable reste |
| Retirer le message | rechargement : le bandeau **disparaît** |
| Une fenêtre close | rien ne s'affiche |

**Ce qui n'est pas au jalon, et qu'il ne faut pas chercher** : le bandeau sur l'accueil, et la page publique d'une activité — **qui n'existe pas** (R26). C'est ce constat qui a déplacé l'exposition sur la page des programmations.

---

## 10. Les frontières, mécaniquement

| Contrôle | Commande | Attendu |
|---|---|---|
| Aucune arête entre modules | `cargo tree -p live \| grep -c 'modules/'` | 0 — idem pour `analytics` |
| **Aucune écriture hors schéma** | le test dédié parcourt `src/` des deux crates | aucune occurrence d'`INSERT INTO`, `UPDATE` ou `DELETE FROM` visant un autre schéma — `platform` et `reference` compris |
| `repo/cross/` liste exactement ce qui traverse | relecture | `live` : event, programme, org. `analytics` : event, programme, org, live. **Pas de `platform.rs` ni de `reference.rs`** : le noyau partagé n'est pas une frontière (principe III) |
| Zéro événement émis par le code | test qui compte l'outbox | **exactement une** ligne par publication |
| `published_by` non nul | test sur la valeur de la colonne | l'identifiant de la personne, jamais `NULL` |

---

## 11. L'accessibilité et le responsive, sur les deux écrans

Les deux écrans ont été vérifiés au navigateur les 17 et 18/08 et **ne sont pas réécrits**. Ce qui doit être repassé, c'est ce que le branchement peut casser :

| Contrôle | Attendu |
|---|---|
| Aucun défilement horizontal à 375 px | `scrollWidth === clientWidth === 375`, sur `/admin`, `/admin/incidents` et `/programmations` |
| Le bandeau de données d'exemple | **absent** des deux écrans, API configurée |
| Thème sombre | bascule **à chaud**, sans rechargement — c'est ce test qui avait révélé deux défauts le 18/08 |
| Anglais | `/en/admin` et `/en/admin/incidents` : aucune clé brute |
| Les quatre états | chargement, vide, erreur, **accès refusé** — le dernier distinct du vide : « vous n'avez pas ce droit » n'est pas « il n'y a rien à voir » |
| Cibles tactiles | 44 px sur les lignes d'action et les gestes du poste de direct |
| Le bandeau public | lu par un lecteur d'écran : le message, sa gravité, et **le nom de son activité** |

---

## 12. Ce que ce jalon ne prouve pas

| Point | Pourquoi |
|---|---|
| Les réunions de visioconférence et les diffusions | hors périmètre — les quatre cinquièmes du schéma `live` (R4) |
| Les deux indicateurs de santé « visio » | la vue les calcule, l'écran les affiche, **aucun écran ne les règle** : leur valeur est zéro tant qu'aucune réunion n'existe |
| `live.active_incidents(session)` | non appelée : elle viendra avec la page publique d'une activité |
| Un rafraîchissement déclenché par un événement de domaine | délibérément absent — l'effet serait invisible et l'écart au plus d'un quart d'heure (R28) |
