# ADR-023 — Direction « Nuit » : sombre, le temps au centre

**Statut** : accepté — 05/10/2026. Remplace [ADR-018](018-direction-typographique-quatre-onglets.md) pour l'apparence ; garde ses quatre onglets.

## Contexte

L'application construite sur la direction « Typographique » a été jugée trop plate par le commanditaire : « la première page ressemble à un site web ». Quatre directions lui ont été montrées sur les mêmes écrans (Douce, Nuit, Mosaïque, Billet). Il a retenu **Nuit**, et demandé de l'appliquer à toute l'application au pixel près.

## Décision

- **La maquette de [design/nuit/](../design/nuit/LISEZMOI.md) fait foi**, couleurs, polices et mesures comprises. Seule la disposition s'ajuste à ce que l'application porte.
- **Thème unique, sombre** : fond `#0B1018`, accent vert anis `#C8F169`, attention `#FFC857`, alerte `#FF8E72`. Le choix Clair / Sombre / Système disparaît.
- **Sora** pour les titres et les chiffres, **Manrope** pour le texte, toutes deux embarquées. Atkinson Hyperlegible Next est retirée.
- **Le temps au centre de l'accueil** : un compte à rebours (minutes avant la prochaine session, ou jours avant l'ouverture) et un fil du jour qui montre les trois agendas sur une même ligne horaire, sans les confondre.
- **Formes arrondies** (6 à 24 px) au lieu de la direction carrée ; barre d'onglets de 88 px, onglet actif à l'accent.
- Les quatre onglets, le lexique accessible de partout et les règles de contenu (trois agendas, source officielle, validation humaine) ne changent pas.

## Conséquences

- La couleur de charte de l'IFDD (vert et jaune foncés) cesse d'être l'identité de l'application ; [05-design.md](../05-design.md) § Couleurs et § Typographie sont dépassés sur ces points.
- **Plein soleil** : un fond sombre se lit moins bien sous un soleil fort. Le commanditaire l'a accepté en choisissant le thème unique ; les contrastes restent vérifiés par `scripts/guide-nego-contrastes.mjs`.
- Tous les écrans et composants de l'application sont repris ; la planche des composants (`/guide-nego/composants`) suit.
