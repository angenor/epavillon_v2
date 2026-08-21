# Contrat — Événements

**Fonctionnalité** : Sessions (B5) · **Date** : 2026-08-21

> Ce module est le premier du dépôt à **consommer** un événement de domaine, et l'un des rares à n'en **émettre aucun**. Les deux faits sont liés : la base émet déjà tout ce qu'il y aurait à annoncer.

---

## 1. Ce que le service émet : **rien**

Zéro appel à `kernel::events::emit`. `crates/contracts/src/programme.rs` n'est pas modifié, et **son absence de charge utile nouvelle est la décision**, pas un oubli.

C'est le piège de B1 (`anonymize_person()`), de B2 (`merge_organizations()`) et de B4 (`tg_guard_proposal_status()`), à l'identique — et cette fois il y en a **deux**.

---

## 2. Ce que la base émet — vérifié dans le corps des deux fonctions

### `tg_sessions_emit_events()` — `AFTER INSERT OR UPDATE OF status, starts_at, ends_at`

| Type d'événement | Quand |
|---|---|
| `programme.session.created` | à l'insertion — donc **une fois par séance née d'une acceptation** |
| `programme.session.planned` · `.scheduled` · `.live` · `.completed` · `.postponed` · `.cancelled` | à chaque changement d'état |
| `programme.session.rescheduled` | quand le début change sans que l'état change |

Charge utile : `event_id`, `title`, `starts_at`, `ends_at`, `timezone`, `format`, et `previous_starts_at` sur une mise à jour.

**Deux silences importants, et ils sont dans le corps de la fonction** :

- Le corps sort immédiatement (`RETURN NULL`) si **ni l'état ni le début** n'ont changé. Une séance simplement rendue publique, ou dont seule la fin bouge, n'émet donc rien.
- Le déclencheur ne se réveille pas sur `room_id` : **un changement de salle n'émet rien** (écart n° 118). Une séance déplacée du stand vers une salle virtuelle ne prévient aucun inscrit. Consigné, non corrigé — le corriger demanderait de modifier le SQL, et B6 dira s'il lui faut ce signal.

### `tg_registrations_emit_events()` — `AFTER INSERT OR UPDATE OF status`

| Type d'événement | Quand |
|---|---|
| `programme.registration.created` | à l'inscription |
| `programme.registration.registered` · `.waitlisted` · `.cancelled` · `.attended` · `.no_show` | à chaque changement d'état — **y compris une promotion depuis la liste d'attente** |

Charge utile : `session_id`, `person_id`, `status`, `locale`.

**Conséquence directe pour ce jalon** : la promotion depuis la liste d'attente produit d'elle-même l'avis que la personne attend. Le service n'a rien à ajouter (R20).

---

## 3. Ce que le module consomme : **une** annonce

### `event.programme.published` → publication effective des séances

C'est l'autre moitié de la publication. B3 contrôle, estampille l'édition et **annonce** ; B5 reçoit et rend publiques les séances désignées. Le contrat est fixé depuis le 20/08 aux points bloqués et dans `contracts/src/event.rs`.

**Charge utile reçue** — `ProgrammePublished` :

| Champ | Usage |
|---|---|
| `event_id` | l'édition visée |
| `published_at` | **la date à poser**, celle de l'émetteur — jamais l'instant du traitement |
| `selection.statuses` | les états retenus — `planned` et `scheduled` |
| `selection.only_unpublished` | ne viser que les séances pas encore publiques |
| `published_count` | ce que l'émetteur a compté ; **lu par le test, pas par le service** |

**Les cinq règles du consommateur**, toutes tenues :

1. **Garde de rejeu** — portée par le noyau : le relais appelle `claim(consommateur, événement)` **avant** `handle()`, et n'appelle pas le consommateur une seconde fois. Aucun code d'idempotence à écrire ici (R13).
2. **Le prédicat porté, et pas un autre** — édition, états de `selection.statuses`, `published_at IS NULL` si `only_unpublished`.
3. **Il n'écrit pas `event.events.programme_published_at`** — déjà posée par l'émetteur, et écrire hors de son schéma dans un module métier est interdit.
4. **Une seconde livraison ne publie rien de plus** — conséquence de la règle 1, mesurée et non supposée.
5. **La date posée est celle de l'annonce.**

**Ce que le consommateur écrit** — un seul ordre, deux colonnes :

```sql
UPDATE programme.sessions
   SET published_at = $published_at,
       -- Une séance publiée est « programmée ». Le modèle nomme l'état
       -- (« programmé et publié »), le front colore `planned` comme l'état de
       -- travail, et les données simulées font de même.
       status = CASE WHEN status = 'planned' THEN 'scheduled'::programme.session_status
                     ELSE status END
 WHERE event_id = $event_id
   AND status::text = ANY($statuses)
   AND ($only_unpublished = false OR published_at IS NULL)
```

**Le changement d'état est un effet voulu, et le déclencheur trie lui-même** : `status` figure dans la liste `SET`, donc `tg_sessions_emit_events()` se réveille pour chaque ligne — mais son corps sort pour celles dont l'état n'a pas bougé. Une édition de quarante séances « pressenties » émet donc **quarante** `programme.session.scheduled`, et une republication n'en émet aucun. Ces quarante annonces sont exactement le signal dont B6 a besoin pour planifier les rappels.

**Contexte d'écriture** : le relais ouvre sa transaction avec `RequestContext::background("outbox")`. La publication est auditée comme une écriture de fond, sans acteur nommé — ce qui est exact, personne ne publie une séance à la main.

---

## 4. Le seul écart possible entre l'annonce et l'effet, nommé d'avance

L'émetteur compte à l'instant T, sous l'instantané de **sa** transaction ; le consommateur applique le prédicat à T + ε.

Entre les deux, **aucune route de ce jalon** ne change l'état d'une séance ni ne la dépublie. La seule écriture capable de faire diverger les deux nombres est la **naissance d'une séance** : un dossier retenu dans cet intervalle produit une séance « pressentie » et non publiée, que le prédicat attrapera.

**L'effet peut donc dépasser l'annonce, jamais l'inverse.** Le symptôme est bénin — une séance rendue publique quelques millisecondes plus tôt que prévu, sur une édition qu'on venait de publier. Il est **mesuré par un test de bout en bout** (SC-016) et consigné, jamais supposé nul.

---

## 5. Ce que ce module ne consomme pas, et pourquoi

| Événement | Décision |
|---|---|
| `programme.proposal.accepted` | **Non consommé.** La naissance des séances est **synchrone**, dans la transaction de l'acceptation (R3) : le planificateur doit avoir quelque chose à placer au moment où l'équipe regarde son écran |
| `event.edition.updated` (`period_changed`, `timezone_changed`) | **Non consommé.** Déplacer les dates d'une édition ne déplace pas les séances déjà arbitrées, exactement comme corriger un dossier ne déplace pas sa séance. C'est un geste de l'IFDD, pas un effet de bord |
| `org.organization.merged` | **Non consommé.** Le registre `org.organization_references` déclare déjà les trois colonnes d'organisation de ce module ; la fusion les réaffecte elle-même |

---

## 6. Récapitulatif

| | Nombre |
|---|---|
| Événements **émis par le service** | **0** |
| Types émis par la **base** sur ce périmètre | 8 pour les séances, 6 pour les inscriptions |
| Événements **consommés** | **1** — `event.programme.published` |
| Travaux différés déclarés | **0** |

**Le module est le premier consommateur d'outbox du dépôt.** La machinerie existe dans le noyau depuis B1 et n'avait jamais servi ailleurs que pour la télémétrie ; `ConsumerRegistry` gagne `register_all()`, par symétrie avec `JobRegistry`, et `worker/main.rs` gagne une ligne.
