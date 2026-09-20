# ADR-000 — Guide Négo est la face mobile de l'ePavillon

**Statut** : accepté — 18/09/2026

## Contexte

Le modèle de données de l'ePavillon contient déjà les schémas `negotiation`, `training` et `tool` : documents, réunions, canaux, quiz, assistant. Ils étaient prévus au jalon 4 et n'ont aucun code. Documents de négociation, sessions et réunions de la Francophonie sont communs aux deux produits.

## Décision

Guide Négo n'est pas un nouveau projet : même dépôt, même base, même API Rust, même compte, même back-office. Il ajoute un client mobile, les crates `negotiation` puis `training`, et un service d'IA.

## Conséquences

- Les règles du dépôt s'appliquent telles quelles : SQL d'abord, contrat d'API engendré, permissions avec portée.
- Un document publié une fois sert le site et l'application.
- **Le design, lui, n'est pas partagé** — [ADR-016](016-vert-et-jaune-fonces-police-hors-charte.md).
- Le suivi du projet est distinct — [ADR-017](017-le-suivi-vit-dans-progress-md.md).
