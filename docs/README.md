# Le modèle de données — vue d'ensemble

Ce document explique ce que le modèle corrige de la version précédente, comment il est organisé, et ce qu'il garantit lui-même. Pour **construire**, ce n'est pas ici qu'il faut commencer :

> **Où trouver quoi** — le tableau d'orientation complet (conventions, règles métier, prompts, cadrage, charte, historique du commanditaire) vit dans [`../CLAUDE.md`](../CLAUDE.md) et n'est pas repris ici.
> Pour savoir quel fichier SQL lire pour la tâche du jour, aller directement à [MODELE_INDEX.md](MODELE_INDEX.md).

Le modèle lui-même vit dans **[database/](database/)** : 18 fichiers SQL, 14 143 lignes, chaîne validée sur PostgreSQL 17 + pgvector.

**Relevé du 16 août 2026, sur une base réellement chargée.** C'est un constat de mesure, pas une constante du projet :

| Schémas | Tables | Vues | Vues matérialisées | Fonctions | FK inter-modules |
|---------|--------|------|--------------------|-----------|------------------|
| 15 | 142 | 14 | 7 | 153 | 167, aucune non conforme |

Les tables sont comptées hors partitions filles. Ces chiffres bougent dès qu'un fichier SQL change : **les recompter plutôt que les recopier**.

```sql
-- Recomptage du modèle, sur la base locale chargée (voir ENVIRONNEMENT_LOCAL.md)
WITH s AS (
    SELECT oid, nspname FROM pg_namespace
    WHERE nspname IN ('platform','reference','identity','org','media','event',
                      'programme','live','publication','negotiation','engagement',
                      'tool','training','analytics','legacy')
)
SELECT (SELECT count(*) FROM s)                                                AS schemas,
       (SELECT count(*) FROM pg_class c JOIN s ON s.oid = c.relnamespace
          WHERE c.relkind IN ('r','p') AND NOT c.relispartition)               AS tables,
       (SELECT count(*) FROM pg_class c JOIN s ON s.oid = c.relnamespace
          WHERE c.relkind = 'v')                                               AS vues,
       (SELECT count(*) FROM pg_class c JOIN s ON s.oid = c.relnamespace
          WHERE c.relkind = 'm')                                               AS vues_materialisees,
       (SELECT count(*) FROM pg_proc p JOIN s ON s.oid = p.pronamespace
          WHERE p.prokind = 'f')                                               AS fonctions,
       (SELECT count(*) FROM platform.cross_module_fk_report
          WHERE is_cross_module)                                               AS fk_inter_modules,
       (SELECT count(*) FROM platform.cross_module_fk_report
          WHERE NOT is_compliant)                                              AS fk_non_conformes;
```

```bash
# Volume du modèle
wc -l docs/database/*.sql | tail -1
```

---

## En une page : ce qui change

La v1 fonctionne, mais elle a été construite sous contrainte et porte une dette structurelle identifiable. Voici, point par point, ce que la v2 corrige.

| Problème v1 | Cause | Réponse v2 |
|-------------|-------|------------|
| **Organisations en double, impossibles à fusionner** | Recherche sur le nom complet seulement ; le sigle vivait dans une autre colonne. Aucune fonction de fusion. | Toutes les dénominations (nom, sigle, traduction, ancien nom) dans une seule table indexée en trigramme · index unique sur le nom normalisé par pays · domaines de courriel vérifiés · `org.merge_organizations()` atomique et tracée |
| Conflits de créneaux réglés à l'œil | Aucune détection | `programme.detect_conflicts()` recense tous les chevauchements avec leur gravité · rien n'est bloqué, l'admin réorganise par glisser-déposer · contrôle avant publication |
| **Co-organisation impossible** | Une seule `organization_id` par activité | `proposal_organizations` / `session_organizations` : porteur principal, co-organisateurs, partenaires, soutiens |
| Historique des modifications inexistant ou partiel | Table alimentée à la main par le code | `platform.entity_history()` : sous-produit du journal d'audit, donc exhaustif · lecture dédiée des reports de créneau |
| Page d'administration séparée pour un événement | Rôles globaux uniquement | Rôle `admin` attribuable **sur un seul événement** · un back-office, périmètre variable |
| `activities` mélangeait dossier soumis et activité diffusée | Une seule table, une seule colonne de statut pour deux cycles de vie | `programme.proposals` (dossier) ≠ `programme.sessions` (occurrence publiée), relation 1-N |
| `session_edition` greffé dans les inscriptions | Une proposition ne pouvait donner qu'une activité | `sessions.sequence_number` : une proposition engendre N sessions nativement |
| 6 colonnes `guest_*` + 2 `CHECK` croisés | Personne et compte confondus | `identity.people` ≠ `identity.accounts` : un invité est une personne sans compte, rattachée à son compte le jour où elle en crée un |
| Ajouter une question au formulaire = migration | Colonnes en dur (`referral_source`, `paco_demographic_data`…) | `programme.registration_form_fields` : les questions sont des données, les réponses un JSON indexé en GIN |
| Ajouter une thématique = migration DDL | 15 valeurs dans un type ENUM | `reference.taxonomy_terms` : administrable, traduit, ordonné, dépréciable |
| 6 colonnes d'URL de bannière par table | URL absolues stockées en base | `media.assets` (bucket + clé) et `media.renditions` (variantes en lignes) · changer de fournisseur = changer un réglage |
| Notification ou courriel perdu sur panne réseau | Effets de bord déclenchés hors transaction | `platform.outbox_events` : l'état et son annonce commitent ensemble ou pas du tout |
| Note libre sur 20, décision injustifiable | Aucune grille de critères | `event.review_criteria` pondérés et éliminatoires · `programme.review_scores` par critère · évaluation en aveugle |
| « Révisionniste » valait pour toutes les COP | 8 rôles globaux dans un ENUM | RBAC scopé : `global` / `organization` / `event` / `negotiation_space` |
| Journées thématiques codées dans le routeur Vue | `/programmations/2025/journee-jeunesse` en dur | `event.programme_tracks` administrables, composées à la main après sélection via `programme.session_tracks` — à ne pas confondre avec `event.event_days`, qui n'est que le calendrier de l'édition |
| Aucune traçabilité des décisions | Pas de journal d'audit | `platform.audit_log` partitionné + `programme.proposal_transitions` |
| Aucune conformité RGPD | Absente du modèle | Consentements horodatés, demandes d'export et d'effacement, `identity.anonymize_person()` |
| Pré-rendu Puppeteer à chaque nouvelle activité | SPA Vue sans rendu serveur | Nuxt en rendu serveur : plus aucun redéploiement pour publier une page |

---

## Carte des modules

Un module = un schéma PostgreSQL = un crate Rust = une frontière de service potentielle.

```
                          ┌───────────────────────────────┐
                          │  NOYAU PARTAGÉ                │
                          │  platform · reference         │
                          │  audit · outbox · jobs        │
                          │  pays · taxonomies · locales  │
                          └───────────────┬───────────────┘
                                          │  (FK libres vers le noyau)
        ┌─────────────┬─────────────┬─────┴───────┬─────────────┬─────────────┐
        ▼             ▼             ▼             ▼             ▼             ▼
   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐  ┌──────────┐  ┌─────────┐
   │identity │   │   org   │   │  media  │   │  event  │  │engagement│  │  tool   │
   │personnes│◄──┤  fiches │   │ Garage  │   │ séries  │  │ courriels│  │sondages │
   │  RBAC   │   │ fusion  │   │variantes│   │ appels  │  │ rappels  │  │ IA/RAG  │
   └─────────┘   └─────────┘   └─────────┘   └────┬────┘  └──────────┘  └─────────┘
                                                  │                        détachable
                                    ┌─────────────┴─────────────┐          par design
                                    ▼                           ▼
                            ┌───────────────┐           ┌───────────────┐
                            │   programme   │           │   training    │
                            │ propositions  │           │ formations    │
                            │ revue · notes │           │ chapitres     │
                            │ sessions      │           │ quiz          │
                            │ inscriptions  │           │ attestations  │
                            └───────┬───────┘           └───────────────┘
                                    │
                  ┌─────────────────┼─────────────────┐
                  ▼                 ▼                 ▼
             ┌─────────┐     ┌─────────────┐    ┌───────────┐
             │  live   │     │ publication │    │negotiation│
             │ Zoom    │     │  articles   │    │  espaces  │
             │ YouTube │     │  quotas     │    │  canaux   │
             │incidents│     │             │    │ documents │
             └─────────┘     └─────────────┘    └───────────┘

                          ┌───────────────────────────────┐
                          │  analytics — lecture seule    │
                          │  aucune FK sortante           │
                          │  déportable sur un réplica    │
                          └───────────────────────────────┘

                          ┌───────────────────────────────┐
                          │  legacy — zone de transit     │
                          │  reprise des données v1       │
                          │  supprimé après la bascule    │
                          └───────────────────────────────┘
```

`training` est un module de plein droit, pas une annexe : `125_training.sql` est le plus gros fichier du modèle (1 829 lignes) et le commanditaire le veut au produit minimum viable. Il se rattache à `event` et `live` pour les formations tenues pendant un événement ou en visioconférence, et emprunte `identity`, `org`, `media` et `reference` comme tous les autres.

`legacy` n'est pas un module métier mais la zone de transit de la reprise v1 (`910_migration_v1.sql`) : chargement brut, résolution des doublons, puis écriture vers les schémas v2. Le schéma est supprimé une fois la bascule validée.

**La règle qui tient l'ensemble** : toute clé étrangère traversant deux schémas métier porte le préfixe `xmod_fk_`. La vue `platform.cross_module_fk_report` vérifie cette convention, et `platform.generate_module_decoupling_script('<module>')` produit les instructions de découplage le jour où un module part en service autonome.

---

## Le modèle de données

### Ordre d'exécution

Les fichiers sont numérotés dans leur ordre de dépendance. **Ne pas les réordonner.**

| # | Fichier | Schéma | Contenu |
|---|---------|--------|---------|
| 000 | [`000_bootstrap.sql`](database/000_bootstrap.sql) | — | Extensions, schémas, rôles, domaines (`i18n_text`, `email`, `slug`, `url`, `timezone_name`), UUID v7, normalisation textuelle, triggers génériques, contrôle des frontières de modules |
| 010 | [`010_platform.sql`](database/010_platform.sql) | `platform` | Registre des modules, journal d'audit partitionné, outbox et inbox transactionnels, file de travaux, réglages, drapeaux de fonctionnalités |
| 020 | [`020_reference.sql`](database/020_reference.sql) | `reference` | Locales, pays et statut OIF, taxonomies administrables, rattachement générique aux termes |
| 030 | [`030_identity.sql`](database/030_identity.sql) | `identity` | Personnes, comptes et authentification, sessions, RBAC scopé, profils négociateurs, RGPD |
| 040 | [`040_organizations.sql`](database/040_organizations.sql) | `org` | Organisations, dénominations, domaines, adhésions, détection et fusion des doublons |
| 050 | [`050_media.sql`](database/050_media.sql) | `media` | Objets S3, variantes, rattachements contrôlés, quotas de stockage, détection des orphelins |
| 060 | [`060_events.sql`](database/060_events.sql) | `event` | Séries, éditions, calendrier (`event_days`) et journées spéciales (`programme_tracks`), lieux et salles, appels à propositions, grilles d'évaluation |
| 070 | [`070_programme_proposals.sql`](database/070_programme_proposals.sql) | `programme` | Propositions, machine à états, intervenants, documents, revues et notes, échanges |
| 075 | [`075_programme_sessions.sql`](database/075_programme_sessions.sql) | `programme` | Sessions programmées, détection des conflits, formulaires d'inscription, inscriptions, questions du public |
| 080 | [`080_live.sql`](database/080_live.sql) | `live` | Réunions visio agnostiques, synchronisation des inscrits, webhooks, diffusions, incidents |
| 090 | [`090_publications.sql`](database/090_publications.sql) | `publication` | Articles, quotas éditoriaux et de stockage appliqués par la base, révisions, modération |
| 100 | [`100_negotiations.sql`](database/100_negotiations.sql) | `negotiation` | Espaces, réunions unifiées, documents d'aide, canaux d'échange partitionnés |
| 110 | [`110_engagement.sql`](database/110_engagement.sql) | `engagement` | Notifications, modèles, courriels partitionnés, rappels sans doublon, commentaires, messagerie |
| 120 | [`120_tools.sql`](database/120_tools.sql) | `tool` | Sondages et évaluations, assistants IA et RAG (pgvector) — module conçu détaché |
| 125 | [`125_training.sql`](database/125_training.sql) | `training` | Formations, chapitres, enregistrements et supports, quiz, évaluation finale, progression, attestations |
| 130 | [`130_analytics.sql`](database/130_analytics.sql) | `analytics` | Vues matérialisées du back-office, santé opérationnelle, mesure d'audience |
| 900 | [`900_seed.sql`](database/900_seed.sql) | — | Réglages, drapeaux, pays, séries d'événements, organisation pivot, compte d'administration |
| 910 | [`910_migration_v1.sql`](database/910_migration_v1.sql) | `legacy` | Reprise des données v1 : chargement, résolution des doublons, transformation, réconciliation |

### Exécution

Les fichiers sont montés dans le conteneur PostgreSQL et exécutés au premier démarrage — l'ordre alphabétique fait le travail. Voir [ENVIRONNEMENT_LOCAL.md](ENVIRONNEMENT_LOCAL.md).

Contrôles :

```sql
-- Frontières de modules : doit renvoyer zéro ligne
SELECT * FROM platform.cross_module_fk_report WHERE NOT is_compliant;

-- Projections analytiques : les 7 vues doivent se rafraîchir en mode concurrent
SELECT analytics.refresh_all(true);
SELECT view_name, succeeded, error_message FROM analytics.refresh_log WHERE NOT succeeded;

-- Santé opérationnelle : à consulter chaque matin en exploitation
SELECT * FROM analytics.v_operational_health;
```

Ces contrôles, plus la compilation du front et de l'API, sont à passer avant tout commit important — une migration qui ne passe pas sur une base vierge se découvre autrement au déploiement, au pire moment.

L'ensemble a été chargé et vérifié sur PostgreSQL 17 avec pgvector le 16 août 2026 : les 18 fichiers passent sous `ON_ERROR_STOP=1`, le seed est rejouable, les 167 clés étrangères inter-modules relevées ce jour-là étaient toutes conformes — le compte se refait avec la requête donnée en tête de document, il ne se recopie pas —, et un scénario fonctionnel de bout en bout confirme le comportement annoncé — rapprochement et fusion d'organisations, refus des transitions d'état interdites, co-organisation, détection des conflits de créneaux sans blocage, refus d'une inscription incomplète, bascule en liste d'attente, portée d'administration limitée à un événement, émission des événements de domaine.

En production, ces fichiers sont la **source de référence**. Ils sont découpés en migrations incrémentales (SQLx / refinery) au moment de l'implémentation, et chaque application est tracée dans `platform.schema_migrations`.

---

## Conventions du modèle

| Sujet | Règle |
|-------|-------|
| Clés primaires | `id uuid PRIMARY KEY DEFAULT platform.uuid_v7()` — ordonnées dans le temps, opaques |
| Horodatage | `timestamptz` partout, stocké en UTC, converti à l'affichage |
| Textes traduisibles | `platform.i18n_text` — `{"fr": "…", "en": "…"}`, français obligatoire |
| Adresses de courriel | `platform.email` (citext + validation) |
| Segments d'URL | `platform.slug` |
| Vocabulaires ouverts | `reference.taxonomy_terms`, jamais un ENUM |
| Machines à états | ENUM PostgreSQL + table de transitions autorisées |
| Contraintes | `ck_` vérification · `ux_` unique · `ix_` index · `ex_` exclusion |
| FK inter-modules | `xmod_fk_<table>_<cible>` — obligatoire, vérifié automatiquement |
| Suppression | `deleted_at` uniquement là où l'historique a une valeur ; sinon suppression réelle |
| Auteur d'une écriture | `platform.current_actor_id()`, alimenté par `SET LOCAL app.actor_id` |
| Effets de bord | `platform.emit_event()` dans la transaction, jamais d'appel direct entre modules |
| Documentation | `COMMENT ON` en français directement en base |

---

## Ce que la base garantit elle-même

La leçon principale tirée de la v1 : **une règle appliquée seulement par l'interface n'est pas appliquée**. Les invariants suivants sont portés par le SGBD et valent pour tous les chemins d'écriture — API, import massif, script d'administration, correction en console.

Avec une nuance : **tout ce qui est vrai n'a pas vocation à être bloqué** — l'énoncé de référence est la règle métier n°2 de [`../CLAUDE.md`](../CLAUDE.md), à laquelle il faut se reporter avant d'implémenter quoi que ce soit sur les créneaux.

| Invariant | Mécanisme |
|-----------|-----------|
| Pas deux organisations actives homonymes dans un même pays | `ux_organizations_name_country` (index unique partiel) |
| Un domaine vérifié appartient à une seule organisation | `ux_organization_domains_verified` |
| Pas de chaîne de fusion `A → B → C` | `tg_organizations_no_merge_chain` |
| Un seul appel à propositions par édition | `ux_calls_one_per_event` |
| Un seul porteur principal par proposition et par session | `ux_proposal_organizations_lead`, `ux_session_organizations_lead` |
| Un rôle n'est attribuable que sur une portée qu'il autorise | `tg_role_assignments_check_scope` |
| Jamais deux directs simultanés sur un même canal | `ux_streams_single_live_per_channel` |
| Une journée spéciale ne peut pas aspirer une activité d'un autre événement | `tg_session_tracks_check_event` |
| Aucune transition d'état imprévue sur une proposition | `programme.proposal_transitions_allowed` + trigger |
| Recevabilité d'une soumission (fenêtre, plafond, organisation vérifiée) | `tg_proposals_check_eligibility` |
| Aucune note supérieure au maximum d'un critère | `tg_review_scores_bounds` |
| Aucune inscription sans les réponses obligatoires | `tg_registrations_validate` |
| Aucun dépassement de jauge silencieux | Bascule automatique en liste d'attente |
| Aucun dépassement de quota de publication | Trigger avec verrou consultatif par organisation |
| Aucun rappel envoyé deux fois | Index unique `(destinataire, session, décalage)` |
| Aucune réponse nominative sur un sondage anonyme | `CHECK` + trigger, drapeau rendu immuable |
| Aucun rattachement de média sur un rôle non déclaré | Table blanche `media.attachable_roles` + trigger |
| Aucun état incohérent entre une décision et sa notification | Outbox transactionnel |

---

## Ordre de lecture recommandé

1. **[CADRAGE.md](CADRAGE.md) §2** — le constat sur la v1. C'est ce qui justifie tout le reste.
2. **[CADRAGE.md](CADRAGE.md) §6** — les quatorze décisions d'architecture, en format court.
3. **[`040_organizations.sql`](database/040_organizations.sql)** — la réponse détaillée au problème n°1, avec ses quatre verrous.
4. **[`070_programme_proposals.sql`](database/070_programme_proposals.sql)** et **[`075_programme_sessions.sql`](database/075_programme_sessions.sql)** — le cœur métier.
5. **[`910_migration_v1.sql`](database/910_migration_v1.sql)** — la stratégie de reprise, et pourquoi la résolution des doublons doit précéder l'écriture.

---

## Par où commencer à construire

1. **[PROMPTS_DEVELOPPEMENT.md](PROMPTS_DEVELOPPEMENT.md), prompt A0.0**, puis **[ENVIRONNEMENT_LOCAL.md](ENVIRONNEMENT_LOCAL.md)** — initialiser le dépôt (`ops/`, `Makefile`, `.gitignore`), puis monter l'environnement local : Postgres avec le schéma chargé, Valkey, Garage, Mailpit, Jaeger.
2. **[PROMPT_STYLE_GUIDE.md](PROMPT_STYLE_GUIDE.md)** — produire le guide de style avec Claude Design, en récupérer les jetons CSS.
3. **[PROMPTS_DEVELOPPEMENT.md](PROMPTS_DEVELOPPEMENT.md), prompts A0.1 à A0.4** — le socle front, les types dérivés du modèle et les données simulées. C'est cette série qui fait le pont entre le SQL et l'interface : les écarts entre les deux apparaissent immédiatement à l'écran.
4. Puis les pages, **dans l'ordre du diagramme de dépendances** de [PROMPTS_DEVELOPPEMENT.md](PROMPTS_DEVELOPPEMENT.md) — A1 avant A2, A6 et A7 avant A8. Cet ordre ne se discute pas : chaque écran s'appuie sur le précédent.

   Deux d'entre eux méritent plus de soin que leur rang ne le laisse croire : **A2** (rattachement d'organisation), où se joue la qualité du référentiel, et **A8** (fiche d'évaluation), le plus dense — s'il tient, les autres tiennent. C'est l'attention qu'on leur porte qui change, pas leur ordre.

## Points en attente d'arbitrage

Détaillés en annexe du cadrage : reprise ou abandon de la messagerie directe, sens exact de « QCD » pour les quiz, source de référence pour le statut OIF des pays, durées de conservation RGPD, périmètre d'autorisation de l'assistant IA, hébergement du stockage objet.

**Ce qui commande le calendrier** : l'appel à propositions de la COP31 doit partir. Le jalon 1 ne contient que ce qui y concourt ; les autres modules affichent « En cours de maintenance » jusqu'à leur ouverture. L'échéance de novembre 2026 reste servie par la v1.
