# ePavillon v2

Plateforme numérique de l'Institut de la Francophonie pour le développement durable (IFDD), organe de l'Organisation internationale de la Francophonie.

Elle sert les conférences des Nations unies sur le climat, la biodiversité et la désertification : appels à propositions, sélection des activités par un comité, programmation du pavillon francophone, diffusion en direct, inscriptions. Elle porte aussi des webinaires, des formations et un espace réservé aux négociateurs francophones.

Cette version remplace une première plateforme en production depuis plusieurs années.

**Pile** : Nuxt 4 · Rust + Actix Web + SQLx · PostgreSQL 17 + pgvector · Garage (S3) · Valkey
**Architecture** : monolithe modulaire — un module = un schéma PostgreSQL = un crate Rust = une frontière de service potentielle.

---

## Démarrer

1. Monter l'environnement local — [docs/ENVIRONNEMENT_LOCAL.md](docs/ENVIRONNEMENT_LOCAL.md)
2. Produire le guide de style avec Claude Design — [docs/PROMPT_STYLE_GUIDE.md](docs/PROMPT_STYLE_GUIDE.md)
3. Lancer les prompts de construction, dans l'ordre — [docs/PROMPTS_DEVELOPPEMENT.md](docs/PROMPTS_DEVELOPPEMENT.md)

Avant chaque session : lire [docs/PROGRESSION.md](docs/PROGRESSION.md). Après chaque session : le mettre à jour.

---

## Structure

```
epavillon_v2/
├── CLAUDE.md              À lire en premier — conventions, règles métier, où trouver quoi
├── docs/
│   ├── PROGRESSION.md     La mémoire du projet entre deux sessions
│   ├── MODELE_INDEX.md    Quels fichiers SQL lire pour quelle tâche
│   ├── PROMPTS_DEVELOPPEMENT.md   Les prompts de construction, dans l'ordre
│   ├── PROMPT_STYLE_GUIDE.md      Le prompt du guide de style
│   ├── ENVIRONNEMENT_LOCAL.md     Services locaux, vérifications avant commit
│   ├── CADRAGE.md         Constat sur la v1, architecture, 14 décisions d'architecture
│   ├── CHARTE_GRAPHIQUE.md        Charte officielle IFDD — source des couleurs
│   ├── README.md          Vue d'ensemble du modèle de données
│   ├── database/          18 fichiers SQL — LA SOURCE DE VÉRITÉ du modèle
│   ├── logos-IFDD-OIF/
│   └── historique/        Ce que demandait le commanditaire, dans ses mots
├── ops/                   (à créer : docker-compose local, configuration Garage)
├── frontend/              application Nuxt        (créée par les prompts A)
└── backend/               workspace Cargo         (créé par les prompts B)
    ├── Cargo.toml
    ├── crates/            kernel · contracts · modules/ · api · worker
    └── migrations/
```

`backend/` et `frontend/` sont symétriques : chacun porte son gestionnaire de dépendances et ses commandes.

---

## Le modèle de données

`docs/database/` contient 18 fichiers SQL numérotés dans leur ordre de dépendance : **149 tables, 12 vues, 7 vues matérialisées, 145 fonctions, 14 schémas**. La chaîne complète est validée sur PostgreSQL 17 avec pgvector, le seed est rejouable, et les 167 clés étrangères inter-modules respectent la convention de nommage qui rend chaque module extractible.

Ces fichiers **font autorité**. Aucun nom de champ ne se devine : on lit le fichier concerné, repéré via [docs/MODELE_INDEX.md](docs/MODELE_INDEX.md). Chaque table est commentée en français directement en base.

Une bonne part des invariants métier est portée par le SGBD plutôt que par le code — index uniques, triggers, machines à états en données — parce que la leçon principale de la v1 est qu'une règle appliquée seulement par l'interface n'est pas appliquée. Avec une nuance assumée : tout ce qui est vrai n'a pas vocation à être bloqué. Les chevauchements de créneaux sont détectés et affichés, jamais refusés.

---

## Méthode de développement

1. **Le front d'abord**, sur des données simulées calquées sur le modèle. Un écart entre le modèle et ce que l'interface demande apparaît immédiatement à l'écran et se corrige tout de suite.
2. **L'API ensuite**, spécifiée avec GitHub Spec Kit à partir de ce que le front consomme réellement.
3. **Raccordement**, en tranchant chaque écart plutôt qu'en le masquant par une conversion.

Les prompts sont dans [docs/PROMPTS_DEVELOPPEMENT.md](docs/PROMPTS_DEVELOPPEMENT.md), un par écran et un par module. Chacun est conçu pour tenir dans une session et pour être repris là où la précédente s'est arrêtée : l'état d'avancement vit dans [docs/PROGRESSION.md](docs/PROGRESSION.md), pas dans le contexte de la conversation.

---

## Périmètre du jalon en cours

Ce qui permet de **lancer l'appel à propositions de la COP31** : authentification, organisations, événements, appel, soumission, espace organisation, back-office.

Les modules Publications, Négociations, Formations, Outils et Messagerie existent dans le modèle de données mais leur interface affiche « En cours de maintenance », commandée par un drapeau dans `platform.feature_flags`. Ils s'ouvriront un par un, sans redéploiement de l'ensemble.
# epavillon_v2
