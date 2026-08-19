# Progression

**Ce fichier est la mémoire du projet entre deux sessions Claude Code.**

Le contexte d'une session se perd ; le dépôt reste. Toute session commence par lire ce fichier et se termine par le mettre à jour. Une session qui ne le fait pas oblige la suivante à tout redécouvrir.

Il ne porte plus que ce qui se lit **en arrivant** : l'état général, l'avancement écran par écran, et ce qui bloque. Le détail vit dans [`progression/`](progression/) — **on n'en ouvre que le fichier utile à la tâche du jour.**

**Dernière mise à jour** : 19 août 2026 — **la progression est découpée dans [`progression/`](progression/)**, un fichier par écran, par jour de journal et par jour de décisions. Avant cela, le même jour : accueil public et vitrine administrable (A15), le module `content` qui la porte, l'ouverture du verre dépoli dans la charte, les visuels réels repris de la plateforme v1, l'historique des éditions passé au rail-affiche, et les trois formats d'image d'une édition (32:9, 16:9, 1:1).

---

## État général

| Domaine | État |
|---------|------|
| Modèle de données | ✅ Complet et validé — 18 fichiers, 142 tables, chargement vérifié sur PostgreSQL 17 + pgvector |
| Cadrage et décisions | ✅ Complet — 14 ADR, périmètre du jalon arrêté. Audité le 16/08, corrigé |
| Environnement local | ✅ Monté et vérifié le 16/08 — `ops/docker-compose.dev.yml`, `ops/garage.toml`, `Makefile`, `.gitignore`, `.env.example`. Cinq services démarrés, schéma chargé, `make check` au vert |
| Front | 🟨 **Treizième écran métier le 19/08 : l'accueil public et sa vitrine administrable (A15)** — 4 pages (`app/pages/index.vue`, qui remplace la redirection du 17/08, et `app/pages/admin/vitrine/`), 17 composants (`app/components/home/`, `app/components/admin/showcase/`), 3 utilitaires purs, 3 fichiers de contrats, 3 fichiers de mocks, 2 fabriques d'API, 6 fichiers de traduction. **Le modèle a été complété d'abord** : nouveau schéma `content` (`115_content.sql`), vues `event.v_public_editions` et `programme.v_edition_stats`, rôle `video` ajouté à `media.attachment_role`. **Douzième écran métier le 18/08 : les messages d'incident (A13)** — 3 pages sous `app/pages/admin/incidents/`, 6 composants sous `app/components/admin/incidents/`, 1 utilitaire pur (`utils/incident-list.ts`), 1 fichier de contrats (`types/admin-incidents.ts`), 1 fichier de mocks d'écran (`mocks/admin-incidents.ts`), 1 fabrique d'API, 4 fichiers de traduction, plus le raccourci « Signaler un débordement » dans le panneau de séance du planificateur. **Deux compléments du modèle** : la nature « débordement sur le créneau suivant », qui manquait au vocabulaire, et `live.event_incidents()`, qui rend les cinq états d'un message là où le modèle ne savait lire que les actifs. **Onzième écran métier le 18/08 : les utilisateurs, leurs rôles SCOPÉS et le RGPD (A12)** — 3 pages sous `app/pages/admin/utilisateurs/`, 10 composants sous `app/components/admin/users/`, 2 utilitaires purs (`utils/role-scope.ts`, `utils/user-list.ts`), 1 fichier de contrats (`types/admin-users.ts`), 1 dossier de mocks en quatre fichiers (`mocks/admin-users/`), 2 mocks d'entité (`mocks/platform.ts`, `mocks/privacy.ts`), 1 fabrique d'API, 10 fichiers de traduction. Le RBAC scopé de `030_identity.sql` — la réponse au défaut structurel de la v1, huit rôles globaux dans un ENUM — reçoit enfin sa surface : une pastille ne porte jamais un rôle sans sa PORTÉE. **Deux corrections du modèle** : la révocation d'un rôle ne disait ni qui ni pourquoi, et `identity.role.assign` n'était détenue par aucun rôle. **Dixième écran métier le 18/08 : les organisations et la fusion des doublons (A11)** — 4 pages sous `app/pages/admin/organisations/`, 9 composants sous `app/components/admin/organizations/`, 2 utilitaires purs (`utils/organization-list.ts`, `utils/organization-merge.ts`), 1 fichier de contrats (`types/admin-organizations.ts`), **1 dossier de mocks en cinq fichiers** (`mocks/admin-organizations/`), 1 fabrique d'API (`composables/api/admin-organizations.ts`), 8 fichiers de traduction (4 × 2 locales). `analytics.mv_organization_scorecard`, présente dans le modèle depuis le premier jour et affichée nulle part, devient la liste du back-office ; le décompte de fusion se lit dans `org.organization_references` plutôt que dans une liste écrite à la main. **Une correction du modèle** : le registre ne dédoublonnait pas les domaines de courriel. **Révision du tableau de bord le 18/08, sur demande du commanditaire : ApexCharts remplace les cinq graphiques dessinés à la main, et six indicateurs de tête ouvrent la zone des chiffres** — 1 greffon client (`plugins/apexcharts.client.ts`, import dynamique), 1 composable (`useChartTheme`, qui lit les jetons de design dans le DOM), 1 enveloppe (`UiChart`), 2 composants (`AdminStatCard`, `AdminKeyFigures`), 3 composants réécrits (courbe quotidienne, répartition, entonnoir), et la moyenne mobile du modèle enfin tracée. **Neuvième écran métier le 18/08 : le planificateur de créneaux (A9)** — 1 page `app/pages/admin/programmation.vue`, 5 composants sous `app/components/admin/planner/`, 1 utilitaire pur (`utils/planner.ts`), 1 fichier de contrats (`types/admin-planner.ts`), 1 fichier de mocks (`admin-planner.ts`), 2 fichiers de traduction, 1 fabrique d'API (`composables/api/planner.ts`) ; **vue-cal passe en mode éditable**, salles en colonnes, glisser-déposer et redimensionnement, et `detect_conflicts()` cesse de signaler comme bloquant ce qui n'occupe pas le stand. **Huitième écran métier le 18/08, et le plus dense du jalon : la fiche d'évaluation d'une proposition (A8)** — 1 page `app/pages/admin/propositions/[id].vue`, 10 composants sous `app/components/admin/review/`, 1 utilitaire pur (`utils/review-scoring.ts`, qui rejoue `refresh_proposal_score()` en direct), 1 fichier de contrats (`types/admin-review.ts`), 1 fichier de mocks (`proposal-review.ts`, qui applique le VOILE de l'évaluation en aveugle à la source), 2 fichiers de traduction ; `useApi()` se découpe pour la première fois (`composables/api/proposal-review.ts`) et son périmètre d'administration cesse de tester des noms de rôles. **Septième écran métier le 18/08 : la liste des propositions du back-office (A7)** — 1 page `app/pages/admin/propositions/index.vue`, 4 composants sous `app/components/admin/proposals/`, 2 utilitaires purs (`utils/proposal-list.ts`, `utils/permissions.ts`), 1 fichier de contrats (`types/admin-proposals.ts`), 4 fichiers de mocks (`permissions.ts`, `proposal-workflow.ts`, `proposal-reads.ts`, `admin-proposals.ts`), 2 fichiers de traduction ; `v_proposal_dashboard` gagne treize colonnes et `programme.unread_proposals_for()` apparaît, `UiTable` gagne `hideBelow` et `rowLabelKey`. Socle posé le 16/08 — Nuxt 4.5, TypeScript strict, Tailwind v4, Pinia, i18n fr/en, jetons de design, **trois layouts** (`public`, `admin`, `auth`). **Sixième écran métier le 17/08, et premier du back-office : le tableau de bord (A6)** — 1 page `app/pages/admin/index.vue`, 5 composants sous `app/components/admin/` (sélecteur d'événement, file d'actions, entonnoir, courbe, répartition, santé), 2 fichiers de types (`types/analytics.ts`, `types/admin-dashboard.ts`), 3 fichiers de mocks (`incidents.ts`, `analytics.ts`, `admin-dashboard.ts`), 2 fichiers de traduction ; le store `admin-scope` charge désormais le périmètre, le sélecteur d'événement passe de la navigation latérale à la tête de page et vit dans l'URL, `130_analytics.sql` gagne `mv_daily_registrations` et `080_live.sql` `active_incidents_for_event()`. **Cinquième écran métier le 17/08 : l'espace organisation (A5)** — 2 pages sous `app/pages/organization/`, 7 composants sous `app/components/workspace/`, 1 utilitaire pur (`utils/proposal-timeline.ts`), 2 fichiers de types (`types/engagement.ts`, `types/organization-workspace.ts`), 3 fichiers de mocks (`reminders.ts`, `organization-workspace.ts`, `proposals/history.ts`), 4 fichiers de traduction ; `org.memberships` gagne `invited_by`/`invited_at`, et `UiStatusTimeline` deux corrections. **Quatrième écran métier le 17/08 : le formulaire de soumission (A4)** — 1 page gardée, 11 composants sous `app/components/proposal/`, 1 composable d'enregistrement automatique, 16 fichiers de traduction découpés par étape ; `UiStepper` corrigé (débordement à 375 px, navigation en avant) et `utils/datetime.ts` complété de `instantFromWallClock()`. **Troisième écran métier le 17/08 : la page publique d'une édition (A3)** — 1 page + la redirection d'accueil, 10 composants sous `app/components/event/`, vue-cal introduit en lecture seule, 4 fichiers de traduction, quatre éditions de plus dans les mocks. **Deuxième écran métier le 17/08 : le rattachement à une organisation (A2)** — 1 page, 5 composants, la recherche du modèle rejouée dans les mocks, 4 fichiers de traduction. **Premier écran métier le 17/08 : l'authentification (A1)** — 5 pages, 6 composants, un store de session, les comptes et jetons simulés. **Types du modèle dérivés le 16/08** : 14 fichiers, 62 tables du jalon, 2 vues, 8 retours de fonction, puis alignés le 16/08 sur les deux corrections du modèle (`AdministeredEvents`, `ProposalDashboardRow`). **Données simulées le 16/08** : 22 fichiers sous `app/mocks/`, 13 organisations dont un doublon, 25 personnes, 40 propositions, 32 revues, 30 sessions, 60 inscriptions ; couche d'accès `useApi()` réécrite. **Bibliothèque d'interface le 16/08** : 38 composants sous `app/components/ui/`, les 27 du prompt plus 11 briques ; guide de style vivant à `/style-guide`, 7 sections ancrées, rendu sur données réelles. `make check-front` au vert |
| API | ⬜ Rien de commencé |

L'environnement local, ce qui y a été vérifié et les écarts relevés : [`progression/environnement-local.md`](progression/environnement-local.md).

---

## Front — suivi des prompts

Un écran = un fichier. Il porte ce qui a été livré, les écarts relevés entre le modèle et l'interface, et ce qui a été vérifié. Chaque écart est soit un défaut du modèle, soit un défaut de l'interface — il se tranche, il ne se contourne pas par une conversion. Les prompts correspondants sont dans [PROMPTS_DEVELOPPEMENT.md](PROMPTS_DEVELOPPEMENT.md).

| Prompt | Écran | État | Détail |
|--------|-------|------|--------|
| A0.1 | Socle Nuxt, Tailwind, i18n, jetons | ✅ 16/08 | [écarts et vérifications](progression/ecrans/a0.1-socle.md) |
| A0.2 | Types TypeScript dérivés du SQL | ✅ 16/08 | [écarts et vérifications](progression/ecrans/a0.2-types.md) |
| A0.3 | Données simulées | ✅ 16/08 | [écarts et vérifications](progression/ecrans/a0.3-donnees-simulees.md) |
| A0.4 | Composants d'interface + page de guide de style | ✅ 16/08 | [écarts et vérifications](progression/ecrans/a0.4-composants.md) |
| A1 | Authentification | ✅ 17/08 | [écarts et vérifications](progression/ecrans/a1-authentification.md) |
| A2 | Rattachement à une organisation | ✅ 17/08 | [écarts et vérifications](progression/ecrans/a2-organisation.md) |
| A3 | Page publique de l'événement | ✅ 17/08 | [écarts et vérifications](progression/ecrans/a3-evenement-public.md) |
| A4 | Formulaire de soumission | ✅ 17/08 | [écarts et vérifications](progression/ecrans/a4-soumission.md) |
| A5 | Espace organisation | ✅ 17/08 | [écarts et vérifications](progression/ecrans/a5-espace-organisation.md) |
| A6 | Tableau de bord back-office | ✅ 17/08 | [écarts et vérifications](progression/ecrans/a6-tableau-de-bord.md) |
| A7 | Liste des propositions | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a7-propositions.md) |
| A8 | Fiche d'évaluation | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a8-evaluation.md) |
| A9 | Planificateur de créneaux | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a9-planificateur.md) |
| A10 | Gestion des événements | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a10-evenements.md) |
| A11 | Organisations et fusion | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a11-organisations-fusion.md) |
| A12 | Utilisateurs et rôles | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a12-utilisateurs-roles.md) |
| A13 | Messages d'incident | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a13-incidents.md) |
| A14 | Page « En cours de maintenance » | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a14-maintenance.md) |
| A15 | Accueil public et vitrine administrable | ✅ 19/08 | [écarts et vérifications](progression/ecrans/a15-accueil.md) |

Les pièges transverses, ceux qui ne tiennent à aucun écran : [`progression/pieges.md`](progression/pieges.md).

---

## API — suivi des prompts

Le tableau et les obligations d'API relevées en écrivant les écrans : [`progression/api.md`](progression/api.md). Aucun prompt B n'est commencé.

---

## Où trouver le reste

| Question | Fichier |
|----------|---------|
| Qu'a fait la session d'hier ? | [`progression/journal/`](progression/journal/) — un fichier par jour, le plus récent d'abord |
| Pourquoi cette décision d'interface, de nommage, de découpage ? | [`progression/decisions/`](progression/decisions/) — un fichier par jour |
| Quelles tables ont bougé, et pourquoi ? | [`progression/modele.md`](progression/modele.md) |
| Quels écarts sur l'écran que je reprends ? | [`progression/ecrans/`](progression/ecrans/) — le fichier de son prompt |
| Qu'est-ce qui attend une réponse du commanditaire ? | [`progression/points-bloques.md`](progression/points-bloques.md) |
| Qu'est-ce qui a déjà coûté une erreur ? | [`progression/pieges.md`](progression/pieges.md) |

### Journal

| Jour | Fichier |
|------|---------|
| 19 août 2026 | [`journal/2026-08-19.md`](progression/journal/2026-08-19.md) |
| 18 août 2026 | [`journal/2026-08-18.md`](progression/journal/2026-08-18.md) |
| 17 août 2026 | [`journal/2026-08-17.md`](progression/journal/2026-08-17.md) |
| 16 août 2026 | [`journal/2026-08-16.md`](progression/journal/2026-08-16.md) |

---

## Points bloqués

Trois questions au commanditaire attendent depuis le 16/08, et quelques points restent en suspens : [`progression/points-bloques.md`](progression/points-bloques.md).

---

## Mettre à jour ce dispositif en fin de session

1. **Le journal du jour** — `progression/journal/<date>.md`, une ligne : ce qui a été fait, ce qui bloque, ce qui vient. Le fichier du jour se crée s'il n'existe pas, et se déclare dans le tableau ci-dessus.
2. **Le fichier de l'écran travaillé** — `progression/ecrans/<prompt>.md` : ce qui a été livré, les écarts relevés, ce qui a été vérifié et comment. Un nouveau prompt ouvre un nouveau fichier, déclaré dans le suivi ci-dessus.
3. **Les décisions du jour** — `progression/decisions/<date>.md`, si quelque chose a été tranché.
4. **`progression/modele.md`** — si un fichier de `docs/database/` a changé.
5. **Ici** — l'état général, la ligne de suivi du prompt, et la date de dernière mise à jour en tête.

Rien d'autre n'a besoin d'être touché. **Ne pas regrossir ce fichier** : il se lit en entier à chaque session, c'est ce qui lui donne sa valeur.
