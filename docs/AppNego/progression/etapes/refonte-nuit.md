# Refonte « Nuit »

> [Point central](../../progress.md) · [ADR-023](../../adr/023-direction-nuit.md) · [la maquette qui fait foi](../../design/nuit/LISEZMOI.md)

## État

Toute l'application passe à la direction « Nuit » le 05/10/2026, au pixel près de la maquette. Menée par deux sessions, `bob1` (système, lexique, ressources, entrée et compte) et `bob2` (accueil, sessions, négociations, Francophonie, validation, tirets cadratins), orchestrées depuis une troisième ; protocole et journaux dans `.orchestration/` (non versionné). Reste l'appareil réel, comme pour les autres étapes.

## Décisions

| Date | Décision | Qui |
|---|---|---|
| 05/10 | Direction « Nuit » retenue parmi quatre (Douce, Nuit, Mosaïque, Billet), à appliquer au pixel près ; la disposition seule s'ajuste | Commanditaire |
| 05/10 | Thème unique sombre : le choix Clair / Sombre / Système disparaît | Commanditaire |
| 05/10 | Le créneau neutre du fil du jour (2,1:1) et les filets (1,6:1) gardent les valeurs de la maquette : repères décoratifs, le sens est porté par le texte ; sortis de la mesure de contraste | Orchestrateur |
| 05/10 | Le lexique dit « mis à jour le » (`updated_at`) : il n'a pas de date de vérification dans le modèle, on n'en invente pas | Orchestrateur |
| 05/10 | Le lexique ouvert par « Aa » n'allume aucun onglet (règle testée), contre la maquette qui allume Ressources | Orchestrateur |
| 05/10 | « Synchronisé à » ne paraît plus qu'hors connexion ; l'heure reste dans Réglages | Orchestrateur |
| 05/10 | Les tirets cadratins des textes d'interface sont réécrits (règle de `CLAUDE.md`) ; un intervalle prend le demi-cadratin ; « Bienvenue » reste (règle du site seulement) | Orchestrateur |

## Ce qui change pour la personne

- **Accueil** : un compte à rebours (minutes avant sa prochaine session, ou jours avant l'ouverture), « Le fil du jour » qui montre les trois agendas sur une même ligne horaire sans les confondre, les changements du jour en lignes à pastille. Sans compte, le compte à rebours vise toute la COP et une carte invite à créer un compte. Avant la COP, « Jusqu'à la COP » et « Se préparer ».
- **Sessions, réunions, Pavillon, mon agenda** : une frise verticale commune, barre « MAINTENANT » le jour même.
- **Lexique** : le terme trouvé en grand, à l'accent, sous sa marque « EN ».
- Le « Aa » quitte l'en-tête de l'accueil (une ligne en fin d'accueil le remplace) et de l'écran des sessions, dont le bouton rond ouvre une feuille de filtre.

## Historique

| Lot | Commits | Contenu |
|---|---|---|
| Référence | `cbe5c15`, `5483540`, `31f7ced` | Maquette statique, ADR-023, `CLAUDE.md` |
| bob1, lot 1 | `3731e11`..`474ff97` | Polices Sora et Manrope embarquées, jetons, mesures, pictogrammes, composants de base, planche, contrastes, retrait du choix de thème |
| bob2, lot 1 | `63cd8a6`..`a3b078a` | `edition.ts` garde l'ouverture et la clôture ; logique du temps (`journee.ts`) ; accueil en trois états ; sessions en frise |
| bob1, lot 2 | `b46fc2a`..`5a614da` | Lexique au pixel, ressources, documents, lecteur, FAQ, recherche, notifications, entrée et compte |
| bob2, lot 2 | `5e1c60e`..`8f6eac4` | Frise commune, fiches, Francophonie, Pavillon, validation |
| bob2, tirets | `4d286e8` | 334 libellés fr et en dans 94 fichiers de traduction, et les chaînes affichées par le code |
| Fusion | `15b1018` | Les deux branches dans `main` ; l'intervalle d'heures partagé avec le site (`common.datetime.timeRangeWithZone`) passe au demi-cadratin, l'ePavillon compris |

## Écarts connus

- Les écrans « fermée » et l'état « absent » du lecteur n'ont pas été vus faute de données d'exemple.
- Les commentaires de code portent encore des tirets cadratins : hors du lot, qui ne visait que le texte affiché. Les titres des données d'exemple gardent les leurs (des données).
- `compteurDeCloche` (`signalements.ts`) n'est plus appelé ; gardé avec son test.
