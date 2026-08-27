# Contrat — événements de domaine et travaux différés

**Module** : Direct + Tableaux de bord (B9) · **Date** : 2026-08-27 · [plan.md](../plan.md) · [research.md](../research.md)

---

## 1. Le module émet zéro événement, et c'est une lecture, pas un oubli

**Vérifié dans le corps des fonctions** (`080_live.sql` § 6), pas déduit d'un précédent :

| Fonction appelée | Événement émis | Charge utile | Par qui |
|---|---|---|---|
| `live.publish_incident(id)` | `live.incident.published` | `{ scope, severity, message }` | **la base**, dans la transaction de l'appelant |
| `live.unpublish_incident(id, motif)` | `live.incident.resolved` | `{ reason }` | **la base**, dans la transaction de l'appelant |

**Décision** : ni le service ni le repo n'appellent `platform.emit_event()`. Le crate `live` **n'émet rien**.

**Pourquoi le dire aussi fort.** C'est le piège n° 1 des six modules précédents — B3, B4 et B5 l'ont chacun rencontré, et la spécification B6 l'a nommé « deux fois dans le même jalon ». Un service qui émettrait « pour faire comme les autres » produirait **deux** lignes d'outbox par publication, donc **deux** réveils de chaque consommateur futur, donc deux effets de bord le jour où quelqu'un en branchera un.

**Ce qui le tient** : un test compte les lignes de `platform.outbox_events` avant et après une publication, et exige **exactement une**. Le jour où un `emit_event` est ajouté, le compte double et le test casse.

**Le contexte d'écriture est une condition de justesse, pas une trace.** `live.publish_incident()` pose `published_by` depuis `platform.current_actor_id()`. Une écriture qui contournerait `Db::write()` produirait un `published_by` **nul, sans erreur** — et le back-office afficherait « publié par — ». Un second test le vérifie **sur la valeur de la colonne**, pas sur l'audit.

**Le crate `analytics` n'émet rien non plus**, et n'a rien à émettre : il mesure, il ne change aucun état.

---

## 2. Aucun consommateur, et pourquoi

Le modèle prévoit qu'un abonné de l'outbox demande un rafraîchissement — « appelée par un abonné de l'outbox (après une décision de comité, une fin de session…) ou par la planification périodique », dit le commentaire d'`analytics.enqueue_refresh()`.

**Décision : ce jalon ne branche aucun consommateur** (R28).

**Pourquoi** : l'effet serait **invisible**. Les chiffres se rafraîchiraient un peu plus tôt, et rien à l'écran ne permettrait de distinguer un consommateur qui marche d'un consommateur qui ne se réveille jamais. Un mécanisme qu'on ne peut pas éprouver est un mécanisme dont on ne saura pas qu'il est cassé — c'est exactement le défaut qu'a coûté, en B6, un consommateur écrit d'après un commentaire et jamais réveillé (écart n° 126).

**Et l'écart est borné** : l'anti-rebond du modèle rend l'écart au plus égal à l'intervalle, soit un quart d'heure.

**Ce qui rendrait la décision fausse**, et qu'il faudra alors reprendre : un intervalle allongé au-delà de l'heure, ou une demande explicite de fraîcheur immédiate après une décision de comité. Le point est inscrit au fichier du module.

**Les deux événements émis n'ont donc, à ce jour, aucun consommateur.** Ce n'est pas un défaut : l'outbox les garde, et le relais les publie dans le vide — c'est le comportement normal d'un journal d'événements dont personne ne s'est encore abonné.

---

## 3. Un travail différé, et un seul

| | |
|---|---|
| Tâche | `analytics.refresh_all` |
| File | `analytics` |
| Priorité | **200** — posée par la fonction du modèle. Un rafraîchissement analytique ne passe jamais devant une confirmation d'inscription |
| Tentatives | 3 — posées par la fonction |
| Charge utile | `{ concurrently: true, tranche: "…" }` — posée par la fonction |
| Porte un secret ? | **non** : sa charge utile est la seule matière de diagnostic d'un rafraîchissement mort |

**La file devient écoutée du seul fait que le gestionnaire la déclare.** `JobRegistry::queues()` est construite à partir des files que les gestionnaires nomment, et `platform.claim_jobs()` filtre strictement : un travail déposé dans une file inécoutée s'empile **sans erreur, sans trace, et sans que rien ne l'exécute jamais**. Le commentaire de `worker/src/jobs.rs` nomme déjà « live » et « analytics » — c'est ici que la promesse devient vraie pour la seconde.

### 3.1 Ce que le gestionnaire fait, dans cet ordre

1. `analytics.refresh_all(true)` — **sur le pool, hors transaction d'écriture** : la fonction journalise elle-même, n'écrit dans aucune table auditée, et l'envelopper tiendrait une connexion d'écriture et ses verrous pour rien (R8).
2. Compte les vues en échec dans le résultat, et les **journalise en avertissement** — sans rendre d'erreur : un tableau de bord partiellement à jour vaut mieux qu'un tableau de bord entièrement périmé parce qu'une agrégation a fauté. C'est la décision du modèle, pas la nôtre.
3. Replanifie la chaîne par `analytics.enqueue_refresh(true, intervalle, anti_rebond)`.

**Le mode concurrent depuis une fonction et depuis un bloc transactionnel a été mesuré**, pas supposé — les huit vues, huit succès, dans les deux cas. Le contraire aurait journalisé huit échecs **sans lever**, l'exception étant avalée vue par vue, et le tableau de bord aurait vieilli en silence pendant que le worker croyait travailler.

### 3.2 L'anti-rebond, et le piège qu'il tend

La clé d'unicité est `refresh_all:<tranche>`, la tranche étant `clock_timestamp()` arrondie au pas d'anti-rebond. Le conflit de `platform.jobs` porte sur `(task, idempotency_key)` **quel que soit l'état du travail**, `cancelled` excepté : **un travail déjà réussi bloque une nouvelle mise en file de la même tranche.**

| Réglage | Valeur | Contrainte |
|---|---|---|
| `ANALYTICS_REFRESH_INTERVAL` | 15 min | **doit dépasser la fenêtre d'anti-rebond**, sinon la chaîne se dédoublonne contre elle-même et **s'arrête sans erreur ni trace** |
| `ANALYTICS_REFRESH_DEBOUNCE` | 5 min | l'anti-rebond passé à la fonction |

Le contrôle est fait **au démarrage** et refuse la configuration, comme `EVENT_CALL_AUTOCLOSE_INTERVAL` refuse déjà zéro. Un réglage qui casse la chaîne en silence ne doit pas pouvoir être posé.

**Le plafond vient du modèle** : l'indicateur `analytique_perimee` de `v_operational_health` passe en `attention` à **120 minutes** et en `critique` à **1440**. Quinze minutes laissent la marge d'un rattrapage sans allumer l'alerte ; deux heures l'allumeraient à chaque cycle.

### 3.3 L'armement au démarrage

`analytics::jobs::refresh::planifier(&mut tx, moment)` est appelée par `armer_les_recurrents`, comme les six chaînes déjà en place. Elle **réarme** la chaîne si sa dernière occurrence est morte avant d'avoir posé la suivante.

**Dix redémarrages dans la même tranche n'arment pas dix rafraîchissements** : c'est l'anti-rebond qui le garantit, et un test le vérifie en appelant `planifier` deux fois.

---

## 4. Récapitulatif

| | Émis | Consommés | Travaux |
|---|---|---|---|
| `live` | **0** par le code, 2 par la base | 0 | 0 |
| `analytics` | 0 | 0 | **1** |

C'est le module le plus discret du dépôt sur l'outbox, et c'est cohérent : l'un publie des textes que la base annonce déjà, l'autre ne fait que mesurer.
