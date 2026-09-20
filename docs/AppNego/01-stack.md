# 01 — Pile et architecture

> Guide Négo n'ajoute pas une pile : il ajoute à celle de l'ePavillon un client mobile, deux crates et un service d'IA. Tout ce qui n'est pas dit ici est dans le [CLAUDE.md](../../CLAUDE.md) du dépôt et s'applique tel quel — SQL d'abord, contrat d'API engendré, i18n, quatre états par écran, mille lignes au plus par fichier.

## Vue d'ensemble

```
Guide Négo — Nuxt, installable, hors connexion
      │
   API Rust — sessions, droits, quotas ──── PostgreSQL, la base de l'ePavillon
      │  file de travaux et d'événements
      ├── Service IA Python, interne — schéma tool + pgvector ──── OpenRouter
      └── Worker Rust — import des sessions de négociation, notifications
```

## Les pièces

| Pièce | Choix | Décision |
|---|---|---|
| Client | Le Nuxt 4 existant : un sous-arbre de pages dédié (`frontend/app/pages/guide-nego/`, composants dans `frontend/app/components/guide-nego/` — noms proposés), une quatrième mise en page, rendu côté navigateur, application web installable (PWA) | [ADR-000](adr/000-guide-nego-est-la-face-mobile-de-l-epavillon.md), [ADR-002](adr/002-web-installable-d-abord-capacitor-avant-les-echanges.md) |
| Hors connexion | Service worker ; fichiers dans le cache du navigateur, données dans IndexedDB | [ADR-003](adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) |
| Magasins | Capacitor, avant le module Échanges ; comptes Apple et Google déjà ouverts | [ADR-002](adr/002-web-installable-d-abord-capacitor-avant-les-echanges.md) |
| API | Rust, Actix, SQLx — la même. Nouveaux crates `negotiation`, puis `training` | [03-api](03-api.md) |
| Base | PostgreSQL 17 + pgvector — la même. Schémas `negotiation`, `training`, `tool`, et ceux déjà servis | [02-domaine](02-domaine.md) |
| Fichiers | Garage (S3), par `media.assets` | — |
| Éphémère | Valkey : essais de code d'invitation, fraîcheur de l'import | — |
| Travaux | Le worker Rust existant : import des sessions de négociation en chaîne récurrente, notifications, relais d'outbox | [ADR-009](adr/009-la-source-officielle-fait-foi.md) |
| IA | Service Python (FastAPI), interne, jamais exposé ; il possède le schéma `tool` | [ADR-004](adr/004-rust-en-facade-python-au-sidecar.md) |
| Modèles | OpenRouter : Gemini pour rédiger, voyage-4 pour vectoriser | [ADR-005](adr/005-openrouter-et-embedding-versionne.md) |
| Notifications | Courriel et alertes dans l'application d'abord ; notifications poussées avec Capacitor | — |
| Comptes | Un seul compte ; cookie sur le web, jeton dans la coquille Capacitor | [ADR-001](adr/001-un-seul-compte-la-session-distingue-le-mobile.md) |

## Hors connexion : ce qui est gardé sur le téléphone

| Contenu | Règle |
|---|---|
| Coquille de l'application, polices, pictogrammes | Toujours, dès l'installation |
| Lexique et FAQ | En entier, rafraîchis à chaque ouverture avec réseau |
| Sessions de négociation, réunions de la Francophonie, activités du Pavillon | Les jours de l'événement en cours, avec l'heure de dernière lecture |
| Documents | À la demande ; la place occupée est visible et se libère en un geste |
| Favoris, thématiques suivies, signalements en attente d'envoi | Toujours ; envoyés au retour du réseau |
| Documents réservés | Effacés du téléphone à la déconnexion |

Une donnée lue hors connexion dit **quand** elle a été lue. L'assistant et les échanges sont les seuls à exiger le réseau.

## Pièges connus

- **Le préfixe `/v2`.** Il a déjà produit trois échecs silencieux (cookies, origine autorisée, chemins publics). La portée du service worker et le point de départ du manifeste en sont les prochains candidats : tout dérive d'`APP_PUBLIC_URL`.
- **La coquille Capacitor change d'origine.** La session par cookie n'y passe plus, et l'origine doit être autorisée par l'API. D'où la session par jeton, réservée à ce client.
- **La dimension des vecteurs.** `tool.knowledge_chunks.embedding` est figé à 1536 ; voyage-4 en produit 1024. À corriger dans le SQL, avec une colonne nommant le modèle d'origine, **avant le premier document indexé**.
- **Le drapeau de module.** `negotiation.enabled` ferme aujourd'hui `/negociations`. Guide Négo a des parties ouvertes à tous : il lui faut sa propre entrée au registre `frontend/app/utils/feature-modules.ts`, et son drapeau semé en base.
- **Deux designs sous un même toit.** Guide Négo a son propre système — jetons, composants, polices —, rangé dans son dossier et borné à sa mise en page. Il ne touche à aucun jeton du site, et n'emprunte aucun de ses composants d'interface.
- **Ne jamais lancer `make check`** : il détruit la base locale. `make check-safe` avant tout commit.

## Ce qui se réutilise de l'ePavillon

Le compte et la session, le contrôle des droits par permission et par portée, le téléversement et le service des fichiers, les taxonomies, les notifications et leurs préférences, les activités du Pavillon (`GET /schedule` et les inscriptions), le worker et ses chaînes récurrentes, `useApi()` et ses quatre primitives, le client TypeScript engendré. La bibliothèque de composants du site ne sert qu'aux écrans de back-office : **l'application a son propre design** — [ADR-016](adr/016-vert-et-jaune-fonces-police-hors-charte.md).

## Méthode

Un module = une spécification Spec Kit (`/speckit-specify`, `-plan`, `-tasks`, `-implement`), dans l'ordre de la [feuille de route](04-roadmap.md). Pour chacun : le SQL d'abord, puis la maquette validée, puis le code. Le suivi vit dans [progress.md](progress.md), pas dans la progression de l'ePavillon.
