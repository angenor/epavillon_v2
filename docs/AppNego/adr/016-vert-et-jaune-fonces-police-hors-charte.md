# ADR-016 — Un design propre : vert et jaune foncés, police hors charte

**Statut** : accepté — 18/09/2026

## Contexte

Guide Négo se lit en plein soleil, d'une main, sur un téléphone de milieu de gamme. Le porteur veut une application agréable et belle, aux couleurs de l'IFDD, et **sans rapport avec le design de l'ePavillon**. Les polices de la charte, Helvetica et NeueMaverick, ne conviennent pas au mobile.

## Décision

- Guide Négo a **sa propre identité visuelle**. Elle ne reprend du site ni le cyan, ni la mise en page, ni les composants.
- Dominante **vert foncé**, seconde **jaune foncé** : nuances foncées des couleurs de la charte de l'IFDD, contrastes mesurés. Le reste de la charte sert aux états.
- Une police **hors charte**, à licence ouverte, embarquée, choisie pour sa lisibilité sur mobile.
- La direction artistique se choisit parmi **trois propositions** de la maquette.

## Conséquences

- Les règles de lisibilité en plein soleil sont chiffrées — [05-design](../05-design.md).
- Dans le code, le système de design de Guide Négo vit dans son dossier, borné à sa mise en page : il ne touche à aucun jeton du site.
- Seuls les écrans de back-office, qui s'ajoutent à celui de l'ePavillon, en gardent l'apparence.
- L'écart avec les polices de la charte est assumé et daté ici.
- Le choix a été fait le 19/09 : [ADR-018](018-direction-typographique-quatre-onglets.md).
