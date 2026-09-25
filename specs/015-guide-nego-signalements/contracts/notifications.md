# Contrat — notifications et courriels

## Événements émis par `negotiation` (outbox, `platform.emit_event()` via `kernel::events::emit`)

`negotiation` calcule les destinataires (ses fonctions SQL) et compose l'avis ; chaque événement porte
la charge commune `notification` (R8) :

```text
notification: { type_code, recipients: [uuid], title: {fr, en}, body: {fr, en}, link_path,
                subject: {schema: 'negotiation', table, id}, group_key | null, replace, variables }
```

| `event_type` | Émis par | Destinataires | Lien |
|---|---|---|---|
| `negotiation.meeting.changed` | l'import (`start`, `venue`, annulation) | `change_recipients(meeting)` | `/guide-nego/negociations/<id>` |
| `negotiation.report.published` | le travail de publication, changement | `change_recipients(meeting)` | idem |
| `negotiation.network_meeting.published` | le travail de publication, réunion non annoncée | `network_recipients(network_meeting)` | `/guide-nego/negociations/reseau/<id>` |
| `negotiation.report.decided` | publication ou refus | l'autrice | `/guide-nego/negociations/signalements` |

Aucun événement n'est émis quand la liste est vide. **Rien n'est émis ni posé à la validation** : seul
le travail de publication (R3) émet et pose les courriels, et seulement s'il pose `published_at`.
Texte : il commence par l'état (« Déplacée — », « Annulée — », « Non annoncée — »), finit par
« Sessions de négociation » (FR-028) ; « signalé par le réseau » pour les signalements ; aucune
autrice. `group_key = <type>:<id>:<jour>`, `replace: true` ; décision sans clé.

## Consommateur `engagement.notifications` — une branche générique

Pour tout événement dont la charge porte `notification` et dont `type_code` est un type actif :
un avis `in_app` par destinataire (après `canal_autorise`), avec le titre, le corps, le lien, le sujet,
la clé et l'option `replace` reçus. **Aucune règle ni table de `negotiation` n'y figure.** Les
branches existantes de `programme` ne changent pas.

`GET /notifications` gagne `?module=<module_code>` (filtre sur `notification_types.module_code`).

## Courriel — `negotiation.session_change_email`

- Posé par `negotiation`, dans la transaction qui émet `meeting.changed` (import) ou par le travail de
  publication : **un travail par destinataire**, clé `email:<cible>:<tranche de 10 min>:<personne>`,
  `run_at` = fin de la tranche (R9).
- À l'exécution : relit la cible et **les seuls signalements publiés**, vérifie l'accord de la personne
  (`identity.current_consents`, `guide_nego_notifications`, aucune ligne = allumé), envoie un courriel
  (langue de la personne, heures avec le fuseau de la COP, lien vers la fiche).
- Gabarits dans `negotiation/src/mail.rs`, FR/EN, sujet qui commence par l'état.

## Application

- Cloche dans l'emplacement `action` de `GnEntete`, compteur jaune (`unread_count`), cible 48 px,
  libellé « Notifications, N non lues ».
- `pages/guide-nego/notifications.vue` — `GET /notifications?module=negotiation&limit=50` (filtre générique par origine), groupé par jour dans le fuseau de
  la COP, non lue en 600 + carré jaune, « Tout marquer comme lu » (`POST /notifications/read`), un
  toucher marque lu (file) puis ouvre `link_path`. Garde `notifications`, heure de lecture, vide,
  hors connexion.
- Client : `frontend/app/composables/api/notifications.ts` (nouveau), une ligne de montage dans
  `useApi.ts`.
