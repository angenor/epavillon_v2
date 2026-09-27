# Contrat — l'application

| Lieu | Maquette | Rôle |
|---|---|---|
| `components/guide-nego/francophonie/GnSectionPavillon.vue` (réécrit) | 10 · 1c | bloc de lieu, bande des jours de toute l'édition (passés compris), « Aujourd'hui » + compteur, « Hier — rediffusions », « Les jours suivants », états chargement / vide / programme non publié / hors connexion |
| `components/guide-nego/francophonie/GnBlocLieu.vue` | 01 · 4 decies | nom, adresse, « Voir sur le plan du site » (`map_url`) |
| `components/guide-nego/francophonie/GnLigneActivite.vue` | 10 · 1c | origine, heure, titre, état de l'activité, marque d'inscription ou « Rediffusion · N min », chevron |
| `pages/guide-nego/francophonie/pavillon/[slug].vue` | 10 · 1d | surtitre, titre, marques, bloc de lieu, heure avec jour et fuseau, organisateurs, langue, intervenantes, rediffusion, inscription ; **pas d'« Ajouter à mon agenda »** |
| `components/guide-nego/francophonie/GnFormulaireInscription.vue` | — | champs du modèle seulement, pays prérempli, consentement d'une donnée sensible |
| `components/guide-nego/journee/GnJourneeLignePavillon.vue` (réécrit) | 02 · 07 | activités du jour, sinon « Rien aujourd'hui. Prochaine : … » |
| `GnEtiquettePavillon` | — | reçoit `vers` = la fiche de l'activité liée, depuis la ligne et la fiche d'une réunion |

**États** : de l'activité — Prévue, En cours (aplat jaune sur l'heure), Terminée, Annulée, Reportée ;
d'inscription — Inscrite, Liste d'attente (position), Complet, « Rediffusion · N min », Sans inscription.

## Garde et file

| Clé | Contenu |
|---|---|
| `pavillon:<slug>` | l'édition (`/schedule`) et le lieu (`/venues`) |
| `pavillon-activite:<édition>:<slug>` | détail et formulaire |
| `mes-inscriptions-pavillon` | `/registrations/mine` |

Intention `inscription-pavillon:<session_id>` (R3). Client : `composables/api/pavillon.ts` + une ligne
de montage dans `useApi.ts`. Composables `useGnPavillon`, `useGnInscriptionsPavillon`. Règles pures
`utils/guide-nego/pavillon.ts` (+ tests) : jour, veille, jours suivants, états, marque, prochaine, ligne de
« Ma journée », **aucun filtre de thématique**.

Textes : `components/gn-section-pavillon.json` (étendu), `gn-journee-ligne-pavillon.json` (étendu),
`gn-ligne-activite.json`, `gn-bloc-lieu.json`, `gn-formulaire-inscription.json`,
`pages/guide-nego.pavillon-activite.json`.
