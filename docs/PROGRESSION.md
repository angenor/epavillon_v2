# Progression

**Ce fichier est la mémoire du projet entre deux sessions Claude Code.**

Le contexte d'une session se perd ; le dépôt reste. Toute session commence par lire ce fichier et se termine par le mettre à jour. Une session qui ne le fait pas oblige la suivante à tout redécouvrir.

**Dernière mise à jour** : 16 août 2026 — audit de la documentation et corrections.

---

## État général

| Domaine | État |
|---------|------|
| Modèle de données | ✅ Complet et validé — 18 fichiers, 142 tables, chargement vérifié sur PostgreSQL 17 + pgvector |
| Cadrage et décisions | ✅ Complet — 14 ADR, périmètre du jalon arrêté. Audité le 16/08, corrigé |
| Environnement local | ⬜ À monter — voir [ENVIRONNEMENT_LOCAL.md](ENVIRONNEMENT_LOCAL.md), les fichiers `ops/` et `Makefile` restent à créer |
| Front | ⬜ Rien de commencé |
| API | ⬜ Rien de commencé |

---

## Journal

Une ligne par session. La plus récente en haut. Court : ce qui a été fait, ce qui bloque, ce qui vient.

| Date | Session | Fait | À suivre |
|------|---------|------|----------|
| 2026-08-16 | Audit documentaire | Revue des 10 fichiers Markdown ([AUDIT_DOCUMENTATION.md](AUDIT_DOCUMENTATION.md)). Restauration de `docs/historique/`, correction des vestiges de décisions retournées, recomptage du modèle, préambule ajouté à tous les prompts | Trancher les arbitrages listés plus bas, puis monter l'environnement local |
| 2026-08-16 | Mise en place | Dossier de projet créé, documentation réorganisée, `CLAUDE.md` et index du modèle écrits | Monter l'environnement local, puis produire le guide de style |

---

## Front — suivi des prompts

Cocher au fur et à mesure. Les prompts correspondants sont dans [PROMPTS_DEVELOPPEMENT.md](PROMPTS_DEVELOPPEMENT.md).

| Prompt | Écran | État | Notes |
|--------|-------|------|-------|
| A0.1 | Socle Nuxt, Tailwind, i18n, jetons | ⬜ | |
| A0.2 | Types TypeScript dérivés du SQL | ⬜ | |
| A0.3 | Données simulées | ⬜ | |
| A0.4 | Composants d'interface + page de guide de style | ⬜ | |
| A1 | Authentification | ⬜ | |
| A2 | Rattachement à une organisation | ⬜ | Écran critique — qualité du référentiel |
| A3 | Page publique de l'événement | ⬜ | |
| A4 | Formulaire de soumission | ⬜ | |
| A5 | Espace organisation | ⬜ | |
| A6 | Tableau de bord back-office | ⬜ | |
| A7 | Liste des propositions | ⬜ | |
| A8 | Fiche d'évaluation | ⬜ | Écran le plus dense |
| A9 | Planificateur de créneaux | ⬜ | |
| A10 | Gestion des événements | ⬜ | |
| A11 | Organisations et fusion | ⬜ | |
| A12 | Utilisateurs et rôles | ⬜ | |
| A13 | Messages d'incident | ⬜ | |
| A14 | Page « En cours de maintenance » | ⬜ | |

## API — suivi des prompts

| Prompt | Module | État | Notes |
|--------|--------|------|-------|
| B0 | Constitution Spec Kit | ⬜ | |
| B1 | Socle + Identité | ⬜ | |
| B2 | Organisations | ⬜ | |
| B3 | Événements | ⬜ | |
| B4 | Propositions | ⬜ | |
| B5 | Sessions | ⬜ | |
| B6 | Média + Engagement | ⬜ | |
| B7 | Raccordement du front | ⬜ | |

---

## Modifications du modèle de données

Toute modification d'un fichier de `docs/database/` se note ici. C'est ce qui permet de savoir, plus tard, pourquoi le schéma diffère de ce que décrit le cadrage.

| Date | Fichier | Changement | Motif |
|------|---------|------------|-------|
| 08-16 | `075_programme_sessions.sql` | Suppression des contraintes d'exclusion `ex_sessions_no_room_overlap` et `ex_sessions_no_broadcast_overlap` ; remplacées par `detect_conflicts()` et `publication_readiness()` | Le commanditaire : les chevauchements ne doivent pas être bloqués, l'admin réorganise par glisser-déposer |
| 08-16 | `060_events.sql` | Index unique `ux_calls_one_per_event` — un seul appel à propositions par édition | Une version antérieure en autorisait plusieurs ; le commanditaire a tranché |
| 08-16 | `070` et `075` | Ajout de `proposal_organizations` et `session_organizations` (porteur principal, co-organisateurs, partenaires, soutiens) | La co-organisation n'existait pas dans le modèle ; elle est courante en réalité |
| 08-16 | `040_organizations.sql` | Colonne `dedupe_on` dans `organization_references` + déduplication dans `merge_organizations()` | Sans elle, la fusion échouait dès que deux organisations co-organisaient la même activité |
| 08-16 | `060_events.sql` | Ajout de `event.programme_tracks` + `programme.session_tracks` ; `event_days` ramené à son rôle de calendrier | Une journée spéciale n'occupe pas forcément le jour entier et peut déborder sur deux jours |
| 08-16 | `060` et `075` | Ajout de `event.broadcast_channels` et de `sessions.broadcast_channel_id` | Règle « un seul direct à la fois » : le canal devient une ressource réservable |
| 08-16 | `030_identity.sql` | Rôle `admin` attribuable sur la portée `event` + `administered_events()` + trigger de contrôle de portée | Confier un événement à un responsable sans lui ouvrir le reste (cas rencontré en v1) |
| 08-16 | `010_platform.sql` | Ajout de `entity_history()` ; historique dérivé du journal d'audit | Le commanditaire demande l'historique des modifications d'activité |
| 08-16 | `125_training.sql` | Nouveau module Formations (12 tables) | Remonté au produit minimum viable à la demande du commanditaire |
| 08-16 | `030_identity.sql` | `privacy_requests.due_at` passé de colonne générée à `DEFAULT` | `timestamptz + interval` est STABLE, donc interdit dans une expression `GENERATED` |

---

## Écarts constatés entre le modèle et l'interface

À remplir pendant la phase front. Chaque écart est soit un défaut du modèle, soit un défaut de l'interface — il se tranche, il ne se contourne pas par une conversion.

| Écart | Où | Décision |
|-------|----|----------|
| — | — | — |

---

## Décisions prises en cours de route

Ce qui n'était pas dans le cadrage initial et qu'il a fallu trancher. **Quand un document semble contredire une décision, c'est ce tableau qui dit laquelle est la plus récente.**

| Date | Décision | Raison |
|------|----------|--------|
| 08-16 | **Les chevauchements de créneaux sont signalés, jamais bloqués.** Seule la publication du programme est conditionnée | Une contrainte qui refuse l'écriture transforme l'outil d'arbitrage en mur : un planificateur travaille par déplacements successifs et passe par des états incohérents (ADR-13) |
| 08-16 | **Un seul appel à propositions par édition**, zéro s'il n'y a pas de pavillon | Les journées thématiques sont composées *après* sélection, à partir du vivier commun — elles n'ouvrent pas leur propre fenêtre |
| 08-16 | **La co-organisation est de premier ordre** : porteur principal + co-organisateurs, partenaires, soutiens | Sans elle, les co-organisateurs restaient dans le texte de présentation, invisibles des filtres et des statistiques |
| 08-16 | **Les journées spéciales ne sont pas des jours de calendrier** mais des fils composés à la main (`programme_tracks`) | Une journée spéciale peut n'occuper qu'une matinée, un jour peut en porter deux, un fil peut déborder sur deux jours |
| 08-16 | **Le rôle d'administrateur est attribuable sur un seul événement** | Évite de redévelopper une page d'administration séparée comme en v1 (ADR-14) |
| 08-16 | **Le module Formations entre au produit minimum viable** | Demande explicite du commanditaire : « le module formation est important, il doit être construit au MVP » |
| 08-16 | **Le workspace Cargo vit dans `backend/`**, pas à la racine | Symétrie avec `frontend/` ; à la racine, le dépôt se lisait comme « un projet Rust contenant un frontend » |
| 08-16 | **Traductions, types et mocks découpés par écran**, pas par domaine | Un fichier par domaine reste trop volumineux : le seul formulaire de soumission compte sept étapes |
| 08-16 | **Pas d'intégration continue, mais trois vérifications locales** dans un `Makefile` | Développeur seul et pressé : une chaîne complète est du temps pris sur la livraison |
| 08-16 | **Les quiz de formation ne réutilisent pas le module `tool`** | `tool` est conçu pour être extrait ; un quiz de formation est indissociable de la progression et de l'attestation |

---

## Points bloqués ou en attente

| Sujet | Nature | Depuis |
|-------|--------|--------|
| **Portée de la règle « un seul direct »** — le modèle garantit un direct par *canal* ; si deux événements ouvrent chacun le leur, deux directs simultanés redeviennent possibles. Faut-il un verrou global ? | Arbitrage à rendre | 2026-08-16 |
| **Édition, date et ville de référence** — les données simulées disent « COP31, Belém, novembre 2027 », or Belém accueillait la COP30 | Information à fixer | 2026-08-16 |
| **Évaluation en aveugle** — le commanditaire demande que les révisionnistes voient les notes des autres et la moyenne ; le modèle propose l'aveugle en option. Quelle valeur par défaut ? | Arbitrage à rendre | 2026-08-16 |
| **Reprise des données v1** — placée au jalon 2, alors que le critère de sortie du jalon 1 exige des « données réelles importées » | Incohérence de planification | 2026-08-16 |
| **Qui sert la COP31** — v1 ou v2, et comment les propositions collectées d'un côté rejoignent l'exécution de l'autre | Arbitrage à rendre | 2026-08-16 |
| **Accessibilité et bilinguisme** — imposés comme règles de code, mais absents des exigences produit et des non-objectifs. Écartés ou oubliés ? | Arbitrage à rendre | 2026-08-16 |
| **Synchronisation Google Agenda / Apple** — demandée par le commanditaire en « phase 2+ », absente de toute la documentation | À réintégrer au cadrage | 2026-08-16 |
| Sens exact de « QCD » pour les quiz de formation | À confirmer auprès de l'IFDD | 2026-08-16 |
| Statut OIF des pays | Liste officielle à obtenir, ne peut pas être devinée | 2026-08-16 |
| Reprise ou abandon de la messagerie directe | Arbitrage à rendre | 2026-08-16 |
