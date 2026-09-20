# ADR-018 — Direction « Typographique », quatre onglets et un bouton de lexique

**Statut** : accepté — 19/09/2026

## Contexte

Claude Design a proposé plusieurs directions artistiques sur l'écran « Sessions de négociation », et plusieurs barres d'onglets. Le porteur a choisi. L'écran de référence, en clair et en sombre, est gardé dans [design/ecrans/00-direction-retenue.html](../design/ecrans/00-direction-retenue.html).

## Décision

- **Direction « Typographique »** : blanc dominant, vert très foncé `#233400` comme encre des titres et des filets forts, `#3C5404` en accent. La beauté vient de la typographie et du rythme, pas de l'ornement.
- **Une seule police : Atkinson Hyperlegible Next**, en 400, 600 et 700, chiffres tabulaires. Conçue pour la basse vision, à licence ouverte ; elle sera embarquée dans l'application.
- **Quatre onglets avec pictogrammes** : Accueil · Négociations · Francophonie · Ressources. **Le lexique n'est pas un onglet** : un bouton « Aa », permanent, en haut à droite de chaque écran. **Complété le 20/09** : à l'ouverture des Échanges, la barre passe à cinq onglets à largeur de libellé — Accueil · Négociations · Francophonie · Échanges · Ressources — ; l'option d'un bouton d'en-tête a été écartée, les Échanges devant rester sous le pouce.
- **États d'une session** : En cours vert `#557607` · Déplacée cyan `#0C6792` · Annulée rouge `#C7101D` · Non annoncée violet `#732F85` · Terminée gris `#565554` · Prévue noir — toujours avec un pictogramme et un mot.
- **Thème sombre** sur fond `#101704`, vert et jaune désaturés.

## Conséquences

- Le lexique s'ouvre d'un geste depuis n'importe où : c'est l'usage en salle qui le demandait.
- À régler au système de design : le **rôle du jaune foncé**, presque absent de cette direction ; le relief de la session « En cours » ; des filets de liste trop pâles pour le plein soleil ; des signes d'état à remplacer par de vrais pictogrammes ; aucune taille sous 15 px hors onglets.
