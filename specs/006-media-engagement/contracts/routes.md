# Contrat — Routes

**Fonctionnalité** : Média + Engagement (B6) · **Date** : 2026-08-21

> **Ce document ne définit aucune forme de réponse.** Les formes vivent dans `frontend/app/types/` et n'y ont qu'une seule source ; les cinq formes ajoutées sont décrites dans [`../data-model.md`](../data-model.md) § 7. On indique ici, pour chaque route : le verbe, le chemin, l'autorisation exigée, le type de requête et de réponse **par son nom TypeScript**, et la politique de statut.
>
> La documentation OpenAPI est **engendrée depuis le code**. Ce fichier est la carte, pas la documentation.

---

## Préfixe, transport et politique de statut

Rien ne change depuis B1 à B5 : préfixe `/api`, `Accept-Language` sur chaque requête, `X-Request-Id` sur chaque réponse, session par cookies, vérification de l'origine sur toute écriture, en-têtes CORS posés.

**La règle de statut est celle de B1** : un refus **exprimé par le contrat** sort en **200** avec son discriminant ; un refus **non exprimé** sort en statut d'erreur avec un corps d'erreur ([`errors.md`](errors.md)).

**Deux exceptions de transport, propres à ce module :**

1. **`POST /media/assets` est en `multipart/form-data`**, seule route du dépôt à ne pas parler JSON. Les champs de métadonnées précèdent le fichier, ce qui permet de refuser avant d'avoir tout lu. La limite de corps JSON de l'API (un mégaoctet) ne s'y applique pas ; une limite propre, configurable, la remplace.
2. **`POST /internal/mail-events` n'a ni session ni contrôle d'origine** : elle est appelée par le relais d'envoi, sur un autre serveur, et authentifiée par un jeton porteur. **Sans jeton configuré, elle n'est pas montée** et rend 404 (R30).

---

## Les permissions consommées

**Aucune permission `media.*` n'existe** dans le modèle (écart n° 127). Le droit de poser ou de retirer un fichier est **le droit d'écrire sur l'entité qu'il illustre**, résolu par la table de gardes de `domain/guards.rs` (R15) :

| Entité porteuse | Garde |
|---|---|
| `org.organizations` | adhésion active de référent, **ou** `org.organization.manage` |
| `event.events` | `event.event.manage` sur la portée de l'édition, **et** périmètre d'administration |
| `programme.proposals` | adhésion active à l'organisation porteuse, **ou** `programme.proposal.decide` sur l'édition |
| `identity.people` | soi-même, **ou** `identity.person.manage` |
| `content.highlights` | `content.highlight.manage` sur la portée du contenu |
| `publication.articles` | `publication.article.write` — module fermé par drapeau, la garde existe quand même |

Les quatre permissions du module Engagement viennent du modèle :

| Permission | Ce qu'elle garde ici | Portée exigée |
|---|---|---|
| `engagement.reminder.manage` | l'écriture et la coupure des règles de rappel | l'édition visée, ou globale |
| `engagement.template.manage` | les modèles de messages, leurs révisions, leur publication, et la liste de suppression | globale |
| `engagement.notification.broadcast` | la diffusion d'une annonce | globale |
| `engagement.comment.moderate` | **non consommée** — les commentaires sont hors périmètre (H9) | — |

Et deux permissions d'autres modules gardent les lectures d'administration du média :

| Permission | Ce qu'elle garde | Portée |
|---|---|---|
| `org.organization.manage` | le tableau des quotas et le relèvement d'un plafond | globale |
| `identity.person.manage` | rien ici — citée pour la garde des avatars | — |

---

## Les trente-trois routes

### Module Média — le dépôt et les objets (6)

| # | Verbe | Chemin | Requête | Réponse | Notes |
|---|---|---|---|---|---|
| 1 | POST | `/media/assets/precheck` | `UploadDeclaration` | `UploadVerdict` | **N'écrit rien, ne réserve rien.** Rend le verdict que rendrait le dépôt : accepté, type refusé, poids refusé, quota atteint, ou objet existant si une empreinte est fournie. En **200** dans tous les cas — ce sont des réponses, pas des erreurs |
| 2 | POST | `/media/assets` | `multipart` | `Asset` | Le dépôt. Métadonnées puis fichier. Empreinte calculée pendant la réception. Un rôle et une entité porteuse peuvent être joints : le rattachement est alors posé **dans la même transaction** |
| 3 | GET | `/media/assets/{id}` | — | `Asset` **+** `sources` | Description d'un objet et ses déclinaisons prêtes. Un objet non servable rend son état, jamais un 404 : « en traitement » n'est pas « absent » |
| 4 | DELETE | `/media/assets/{id}` | — | `{ scheduled_purge_at }` | Refusée si l'objet est **encore rattaché** (`MEDIA_ASSET_IN_USE`, avec le nombre). Sinon `media.schedule_asset_purge()` |
| 5 | GET | `/media/assets/{id}/status` | — | `AssetProgress` | L'avancement du traitement : état, verdict d'analyse, déclinaisons prêtes sur déclinaisons attendues. Sans elle, un écran ne sait pas distinguer « en cours » de « en échec » (FR-032) |
| 6 | GET | `/media/roles?owner_schema=&owner_table=` | — | `AttachableRoleRule[]` | Les règles de la table blanche pour une entité : libellé, multiplicité, types, poids maximal, **forme attendue et tolérance**. Ce qu'un écran annonce au lieu de le deviner |

### Module Média — les rattachements (4)

| # | Verbe | Chemin | Requête | Réponse | Notes |
|---|---|---|---|---|---|
| 7 | GET | `/media/attachments?owner_schema=&owner_table=&owner_id=&role=` | — | `AttachedImage[]` | Les médias d'une entité, ordonnés par le tri déclaré. `role` facultatif : sans lui, tous les rôles |
| 8 | POST | `/media/attachments` | `AttachmentPayload` | `Attachment` | Ajouter à un rôle **multiple**. Sur un rôle exclusif déjà pourvu : refus explicite (FR-036) |
| 9 | PUT | `/media/attachments` | `AttachmentAssignment[]` | `AttachedImage[]` | **L'écriture de remplacement, en lot et en une transaction** : poser, remplacer, ou retirer si l'objet est nul. C'est elle que le formulaire d'édition appelle pour ses trois déclinaisons (FR-043, obligation de B3) |
| 10 | DELETE | `/media/attachments/{id}` | — | `{ asset_kept: true }` | Détache. **L'objet stocké demeure** — le champ le dit, parce que c'est la question qu'on se pose en lisant la réponse |

### Module Média — le back-office (3)

| # | Verbe | Chemin | Autorisation | Requête | Réponse |
|---|---|---|---|---|---|
| 11 | GET | `/admin/media/orphans?min_age_days=` | `org.organization.manage`, globale | — | `OrphanAsset[]` — du plus lourd au plus léger, variantes comprises |
| 12 | GET | `/admin/media/quotas` | `org.organization.manage`, globale | — | `QuotaRow[]` — triés par proximité du plafond |
| 13 | PUT | `/admin/media/quotas/{organizationId}` | `org.organization.manage`, globale | `{ max_bytes, max_files, note }` | `QuotaRow` |

### Module Engagement — le calendrier des rappels (2)

**Ces deux routes vivent sous `/sessions`, préfixe que `programme` ouvre depuis B5.** Le scope est donc **composé par l'API**, une seule fois, à partir des deux modules (R29). Deux `web::scope` du même préfixe ne se complètent pas.

| # | Verbe | Chemin | Autorisation | Réponse | Notes |
|---|---|---|---|---|---|
| 14 | GET | `/sessions/{id}/reminders` | adhésion active à l'organisation qui anime, **ou** `programme.registration.manage` sur l'édition | `ReminderSlot[]` **+** `has_rule` | **L'écart n° 34.** Une ligne par (décalage, canal) — jamais un nom. `has_rule` distingue « aucune règle » de « tout est parti » (FR-051) |
| 15 | GET | `/sessions/{id}/reminder-rule` | même garde | `ApplicableReminderRule \| null` | La règle **applicable**, avec son **origine** et l'identifiant dont elle vient (FR-074). C'est ce qui rend la non-cumulation vérifiable |

### Module Engagement — les règles de rappel (3)

| # | Verbe | Chemin | Autorisation | Requête | Réponse |
|---|---|---|---|---|---|
| 16 | GET | `/admin/reminder-rules?event_id=` | `engagement.reminder.manage` sur l'édition + périmètre | — | `ReminderRule[]` — celle de l'édition et celles de ses séances |
| 17 | PUT | `/admin/reminder-rules` | idem, sur la portée **visée** | `ReminderRulePayload` | `ReminderRule` — une **liste** de décalages, jamais un décalage seul (FR-070) |
| 18 | DELETE | `/admin/reminder-rules/{id}` | idem | — | `{ cancelled_reminders }` — le nombre de rappels encore à traiter qui ont été annulés (FR-078) |

### Module Engagement — notifications et préférences (5)

| # | Verbe | Chemin | Autorisation | Requête | Réponse |
|---|---|---|---|---|---|
| 19 | GET | `/notifications?unread_only=&limit=&before=` | session | — | `NotificationFeed` — les lignes **et** le nombre de non lues, dans la même réponse |
| 20 | POST | `/notifications/read` | session | `{ ids? }` | `{ marked }` — sans `ids`, toutes |
| 21 | POST | `/notifications/archive` | session | `{ ids }` | `{ archived }` |
| 22 | GET | `/notification-preferences` | session | — | `NotificationPreferenceRow[]` — chaque ligne dit si le type est **non désactivable** (FR-095) |
| 23 | PUT | `/notification-preferences` | session | `{ type_code, channel, is_enabled }[]` | `NotificationPreferenceRow[]` |

### Module Engagement — les modèles de messages (5)

| # | Verbe | Chemin | Autorisation | Requête | Réponse |
|---|---|---|---|---|---|
| 24 | GET | `/admin/message-templates` | `engagement.template.manage` | — | `MessageTemplateRow[]` |
| 25 | GET | `/admin/message-templates/{id}` | idem | — | `TemplateDetail` — révisions, version servie, variables promises par le type |
| 26 | POST | `/admin/message-templates/{id}/versions` | idem | `TemplateVersionPayload` | `TemplateVersion` — **corps assaini à l'écriture** (R26) |
| 27 | POST | `/admin/message-templates/{id}/versions/{version}/publish` | idem | — | `TemplateDetail` — refusée si une variable citée n'est pas promise (FR-083). Republier une révision antérieure est le **retour arrière** |
| 28 | POST | `/admin/message-templates/{id}/preview` | idem | `{ version?, variables }` | `{ fr: RenderedMail, en: RenderedMail }` — **n'envoie rien** |

### Module Engagement — délivrabilité et diffusion (4)

| # | Verbe | Chemin | Autorisation | Requête | Réponse |
|---|---|---|---|---|---|
| 29 | GET | `/admin/email-suppressions?q=` | `engagement.template.manage` | — | `EmailSuppression[]` |
| 30 | POST | `/admin/email-suppressions` | idem | `{ email, reason, detail?, expires_at? }` | `EmailSuppression` |
| 31 | DELETE | `/admin/email-suppressions/{email}` | idem | — | `{ removed }` |
| 32 | POST | `/admin/notifications/broadcast` | `engagement.notification.broadcast`, globale | `{ title, body, link_path?, audience }` | `{ recipients }` — une notification par destinataire, groupée par clé |

### Ingestion (1)

| # | Verbe | Chemin | Autorisation | Requête | Réponse |
|---|---|---|---|---|---|
| 33 | POST | `/internal/mail-events` | jeton porteur, **hors session** | `MailEvent[]` | `{ applied, ignored }` — une annonce déjà vue est **ignorée**, jamais dupliquée (FR-101) |

---

## Ce qui n'est pas exposé, et pourquoi

| Ce qu'on pourrait attendre | Pourquoi c'est absent |
|---|---|
| Une route qui rend l'objet en flux | La base ne stocke pas d'URL parce que le stockage sert les fichiers **lui-même**. Faire relayer chaque image par l'API annulerait tout l'intérêt du modèle |
| `PUT /events/{id}/images` | Elle vivrait sous le préfixe d'un autre module, pour une entité parmi six. La route 8 le fait pour toutes (R16) |
| Un décompte de notifications non lues séparé | Il est dans la réponse de la route 18. Deux appels donneraient deux chiffres mesurés à deux instants — le défaut que B4 a nommé sur les facettes d'une liste |
| Les commentaires, réactions, messages directs, mises en relation, infolettres | Hors périmètre (H9) |
| Le rappel d'échéance d'un appel | Hors périmètre : le périmètre de ses destinataires n'est pas défini (H10) |
| Une reprise de téléversement interrompu | Aucun écran ne la demande, et elle exigerait un état intermédiaire persistant |
