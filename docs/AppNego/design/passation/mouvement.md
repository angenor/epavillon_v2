# Guide Négo — mouvement

Extrait de « 01 — Système », section 7, et des composants qui bougent (sections 4 bis à 5).

## Principe

**Aucune animation n'est nécessaire pour comprendre.** Le mouvement confirme un geste ou situe un écran ; il ne porte jamais seul une information. Une marque d'état, un message, un compteur restent lisibles sans lui.

## Durées

| Jeton | Durée | Ce qui l'emploie |
|---|---|---|
| `bref` | 120 ms | Pression d'un bouton, bascule d'un filtre ou d'une pilule, apparition d'une marque d'état |
| `standard` | 200 ms | Feuille basse, message éphémère, changement d'onglet |
| `long` | 320 ms | Ouverture d'un document, transition entre écrans |
| `arc` | 1 s, linéaire, en boucle | Arc de chargement (24 px, trait 2,5) |
| `squelette` | 1,6 s, ease-in-out, en boucle | Squelette de chargement, opacité 1 → 0,55 → 1 |
| `ephemere` | 6 s | Durée d'affichage du message éphémère ; délai d'« Annuler » après une acceptation (Validation) |

## Courbes

| Jeton | Valeur | Ce qui l'emploie |
|---|---|---|
| `sortie` | `cubic-bezier(0.2, 0, 0, 1)` | Ce qui apparaît ou entre : feuille basse, message éphémère, marque d'état. Part vite, se pose doucement. |
| `standard` | `cubic-bezier(0.4, 0, 0.2, 1)` | Ce qui se déplace ou change de taille : bascule de filtre, barre de progression, transition d'écran. |

## Ce qui s'anime

- **Feuille basse** : monte du bas en `standard` / `sortie` ; le voile (gris à 60 %) apparaît en même temps.
- **Message éphémère** : entre en `standard` / `sortie`, reste 6 s, sort en `standard`.
- **Marque d'état** qui apparaît ou change (Envoyé → Validé, Proposé → Classé) : `bref` / `sortie`.
- **Filtre, pilule, case, interrupteur, onglet** : bascule en `bref` / `standard`.
- **Barre de progression, jauge** : la largeur suit en `standard` ; le nombre écrit change sans animation.
- **Transition d'écran, ouverture d'un document, dépliage de la barre de lecture** : `long` / `standard`.
- **Arc de chargement** : rotation 1 s linéaire, continue. **Squelette** : pulsation 1,6 s.
- **Barre de lecture** : un toucher au centre la bascule (`standard`) ; le défilement la replie.

## Ce qui ne bouge pas

Le fond, l'en-tête, la barre d'onglets. Aucun défilement automatique, aucun parallaxe, aucun rebond. Les listes ne s'animent pas à l'entrée : elles sont là. L'aplat jaune de la session en cours ne clignote pas ; le point de « En cours » est fixe.

## « Réduire les animations »

Quand le système le demande (`prefers-reduced-motion: reduce`, réglage d'accessibilité Android) :

- toutes les durées tombent à 0 — les feuilles, messages et écrans apparaissent en place ;
- l'arc de chargement et le squelette deviennent fixes (l'arc reste dessiné, immobile ; le squelette garde son opacité 1) ;
- rien n'est perdu : chaque information animée est aussi écrite.

`theme.css` porte ces règles : les variables `--gn-duree-*` passent à 0 et `animation: none` s'applique sous la requête média.

## Ce que la maquette ne montre pas

Les transitions ne sont pas jouées dans la maquette HTML (écrans fixes) ; seuls l'arc (`gn-spin`) et le squelette (`gn-pulse`) sont animés en CSS. Les directions d'entrée (par le bas pour la feuille, latérale pour un écran) suivent l'usage Android et restent à confirmer sur appareil.
