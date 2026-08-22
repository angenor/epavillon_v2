# Phase 1 — Le modèle existant, et ce que le code en fait

**Module** : Média + Engagement (B6) · **Date** : 2026-08-21 · [plan.md](plan.md) · [research.md](research.md)

Le modèle est la source de vérité. Ce document ne le redécrit pas : il dit **ce que le code lit, ce qu'il écrit, ce qu'il ne touche pas**, et **quelles règles vivent dans le service parce que la base ne les porte pas**.

---

## 1. Ce que le module Média écrit, et ce qu'il laisse à la base

### `media.assets`

| Colonne | Écrite par | Note |
|---|---|---|
| `bucket`, `object_key` | le service | clé composée `<année>/<mois>/<uuid>/<nom-normalisé>.<ext>`, convention du modèle |
| `checksum_sha256` | le service | **calculée pendant la réception**, jamais reçue du client sans être recalculée |
| `mime_type`, `byte_size`, `original_filename` | le service | le poids est celui réellement reçu, jamais celui annoncé |
| `width`, `height`, `duration_seconds` | **le travail différé** | nuls à l'insertion ; c'est le relevé qui les pose |
| `owner_person_id`, `owner_organization_id` | le service | au moins l'un des deux (`ck_assets_owner_present`) |
| `visibility` | le service | déduit du rôle visé ; `public` par défaut |
| `status` | le service à l'insertion (`uploaded`), **le travail différé** ensuite | seul `ready` est servi |
| `scan_verdict`, `scan_engine`, `scanned_at`, `scan_details` | **le travail différé** | `none` est un moteur déclaré, pas une absence (R13) |
| `alt_text` | le service | **exigé pour une image** (R9) |
| `caption`, `credit`, `license_code` | le service | facultatifs |
| `deleted_at`, `deleted_by`, `purge_after` | `media.schedule_asset_purge()` | jamais posées à la main |
| `purged_at` | **le travail de purge** | après retrait effectif du stockage |

**Ce que le service ne fait jamais** : mettre en file `media.process_asset`, émettre `media.asset.uploaded`, débiter un quota, refuser un dépassement de quota à l'écriture. Les quatre appartiennent à la base (R3, R5).

### `media.renditions`

Écrite **uniquement** par le travail différé. `variant_code` et `format` viennent de la configuration ; `status`, `byte_size` et `generated_at` sont posés ensemble, `ck_renditions_ready_shape` l'exigeant. Le trigger de comptage d'espace suit.

### `media.attachments`

Écrite par le service. `is_exclusive` est **posée par le trigger** depuis la règle : le service ne la renseigne pas. `alt_text_override` porte le texte propre à un usage (FR-040).

### `media.attachable_roles`

**Lue seulement.** Elle donne au service ce qu'il annonce à l'écran (FR-044) et ce qu'il vérifie avant d'appeler la base. Elle ne dit **pas** qui a le droit : c'est l'écart n° 127, et la garde vit dans `domain/guards.rs` (R15).

### `media.storage_quotas`

**Lue** pour la capacité et le tableau du back-office ; **écrite** seulement sur `max_bytes` et `max_files`, par la route de relèvement (FR-112). Les compteurs `used_*` appartiennent aux triggers et à la réconciliation.

---

## 2. Ce que le module Engagement écrit, et ce qu'il laisse à la base

### `engagement.reminder_rules`

Écrite par le service : portée, décalages (**en minutes à la traversée**, R19), canaux, type, modèle, activité. Les quatre refus du modèle — décalages invalides, portée invalide, unicité par édition, unicité par séance — sont **traduits**, jamais revérifiés.

### `engagement.scheduled_reminders`

Écrite par `engagement.schedule_session_reminders()`, que le service **appelle**. Le service écrit lui-même trois choses que la fonction ne fait pas :

1. **la réactivation** des lignes annulées dont l'instant est encore devant (R21) ;
2. **le décalage** des instants d'envoi lors d'un report ;
3. **l'annulation** avec son motif — séance annulée, inscription annulée, règle coupée.

`status`, `sent_at` et `skip_reason` sont posés par le travail d'envoi.

### `engagement.notifications`

Écrite par le consommateur d'outbox et par la diffusion d'annonce. Le **regroupement** est une écriture du service : incrémenter `group_count` sur la notification non lue portant la même clé plutôt qu'en créer une seconde — le modèle le décrit, `ux_notifications_group` le rend sûr, mais **aucune fonction ne le fait** (FR-092).

### `engagement.notification_preferences`

Écrite par la personne elle-même. Une préférence sur un type **critique** est enregistrée telle quelle : c'est `is_channel_enabled()` qui l'ignore, et la lecture le **dit** à l'écran (FR-095) plutôt que de refuser l'écriture.

### `engagement.message_templates` et `template_versions`

Écrites par le service. `current_version` est l'unique geste de publication, et il est réversible. `body_html` est **assaini à l'écriture** (R26).

### `engagement.email_messages`

Écrite par la garde d'envoi (R24), pour **tout** envoi de la plateforme. Mise à jour par le webhook de délivrabilité. Aucune clé étrangère : la trace survit à l'anonymisation d'une personne (FR-102).

### `engagement.email_suppressions`

Écrite par le webhook (rebond dur, plainte) et par le back-office. Lue **avant chaque envoi**, par la garde.

### Ce que le module ne touche pas

`comments`, `commentable_subjects`, `reactions`, `conversations`, `conversation_participants`, `direct_messages`, `connection_requests`, `blocks`, `newsletter_lists`, `newsletter_subscriptions`, `newsletter_campaigns` — hors périmètre (H9), et aucune ligne de code ne les nomme.

---

## 3. La seule modification du modèle

### `engagement.session_reminder_schedule(p_session_id uuid)` — fonction de lecture, ajoutée à `110_engagement.sql` § 6

| Colonne rendue | Ce qu'elle porte |
|---|---|
| `offset_minutes` | le décalage, en minutes — la forme du contrat du front |
| `channel` | le canal, en texte |
| `scheduled_for` | l'instant d'envoi du groupe |
| `status` | l'état consolidé, selon la règle de R18 |
| `recipient_count` | le **nombre** de destinataires |
| `skip_reason` | le motif dominant, quand le groupe est écarté ou annulé |
| `sent_at` | le dernier envoi effectif du groupe |

Ordonnée du décalage le plus lointain au plus proche.

**Aucune table, aucune colonne, aucun type.** Elle existe parce que l'agrégat a **deux** lecteurs et doit être écrite une fois (R17, FR-052). C'est le précédent exact de `media.attached_image()`.

**Ce qu'elle ne rend jamais** : un identifiant de personne, un nom, une adresse. La contrainte de FR-048 est portée par la **signature** de la fonction, pas par la discipline de ses appelants.

---

## 4. Les règles qui vivent dans le service, et pourquoi

Aucune n'est une réimplémentation : la base ne les porte pas du tout.

| Règle | Pourquoi le service | Écart |
|---|---|---|
| Le texte alternatif est exigé au dépôt d'une image | la base l'exige à `ready`, c'est-à-dire trop tard : l'objet resterait bloqué | n° 129 |
| Le droit de rattacher est le droit d'écrire sur l'entité porteuse | aucune permission `media.*` n'existe | n° 127 |
| Un objet encore rattaché ne se supprime pas | la déduplication traverse les propriétaires | n° 128 |
| L'empreinte est vérifiée pendant la réception | la base ne voit pas le flux | — |
| Les déclinaisons et leur nombre | « la liste vit dans la configuration du worker », dit le modèle | n° 135 |
| Le verdict d'analyse et le moteur | aucun moteur n'est branché en développement | — |
| La réactivation des rappels d'une inscription reprise | `ON CONFLICT DO NOTHING` ne ressuscite pas une ligne annulée | cas limite n° 15 |
| Le décalage des instants lors d'un report | la fonction insère, elle ne déplace pas | — |
| L'annulation des rappels et leur motif | aucune fonction ne l'écrit | — |
| Le regroupement des notifications | le modèle le décrit, aucune fonction ne le fait | — |
| Le rendu d'un gabarit et le refus d'une variable manquante | « appartient au worker », dit le modèle en tête de fichier | — |
| L'assainissement du corps d'un modèle | rien ne l'assainit | n° 32, porté ici |
| La résolution des destinataires d'un avis | le catalogue ne dit pas qui reçoit | consigné |
| La préparation des partitions mensuelles | le worker de maintenance annoncé n'existait pas | n° 137 |
| L'état consolidé d'un groupe de rappels | aucune agrégation n'existe | n° 34 |

---

## 5. Les lectures hors schéma, et leur justification

La règle de frontière de B2 : *un module lit hors de son schéma quand la question porte sur ses propres entités, il n'y écrit jamais, et il n'appelle jamais un autre module.*

### Module Média

| Lecture | Question posée |
|---|---|
| `org.memberships`, `org.organizations` | qui peut poser un fichier sur cette fiche, et quelle organisation le possède |
| `event.events` | à quelle édition appartient l'entité visée, pour vérifier le périmètre |
| `programme.proposals` | à quelle organisation appartient le dossier visé |
| `identity.people` | l'objet vise-t-il la personne connectée |
| `content.highlights` | à quelle édition se rattache le contenu visé |
| `platform.settings` | le point d'accès public, par `media.object_url()` |

### Module Engagement

| Lecture | Question posée |
|---|---|
| `programme.sessions`, `programme.registrations` | quelle séance, quel créneau, quels destinataires — les entités que ses rappels servent |
| `event.events` | quelle édition porte la règle, et le périmètre d'administration |
| `org.memberships` | l'organisation qui anime la séance a-t-elle le droit de lire son calendrier |
| `identity.people` | la langue préférée et l'adresse du destinataire |
| `platform.modules` | le module d'un type de notification |

**Aucune écriture hors schéma dans aucun des deux modules.** C'est vérifié par le contrôle mécanique du [quickstart](quickstart.md), et c'est une amélioration sur B3, B4 et B5, qui en avaient chacun.

---

## 6. Les états, et qui les fait bouger

### Un objet stocké

```
uploaded ──▶ scanning ──▶ processing ──▶ ready
    │            │             │
    │            ▼             ▼
    └──────▶ quarantined    failed
```

`uploaded` est posé par le service ; **tout le reste par le travail différé**. `quarantined` et `failed` sont terminaux. Seul `ready` est servi, et `ready` exige un verdict d'analyse acceptable **et** un texte alternatif si c'est une image — deux contraintes de la base, jamais revérifiées.

### Un rappel programmé

```
pending ──▶ queued ──▶ sent
   │           │
   ├───────────┴──▶ skipped     (adresse supprimée, canal coupé, séance annulée)
   └──────────────▶ cancelled   (inscription annulée, règle coupée)
```

`cancelled` **revient à `pending`** lorsqu'une inscription est reprise et que l'instant est encore devant (R21) — c'est la seule transition en arrière, et elle existe parce que la clé d'unicité interdit de recréer la ligne.

### Une trace d'expédition

```
queued ──▶ sent ──▶ delivered
              │
              ├──▶ bounced      (webhook)
              ├──▶ complained   (webhook)
              └──▶ failed       (tentatives épuisées)
```

`bounced` en rebond dur et `complained` inscrivent l'adresse sur la liste de suppression.

---

## 7. Les formes ajoutées au contrat du front

Le front porte déjà `Asset`, `Rendition`, `AssetSources`, `AttachedImage`, `AttachableRoleRule`, `Attachment`, `StorageQuota`, `ReminderRule`, `ScheduledReminder`, `ReminderSlot`. **Aucune n'est renégociée.** Cinq formes sont **ajoutées**, sans en modifier aucune :

| Forme | Ce qu'elle porte | Pour |
|---|---|---|
| `UploadVerdict` | accepté ou refusé, avec le code, le champ, et l'objet existant s'il y en a un | l'annonce préalable (FR-010) |
| `ApplicableReminderRule` | la règle, son **origine** (`session` ou `event`) et l'identifiant dont elle vient ; nulle si aucune ne s'applique | FR-074, FR-076 |
| `NotificationFeed` | les notifications, le nombre de non lues, et le fait qu'un type soit non désactivable | FR-093, FR-095 |
| `TemplateDetail` | le modèle, ses révisions, la version servie, les variables promises par son type | FR-080 à FR-084 |
| `QuotaRow` | organisation, plafond, consommation, fichiers, part consommée | FR-111 |

`AttachableRoleRule` du front **ne porte pas** `expected_aspect_ratio` ni sa tolérance, que le modèle déclare. L'API les rend en plus : un champ ajouté ne casse rien, et sans eux l'écran ne peut pas annoncer la forme attendue avant le refus. Inscrit aux obligations de B7.
