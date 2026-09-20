# ADR-005 — OpenRouter, et un modèle d'embedding versionné

**Statut** : accepté — 18/09/2026

## Contexte

Le porteur veut choisir ses modèles librement et en changer sans peine. Départ envisagé : Gemini pour rédiger, voyage-4 pour vectoriser, tous deux par OpenRouter.

## Décision

Tous les appels passent par OpenRouter. Le modèle qui **rédige** est un réglage. Le modèle qui **vectorise** est une donnée : chaque vecteur porte le nom du modèle qui l'a produit.

## Conséquences

- Changer le modèle de rédaction tient en une ligne ; le jeu de 30 à 50 questions de référence dit si c'est un progrès.
- Changer le modèle d'embedding impose de **tout réindexer** : deux modèles ne se mélangent pas dans une même recherche.
- **À corriger dans `120_tools.sql` avant le premier document indexé** : les vecteurs sont figés à 1536 dimensions, voyage-4 en produit 1024 ; et rien ne nomme le modèle d'origine.
- OpenRouter est réglé pour écarter les fournisseurs qui entraînent leurs modèles sur les données reçues.
