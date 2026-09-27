# Contrat — l'application

## Routes et composants

| Route / lieu | Maquette | Rôle |
|---|---|---|
| `pages/guide-nego/francophonie/index.vue` (remplace `francophonie.vue`) | 10 · 1a, 1c | en-tête, `GnSegmente` « Réunions · Pavillon » (état dans `?section=`), titre = nom complet de la section, puis `GnSectionReunions` ou `GnSectionPavillon` |
| `components/guide-nego/francophonie/GnSectionReunions.vue` | 10 · 1a | liste, états, pied de liste |
| `components/guide-nego/francophonie/GnSectionPavillon.vue` | — | **état vide seulement** ; l'étape 5 ne réécrit que lui |
| `pages/guide-nego/francophonie/reunions/[id].vue` | 10 · 1b | fiche ; inscription ; lien visio selon R6 |
| `components/guide-nego/francophonie/GnLigneReunion.vue` | 10 · 1a | origine, jour, heures, titre, lieu, une marque, étiquette Pavillon, accès limité |
| `components/guide-nego/GnEtiquettePavillon.vue` | 01 · 4 decies | « Se tient aussi au Pavillon », ouvre `?section=pavillon` (étape 5 : l'activité) |
| `components/guide-nego/journee/GnJourneeLigneSessions.vue` · `GnJourneeLigneReunions.vue` · `GnJourneeLignePavillon.vue` (vide) | 02 · 07 | bloc « Aujourd'hui, vos trois agendas » |

## Garde et file

| Clé | Contenu |
|---|---|
| `reunions:<slug>` | `FrancophoneMeetings` |
| `mes-inscriptions-reunions` | `MyMeetingRegistrations` (liens visio compris) — **le lien quitte la garde dès l'intention de désinscription** ; clé ajoutée à la liste d'effacement de `useGnSession` (déconnexion) et effacée au retrait d'accès |

| Geste | Intention | Rejeu / refus |
|---|---|---|
| S'inscrire / rejoindre la liste d'attente | `PUT …/meeting-registrations/{id}` `{client_ref}` (**neuf à chaque geste**), clé `inscription-reunion:<id>` | `200` ; `409` → abandon et message clair (R5) |
| Se désinscrire | `DELETE …`, même clé (la dernière intention gagne) | idempotent |

## Règles pures — `utils/guide-nego/reunions.ts`

État affiché (priorité Annulée, Terminée — `end_at` passé —, Inscrite, Liste d'attente, Complet, Prévue ;
pas d'« En cours ») ; tri ; réunions du jour et prochaine (fuseau de la COP) ; libellé du bouton (« M'inscrire »,
« Inscrite », « Rejoindre la liste d'attente », « Liste d'attente — position N », « Inscriptions closes »).

## Textes

`pages/guide-nego.francophonie.json` (étendu), `guide-nego.francophonie-reunion.json` ;
`components/gn-ligne-reunion.json`, `gn-etiquette-pavillon.json`, `gn-journee-lignes.json`. Natures et
publics depuis la base.
