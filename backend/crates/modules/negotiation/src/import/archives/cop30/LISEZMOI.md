# Jeu archivé — COP30 (Belém, 2025)

**Source** : calendrier de conférence de la CCNUCC, `https://unfccc.int/unfccc-conference-calendar/12286`
(la page `https://unfccc.int/cop30/schedule` en porte l'identifiant). Pris au navigateur le 25/09/2026.

**Conditions d'utilisation** : le site de la CCNUCC réserve toute reproduction à l'accord écrit du
secrétariat. **Cet accord est en cours de demande.** Le dépôt étant public, le jeu est borné à ce que
les tests et la recette exigent : 47 réunions des 17 et 18 novembre 2025 sur 1 529. La capture
complète reste hors du dépôt (`.sources-ccnucc/`, ignoré par Git).

**Forme** : chaque entrée est recopiée telle quelle, clés et structure intactes — heures naïves en
avance d'une heure sur l'heure locale, fautes de frappe et espaces doubles des titres compris.

| Fichier | Contenu |
|---|---|
| `lecture-1.json` | 44 réunions que l'import retient (négociations, plénières, événements mandatés, consultations de la présidence, coordinations — dont EIG, LMDC, GRULAC, Groupe africain, et YOUNGO qu'aucun groupe ne reconnaît), et 3 qu'il écarte (conférence de presse, événement de la présidence, événement parallèle) |
| `lecture-2.json` | `lecture-1` et exactement quatre écarts : `654214` avancée d'une heure, `654010` changée de salle, `654364` disparue, `699001` nouvelle |
| `illisible.json` | JSON tronqué : un contenu que la source aurait rendu mal formé |
| `injoignable` | Nom réservé : le lecteur répond « source injoignable » |

**Reconstitué** (la COP30 n'en offre pas ces jours-là) :

- `654423` porte le préfixe `** POSTPONED ** ` et `654417` le préfixe `CANCELLED - `, écrits comme
  la source les écrivait à la COP29 (`643003`, `643074`) ;
- dans `lecture-2.json`, l'avance de `654214`, la salle de `654010` et toute l'entrée `699001`
  (copie de `654006` à 18 h, renvoyée au programme de la COP30 faute de fiche réelle).
