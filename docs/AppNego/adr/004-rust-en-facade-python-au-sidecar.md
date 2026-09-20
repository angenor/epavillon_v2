# ADR-004 — Rust en façade, Python au sidecar

**Statut** : accepté — 18/09/2026

## Contexte

Lecture de PDF, découpage, transcription, appels aux modèles : l'outillage de l'IA est en Python. L'ePavillon avait prévu dès l'origine que le schéma `tool` soit détachable, sans aucune clé étrangère sortante (son ADR-11).

## Décision

L'IA vit dans un service Python (FastAPI), **interne et jamais exposé**. L'API Rust reste la seule porte : elle tient la session, les droits et les quotas, et relaie. Le service possède le schéma `tool` et n'écrit nulle part ailleurs.

## Conséquences

- Une seule authentification, une seule politique d'origine, un seul endroit où couper.
- Le service ne lit aucun schéma métier : documents et transcriptions lui arrivent par la file d'événements, avec leur référence de stockage.
- Les réponses en flux traversent l'API Rust.
- Une panne du service d'IA ne touche ni les documents, ni les agendas, ni les échanges.
