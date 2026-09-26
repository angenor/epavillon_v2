# Contrat — l'application : écrans, garde et file

Réutiliser 3a — la fiche, la liste, « Mon agenda » **reçoivent** l'encart et le repère ; aucune copie.

## Écrans

| Route / lieu | Maquette | Ce qui change |
|---|---|---|
| `negociations/[id].vue` | 08 1a, 1e ; 09 1a–1c | encart violet (`GnEncartSignalement`) sous l'état, « Votre signalement — envoyé à … » ; bouton « Signaler un changement » → `GnFeuilleSignaler` (motifs, précision, envoi) |
| `negociations/index.vue` | 07 ; 09 1d | lignes des réunions non annoncées ; repère « Signalé » ; lien « Signaler une réunion non annoncée » en bas de la liste → `negociations/non-annoncee.vue` |
| `negociations/agenda.vue` | 09 3a | repère « Signalé » ; réunions non annoncées suivies |
| `negociations/reseau/[id].vue` | 07 (ligne), 08 | fiche d'une réunion non annoncée : quoi, où, quand, thématique, « Non annoncée — signalée par le réseau, validée à … », ajout à l'agenda ; aucune section « Source officielle » |
| `negociations/signalements.vue` | 09 2a | « Mes signalements » — ouvert depuis le message d'envoi et le profil |
| `validation/signalements.vue` | 11 1a, 1f | file, « Valider » (« Validé. Affiché dans une minute au plus. ») + « Annuler » six secondes, feuille « Ne pas retenir », « Retirer » ; visible si `can_validate_reports` ; entrée depuis Ressources |
| `notifications.vue` | 02 · 10 | centre ; la cloche dans l'en-tête de « Ma journée » et des onglets |
| `ressources/reglages.vue` | 02 · 11 | « Notifications par thématique » (une ligne par thématique suivie, `GnInterrupteur`) ; lien « Mes signalements » |
| `ressources/a-propos.vue` | 02 · 12 | l'interrupteur « Notifications » (courriel), sa phrase |

`GnLigneSession` gagne une prop `signale` (repère losange + « Signalé » + libellé complet pour les
lecteurs d'écran, FR-017) et sait rendre une réunion non annoncée (« Non annoncée — signalée par le
réseau, validée à HH:MM », sans EN ni accès).

## Garde et file

| Clé de garde | Contenu |
|---|---|
| `sessions:<slug>` (3a) | gagne `network_reports` et `network_meetings` |
| `mes-signalements` | `MyReports` |
| `notifications` | les 50 dernières et `unread_count` |
| `reglage-notifications` | `{ email }` |

| Geste | Intention | Rejeu |
|---|---|---|
| Signaler | `POST /negotiation/reports`, clé `signalement:<client_ref>` | `200` sur rejeu ; `409` doublon → abandon dit à l'écran |
| Marquer lu | `POST /notifications/read {ids: [id]}`, **une intention par notification** (`lu:<id>`) | idempotent |
| Thématiques de notification | `PUT /negotiation/me/themes/notifications` | idempotent |
| Accord courriel | `PUT /negotiation/me/notifications` | idempotent |

Le signalement en file paraît aussitôt dans « Mes signalements » comme « Envoyé » (état local
« en attente d'envoi » distinct, texte « Envoyé — partira au retour du réseau »).

**La validation n'entre jamais dans la file** (R4).

## Textes

`pages/guide-nego.negociations-fiche.json` (+ signaler), `guide-nego.negociations.json` (+ non annoncée),
`guide-nego.negociations-agenda.json` (+ repère), `guide-nego.signalements.json`,
`guide-nego.non-annoncee.json`, `guide-nego.validation.json`, `guide-nego.notifications.json`,
`guide-nego.reglages.json`, `guide-nego.a-propos.json` ; `components/gn-encart-signalement.json`,
`gn-feuille-signaler.json`, `gn-cloche.json`. Motifs de signalement et de refus : textes d'interface
(listes fermées) ; thématiques depuis la base.
