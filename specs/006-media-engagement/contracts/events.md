# Contrat — Événements de domaine

**Fonctionnalité** : Média + Engagement (B6) · **Date** : 2026-08-21

> Ce module est **le plus gros consommateur d'outbox du dépôt**, et il n'émet presque rien. C'est le contraire de B3, et c'est délibéré : tout ce qu'il pourrait annoncer, la base l'annonce déjà.

---

## 1. Ce que la base émet SEULE, et que le service n'émet pas

**Le piège n° 1 des cinq modules précédents, et il se présente ici deux fois.**

| Événement | Émis par | Quand |
|---|---|---|
| `media.asset.uploaded` | `media.tg_enqueue_processing` (`050` § 3) | à chaque insertion dans `media.assets` |
| `media.asset.purge_scheduled` | `media.schedule_asset_purge()` (`050` § 6) | à chaque programmation de purge |
| `engagement.reminders.scheduled` | `engagement.schedule_session_reminders()` (`110` § 6) | à chaque matérialisation, avec la règle et le nombre créé |

**Le service n'appelle `platform.emit_event()` pour aucun des trois.** Le même déclencheur met aussi en file `media.process_asset`, et la même fonction met en file un travail par rappel : le service **n'enfile pas non plus**.

**Le test qui l'attrape** : compter les lignes d'`outbox_events` et de `platform.jobs` après un dépôt et après une matérialisation. Un décompte, jamais une relecture de code — c'est la règle établie depuis B4.

---

## 2. Ce que le service émet

**Deux événements, et pas un de plus.**

| Événement | Agrégat | Charge utile | Pourquoi il existe |
|---|---|---|---|
| `media.asset.purged` | `media` / `asset` | `bucket`, `object_key`, `byte_size`, `rendition_bytes`, `owner_organization_id` | La disparition **effective** d'un objet du stockage n'est annoncée par personne : `schedule_asset_purge()` annonce l'**intention**, jamais l'exécution. Sans cette annonce, rien ne peut réagir à une perte définitive |
| `engagement.email.suppressed` | `engagement` / `email_suppression` | `email` (haché), `reason` | Une adresse qui sort du circuit est une information d'exploitation : elle explique pourquoi une personne cesse de recevoir ses avis. **L'adresse n'y figure pas en clair** — l'outbox est relayée et tracée, et une adresse électronique est une donnée personnelle |

**Aucun autre.** En particulier, ni le rattachement, ni le détachement, ni l'écriture d'une règle de rappel, ni la publication d'un modèle : aucun autre module n'a d'effet à en tirer, et un événement sans consommateur est du bruit dans une table qu'on relit.

---

## 3. Ce que le service consomme

**Deux consommateurs, nommés une fois pour toutes** — les noms entrent dans `platform.inbox_events`, et les renommer ferait rejouer tout l'historique.

### `engagement.reminders`

| Écoute | Fait |
|---|---|
| `programme.registration.created` | lit `payload->>'status'`. Si le statut donne droit à un rappel — ni `cancelled`, ni `waitlisted` —, matérialise |
| `programme.registration.registered` · `.attended` | matérialise, **et réactive** les lignes annulées encore à venir (R21) |
| `programme.registration.cancelled` · `.no_show` · `.waitlisted` | annule les rappels encore à traiter de cette personne, avec leur motif |
| `programme.session.created` · `.planned` · `.scheduled` | matérialise pour tous les inscrits |
| `programme.session.rescheduled` | **décale** les instants d'envoi des lignes encore à traiter sur le nouveau créneau, puis matérialise ce qui manque |
| `programme.session.cancelled` · `.postponed` | annule les rappels encore à traiter, motif `session_cancelled` |
| `programme.session.live` · `.completed` | rien — il est trop tard pour rappeler |

**Le branchement se fait sur le STATUT porté par la charge utile, jamais sur `programme.registration.confirmed`, qui n'existe pas** (écart n° 126, R4). C'est la règle la plus importante de ce contrat.

### `engagement.notifications`

| Écoute | Fait |
|---|---|
| **tout** | cherche un type de notification actif **dont le code égale le type de l'événement** ; s'il n'y en a pas, ne fait rien |

**Pourquoi écouter tout** : `notification_types.code` suit « la même grammaire que `outbox_events.event_type` » — le modèle le dit dans son propre commentaire. La correspondance est donc une **donnée**, et ajouter une notification reste un INSERT, comme le modèle le promet.

**Quatre types sont réellement servis dans ce jalon** (R23), les seuls dont les destinataires et les variables se résolvent aujourd'hui :

| Type de notification | Destinataires | Variables |
|---|---|---|
| `programme.registration.confirmed` | l'inscrit | `prenom`, `titre_session`, `date_session`, `lien_participation` |
| `programme.session.cancelled` | les inscrits non annulés | `titre_session`, `motif` |
| `programme.session.rescheduled` | les inscrits non annulés | `titre_session`, `nouvelle_date` |
| `programme.session.reminder` | l'inscrit visé par le rappel | `prenom`, `titre_session`, `date_session`, `delai`, `lien_participation` |

**Les quatorze autres types du catalogue restent déclarés et non consommés**, et le dire est plus honnête que de prétendre les couvrir : le catalogue ne dit pas **qui** reçoit, et cette résolution est du code, type par type. Écart consigné.

**Le rappel de séance ne passe pas par ce consommateur** : il part du travail différé `engagement.send_reminder`, mis en file par la fonction du modèle. Il figure ici parce qu'il partage le même type de notification, le même modèle et la même garde d'envoi.

---

## 4. La garde de rejeu

Portée par le noyau depuis B1 (`platform.inbox_events (consumer, event_id)`), exercée pour la première fois par un module en B5. **Ce module n'écrit aucun code d'idempotence** : `kernel::events::claim()` rend faux pour un événement déjà traité, et le consommateur passe au suivant sans produire d'effet.

**Trois barrières se superposent, et c'est voulu** :

1. la garde de rejeu, qui empêche de retraiter un événement ;
2. `ux_scheduled_reminders_once`, qui empêche de créer deux fois le même rappel — « ce n'est pas une convention de code, c'est une contrainte de base », dit le modèle ;
3. `ux_jobs_idempotency`, qui empêche de mettre deux fois le même envoi en file.

Aucune n'est redondante : la première protège du rejeu d'outbox, la deuxième de deux règles concurrentes, la troisième de deux planifications simultanées.

---

## 5. Les cinq travaux différés

| Tâche | File | Mise en file par | Clé d'idempotence |
|---|---|---|---|
| `media.process_asset` | `media` | le déclencheur du modèle | l'identifiant de l'objet |
| `engagement.send_reminder` | `email` | la fonction du modèle | l'identifiant du rappel |
| `media.purge_assets` | `media` | **elle-même**, et réarmée au démarrage | l'instant de la prochaine occurrence |
| `media.reconcile_quotas` | `media` | **elle-même**, réarmée au démarrage | idem |
| `engagement.ensure_partitions` | `default` | **elle-même**, réarmée au démarrage | idem |

Le réarmage au démarrage est le mécanisme livré en B1 pour la purge des jetons : le worker **ne crée pas** l'occurrence, il rétablit la chaîne au cas où la dernière serait morte avant d'avoir posé la suivante.

**`engagement.send_reminder` porte-t-elle un secret ?** Non : sa charge utile ne contient que des identifiants. Elle ne déclare donc pas `carries_secret()`, et un travail mort garde sa charge utile — seule matière de diagnostic d'un envoi en échec, comme le noyau l'explique.

---

## 6. Ce qui n'est pas un événement, et qui pourrait le paraître

| Ce qu'on pourrait attendre | Pourquoi ce n'en est pas un |
|---|---|
| « Une déclinaison a été fabriquée » | Aucun module n'a d'effet à en tirer. L'écran interroge l'avancement quand il en a besoin |
| « Un quota est presque atteint » | Aucun destinataire n'est défini, et aucun écran n'affiche l'alerte. Le tableau du back-office trie par proximité du plafond, ce qui répond au besoin sans machinerie |
| « Un courriel a été remis » | La trace d'expédition le porte. En faire un événement produirait une ligne d'outbox par courriel remis, pour personne |
| « Une notification a été lue » | C'est une écriture de l'utilisateur sur ses propres données |
