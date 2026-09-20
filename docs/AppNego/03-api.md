# 03 — API

> **Un plan, pas un contrat.** Le contrat est engendré par `make openapi` depuis les routes Rust et vérifié par `make check-api-contract` ; ce fichier dit quelles familles de routes chaque module apportera. Les chemins sont proposés et se fixent à la spécification de chaque module.

## Principes

- **La même API que l'ePavillon**, les mêmes conventions : erreurs en français affichées telles quelles, permissions testées avec leur portée, `app.actor_id` et `app.request_id` posés à chaque écriture, effets de bord par `platform.emit_event()`.
- **Une famille `/negotiation`**, pour ne jamais croiser `/sessions` et `/schedule`, qui servent déjà les activités du Pavillon — la confusion des trois agendas commence dans les chemins.
- **Pensée pour le hors-connexion** : chaque liste accepte « modifié depuis », répond avec une empreinte (`ETag`) et l'heure du serveur ; le lexique et la FAQ se téléchargent d'un bloc.
- **Lecture publique, écriture gardée** : ce qui est ouvert à tous se lit sans session ; le reste exige la permission de négociateur.
- **Le service d'IA n'a aucune route publique** : l'API Rust relaie.

## Familles de routes par module

| Module | Lecture | Écriture |
|---|---|---|
| Socle | Mon accès, mes thématiques suivies, l'état des modules | Utiliser un code d'invitation ; demander l'accès ; suivre ou quitter une thématique ; enregistrer un appareil |
| Documents | Liste filtrée, fiche, adresse de téléchargement signée, mes favoris | Favori ; compteur de téléchargement |
| FAQ et lexique | Rubriques, entrées, recherche ; export complet du lexique et de la FAQ | « Utile », « Dépassé ou faux » ; proposer un terme ; poser une question à un expert |
| Sessions de négociation | Sessions d'un jour, par thématiques ; détail ; état de l'import ; mon agenda | Suivre une session ; signaler un changement ; mes signalements |
| Réunions de la Francophonie | Liste, détail | Inscription |
| Pavillon de la Francophonie | **Existant** : `GET /schedule`, le détail d'une séance, `/registrations` | Existant |
| Échanges | Canaux, messages, non-lus, annuaire, conversations privées | Envoyer, réagir, signaler un message, proposer un document ; flux temps réel |
| Assistant | Mes conversations, une réponse en flux avec ses citations, mon quota | Poser une question ; « Dépassé ou faux » |
| Formations et quiz | Modules, vidéo, transcription ; quiz, mes tentatives | Répondre ; régénérer un quiz pour moi ; proposer à la relecture |
| Restitutions | Les miennes, celles de mes cercles | Rédiger, choisir le cercle |

## Back-office

Sous `/admin`, avec le filtrage par périmètre d'administration déjà en place : documents (publier, remplacer, réserver, marquer « utilisable par l'assistant »), FAQ et lexique, codes d'invitation et demandes d'accès, mode d'admission, file des signalements, interrupteur et journal de l'import, corpus de l'assistant (arrivée, référence, états, notes de correction), quiz (génération, relecture), canaux et modération, mesures d'usage.

## Entre l'API Rust et le service d'IA

| Sens | Moyen | Pour |
|---|---|---|
| Rust → IA | File d'événements (outbox) | Indexer un document ou une transcription promus en référence ; retirer une source dépassée |
| Rust → IA | Appel interne, réponse en flux | Répondre à une question ; générer un quiz ; proposer un classement ; traduire un titre de session |
| IA → base | Écriture dans le seul schéma `tool` | Morceaux, vecteurs, conversations, citations |
| IA → Rust | Événement de retour | « Indexé », « échec », « quiz proposé » |

Le service d'IA ne lit aucun schéma métier : tout ce dont il a besoin lui arrive dans l'événement ou l'appel, fichier compris par sa référence de stockage.

## L'import des sessions de négociation

Un travail récurrent du worker lit la source officielle, compare, et n'écrit que les écarts. Il tient un état lisible par l'application : dernière lecture réussie, dernière tentative, nombre d'écarts. **Passé un seuil réglable de lectures manquées, l'affichage se coupe seul** et laisse le lien vers le programme officiel — [ADR-009](adr/009-la-source-officielle-fait-foi.md). Le mécanisme se construit et s'éprouve sur les données archivées de la COP30, sans attendre l'accord du secrétariat.

## Codes d'erreur

Le catalogue stable de l'ePavillon s'étend ; il ne se double pas. À prévoir : code d'invitation inconnu, expiré, révoqué, épuisé ; trop d'essais ; module réservé ; quota de l'assistant atteint ; source officielle illisible ; signalement en double.
