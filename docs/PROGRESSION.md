# Progression

**Ce fichier est la mémoire du projet entre deux sessions Claude Code.**

Le contexte d'une session se perd ; le dépôt reste. Toute session commence par lire ce fichier et se termine par le mettre à jour. Une session qui ne le fait pas oblige la suivante à tout redécouvrir.

**Dernière mise à jour** : 16 août 2026 — création du dossier de projet.

---

## État général

| Domaine | État |
|---------|------|
| Modèle de données | ✅ Complet et validé — 18 fichiers, 149 tables, chargement vérifié sur PostgreSQL 17 + pgvector |
| Cadrage et décisions | ✅ Complet — 14 ADR, périmètre du jalon arrêté |
| Environnement local | ⬜ À monter — voir [ENVIRONNEMENT_LOCAL.md](ENVIRONNEMENT_LOCAL.md), les fichiers `ops/` et `Makefile` restent à créer |
| Guide de style | ⬜ À produire avec Claude Design (`docs/PROMPT_STYLE_GUIDE.md`) |
| Front | ⬜ Rien de commencé |
| API | ⬜ Rien de commencé |

---

## Journal

Une ligne par session. La plus récente en haut. Court : ce qui a été fait, ce qui bloque, ce qui vient.

| Date | Session | Fait | À suivre |
|------|---------|------|----------|
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
| — | — | — | — |

---

## Écarts constatés entre le modèle et l'interface

À remplir pendant la phase front. Chaque écart est soit un défaut du modèle, soit un défaut de l'interface — il se tranche, il ne se contourne pas par une conversion.

| Écart | Où | Décision |
|-------|----|----------|
| — | — | — |

---

## Décisions prises en cours de route

Ce qui n'était pas dans le cadrage et qu'il a fallu trancher.

| Date | Décision | Raison |
|------|----------|--------|
| — | — | — |

---

## Points bloqués ou en attente

| Sujet | Nature | Depuis |
|-------|--------|--------|
| Sens exact de « QCD » pour les quiz de formation | À confirmer auprès de l'IFDD | 2026-08-16 |
| Statut OIF des pays | Liste officielle à obtenir, ne peut pas être devinée | 2026-08-16 |
| Reprise ou abandon de la messagerie directe | Arbitrage à rendre | 2026-08-16 |
