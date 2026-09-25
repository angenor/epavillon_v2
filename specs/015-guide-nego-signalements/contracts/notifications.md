# Contrat — notifications et courriels

## Événements émis par `negotiation` (outbox, `kernel::events::emit`)

| `event_type` | Quand | `payload` |
|---|---|---|
| `negotiation.meeting.changed` | l'import constate `start`, `venue` ou une annulation sur une session importée | `{ meeting_id, event_id, fields: ['start'|'venue'|'status'], start_at, venue, cancelled }` |
| `negotiation.report.published` | le travail de publication trouve le signalement de changement toujours validé | `{ report_id, meeting_id, event_id, reason }` |
| `negotiation.network_meeting.published` | idem, réunion non annoncée | `{ network_meeting_id, event_id, theme }` |
| `negotiation.report.decided` | validation publiée (travail) ou refus | `{ report_id, author_id, status, reject_reason }` |

**Rien n'est émis ni posé à la validation** : seul le travail de publication (R3) émet, pose les
courriels, et seulement s'il pose `published_at`.

Constantes dans `backend/crates/contracts/src/negotiation.rs`. Aucun nom d'autrice dans les charges
des trois premiers.

## Consommateur `engagement.notifications` — branches ajoutées

Résolution des destinataires dans `engagement/src/repo/cross.rs`, **par les fonctions SQL**
`negotiation.change_recipients(meeting_id)` et `negotiation.network_recipients(network_meeting_id)` —
jamais une seconde écriture de la règle ; **décision** : l'autrice.

Écriture `in_app` (après `canal_autorise`), `link_path = /guide-nego/negociations/<id>` (réunion non
annoncée : `/guide-nego/negociations/reseau/<id>` ; décision : `/guide-nego/negociations/signalements`),
`group_key = <type>:<id>:<jour>` avec **remplacement** du titre et des variables par l'état final
(option `remplacer` ajoutée à `notifications::ecrire`), aucune clé pour `report.decided` ;
`subject_schema = negotiation`, `subject_table` = `meetings` | `network_meetings` | `session_reports`,
`subject_id`. Texte FR/EN : il commence par l'état (« Déplacée — », « Annulée — »,
« Non annoncée — »), finit par « Sessions de négociation » (FR-028) ; « signalé par le réseau » pour
les deux types de signalement ; aucune autrice.

**Le consommateur actuel fige `subject_schema = programme`** : la branche `negotiation` porte le sien.

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
- `pages/guide-nego/notifications.vue` — `GET /notifications?module=negotiation&limit=50` (filtre ajouté à `engagement`), groupé par jour dans le fuseau de
  la COP, non lue en 600 + carré jaune, « Tout marquer comme lu » (`POST /notifications/read`), un
  toucher marque lu (file) puis ouvre `link_path`. Garde `notifications`, heure de lecture, vide,
  hors connexion.
- Client : `frontend/app/composables/api/notifications.ts` (nouveau), une ligne de montage dans
  `useApi.ts`.
