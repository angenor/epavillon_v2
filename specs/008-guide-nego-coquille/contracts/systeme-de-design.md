# Contrat du système de design — Guide Négo 0a

Référence : `docs/AppNego/design/ecrans/01-systeme.html`, `passation/composants.md`, écarts tranchés de `05-design.md`. Tout vit sous `[data-app="guide-nego"]`.

## Fondations — en entier

| Fichier | Contenu |
|---|---|
| `assets/guide-nego/theme.css` | Charte, nuances sombres, rôles clairs et sombres (les huit manquants compris), typographie, mouvement, « réduire les animations » |
| `assets/guide-nego/mesures.css` | Grille, cibles, rayons, filets, zones sûres, dimensions ; `--gn-picto-taille` |
| `assets/guide-nego/police.css` | Dix `@font-face` |
| `assets/guide-nego/pictogrammes.svg` | 57 symboles `gn-*`, sans `<metadata>` ni `<title>` |

## Composants livrés — préfixe `Gn`, un fichier chacun

| Composant | Variantes et états à montrer |
|---|---|
| `GnBarreOnglets` | 4 et 5 onglets ; actif, pressé, focus ; compteur ; défilement propre quand la place manque. Largeur = libellé + 3 px de chaque côté, 48 au moins, reste réparti |
| `GnEntete` | Principal, secondaire (retour) ; « Aa » repos et ouvert ; ligne de connexion en ligne / hors connexion |
| `GnBandeauConnexion` | Avec et sans heure |
| `GnEtatVide` · `GnEtatErreur` | Avec et sans sortie ; erreur à deux sorties |
| `GnChargement` | Arc ; squelette ; fixes sous « réduire les animations » |
| `GnBouton` | Principal, secondaire, discret, dangereux ; repos, pressé, focus, désactivé, actif ; pleine largeur et deux demis |
| `GnChampRecherche` · `GnChamp` · `GnZoneTexte` | Repos, focus, erreur, désactivé ; compteur « n / 600 » |
| `GnCase` · `GnInterrupteur` · `GnCercle` | Coché ou non, focus, désactivé ; ligne de 56 px cible entière |
| `GnOngletsFiltre` · `GnPilule` · `GnSegmente` | Actif, décochable, à chevron. Onglet de filtre et segmenté à **17** (écarts 6 et 15, qui relèvent le 16 de la maquette) ; la pilule reste à **15**, sa taille d'origine, que l'écart ne vise pas |
| `GnEnteteGroupe` | Avec et sans compteur |
| `GnLigneReglage` | Valeur + chevron ; interrupteur ; pressée ; dernière ligne sans filet |
| `GnMarqueEtat` | Les six états de session et les rôles sémantiques ; pictogramme + mot + couleur |
| `GnEtiquette` | Avec et sans pictogramme |
| `GnFeuilleBasse` | Ouverte, options de 56 px, « Annuler » ; piège à focus, Échap |
| `GnConfirmation` | Simple et dangereuse |
| `GnMessageEphemere` | Avec et sans action ; 6 s ; `role="status"` |
| `GnLigneInformation` | Avec et sans sortie |
| `GnPicto` | Tailles 16, 18, 20, 24, 26, 40 |

Tout autre composant de `composants.md` arrive avec l'étape qui l'emploie. Aucun n'est ébauché.

## Règles communes

- Cible de 48 px ; anneau de focus visible ; nom accessible ; état annoncé (`aria-pressed`, `aria-checked`, `aria-selected`, `aria-current`).
- Un état = pictogramme + mot + couleur. Le jaune : seulement ce qui concerne la personne à l'instant.
- Aucun texte sous 15 px, sauf 13 px pour libellés d'onglets, marque de rôle, compteur, jour de la bande.
- Durées et courbes par variables `--gn-duree-*`, `--gn-courbe-*` ; aucune valeur en dur.
- Aucune chaîne en dur, aucune couleur en dur, aucune classe Tailwind, aucun composant ni jeton du site.
- **Toute règle de style s'écrit sous la borne** : `[data-app="guide-nego"] .gn-… { }`, dans les feuilles comme dans les blocs `<style>` des composants. Pas de `<style scoped>` seul.
- **Rien ne se téléporte dans `<body>`** : feuille basse, boîte de confirmation, message éphémère et voile visent `#gn-portail`, que la mise en page porte **dans** son élément racine. Hors de lui : ni jetons, ni thème. `check-guide-nego` refuse `<Teleport to="body">`.
- **Quand la place manque** (police agrandie, moins de 360 px), la barre d'onglets seule défile ; aucun libellé tronqué ni sur deux lignes ; l'onglet actif est amené dans la vue. À 320 px, quatre onglets ne défilent pas.

## Page des composants — `guide-nego/composants`

Sections dans l'ordre de `01-systeme.html` : 1 Couleurs · 2 Typographie · 3 Mesures · 4 Pictogrammes · 5 Composants · 6 États · 7 Mouvement. Bascule clair / sombre / côte à côte. Un fichier par section sous `components/guide-nego/planche/`, pour qu'une étape ajoute les siens sans toucher aux autres. Les sections A à F et 4 bis à 4 septdecies de la maquette sont des notes de conception : elles ne se reprennent pas.
