# Contrat — l'application : écrans, garde et file

Tout se bâtit sur ce que 0a à 0c ont livré : `useGnLecture`, la garde IndexedDB `guide-nego`, la file
d'écritures `useGnFile`, `useGnConnexion`, `useGnThematiques`. **Aucun magasin nouveau** : les clés
suffisent, la base reste en version 3.

## Routes

| Route | Écran de maquette |
|---|---|
| `/guide-nego/negociations` | 07 — la liste d'un jour (remplace la page vide actuelle) |
| `/guide-nego/negociations/[id]` | 08 — la fiche |
| `/guide-nego/negociations/agenda` | 09 — « Mon agenda » |
| `/guide-nego/thematiques` | + le bloc des groupes (écran existant de 0c) |
| `/guide-nego/` | « Ma journée » : **le seul bloc « prochaine-session »** change |

La page « Mon agenda » s'ouvre depuis l'en-tête de l'onglet et depuis le bloc de « Ma journée ».

## Clés de garde (`lectures`)

| Clé | Contenu | Lue par |
|---|---|---|
| `sessions:<edition>` | `OfficialSessions` avec `lu_a` | liste, fiche, agenda, « Ma journée » |
| `mes-groupes` | `MyGroups` | liste (filtre), écran des thématiques |
| `mon-agenda` | `MyAgenda` | agenda, fiche, « Ma journée » |

- **Coupé** : la réponse coupée **remplace** la copie gardée — FR-039 ; la liste ancienne ne survit
  pas à une coupure lue.
- L'édition visée est celle que sert déjà la coquille (`utils/guide-nego/edition.ts`) ; elle gagne
  `slug`, `timezone` et `city`, qui manquaient (commentaire de `jourLisible`).

## Écritures en file

| Geste | Intention | Rejeu |
|---|---|---|
| Choisir ses groupes | `PUT /negotiation/me/groups` avec `If-Match` | `412` abandonne, comme les thématiques |
| Ajouter / changer le rappel | `PUT /negotiation/me/agenda/{id}` | idempotent |
| Retirer | `DELETE /negotiation/me/agenda/{id}` | idempotent |

La garde `mon-agenda` est **réécrite aussitôt** (l'ajout se voit sans réseau), puis relue au retour
du réseau. Deux intentions sur une même session : la dernière gagne dans la file.

## Règles pures (`utils/guide-nego/`, testées par `test:guide-nego`)

- `sessions.ts` — état affiché (`prevue`, `en-cours`, `deplacee`, `annulee`, `terminee`) à partir de
  `status`, `previous`, `cancelled` et de l'instant ; jours de la bande dans le fuseau de la COP ;
  jour choisi à l'ouverture (FR-003) ; tri ; filtre « Mes thématiques » (thèmes suivis, sans thème,
  coordinations des groupes cochés ou toutes si aucun) ; état vide et prochain créneau (FR-011).
- `agenda.ts` — chevauchements (une annulée n'en cause ni n'en porte) ; prochaine session de
  « Ma journée » (agenda, sinon thématiques) ; rappel dû (`maintenant ∈ [début − 15 min, début[`, jamais
  sur une annulée).

## Le rappel (FR-034)

`useGnRappel`, monté dans la mise en page de Guide Négo : chaque minute et au retour au premier plan,
il cherche une session de l'agenda au rappel armé dont le rappel est dû, et affiche **un bandeau en
tête d'écran** — titre, heure avec fuseau, salle — refermable, montré une fois par session
(`gn.rappel.vu.<id>` dans `localStorage`). **Rien d'autre** : ni son, ni vibration, ni notification.

## Le lexique

Toucher le terme anglais du type (`type.term_en`) ouvre `/guide-nego/lexique?terme=<texte anglais>`
(contrat de l'étape 2, [R10](../research.md)). Rien d'autre : ni résolution, ni table côté 3a.

## Textes

`i18n/locales/{fr,en}/pages/guide-nego.negociations.json` (existe, s'étend),
`guide-nego.negociations-fiche.json`, `guide-nego.negociations-agenda.json` ;
`guide-nego.thematiques.json` (+ groupes) ; `components/gn-rappel.json`, `gn-ligne-session.json`,
`gn-bande-jours.json`, `gn-etat-session.json`, `gn-lecture-impossible.json`, `gn-valeur-changee.json`.
Les types, groupes, thématiques et salles viennent de la base.
