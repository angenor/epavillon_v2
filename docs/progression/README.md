# progression/

Le détail de la mémoire du projet. Le point d'entrée reste [`../PROGRESSION.md`](../PROGRESSION.md) : il porte l'état général, le suivi des prompts et les liens. **On n'ouvre d'ici que le fichier utile à la tâche du jour.**

| Dossier ou fichier | Ce qu'il porte | Quand l'ouvrir |
|---|---|---|
| [`journal/`](journal/) | Une ligne par session, un fichier par jour | Pour savoir ce qu'a fait la session précédente |
| [`ecrans/`](ecrans/) | Un fichier par prompt front : ce qui a été livré, les écarts relevés entre le modèle et l'interface, ce qui a été vérifié et comment | Avant de reprendre ou de corriger un écran |
| [`decisions/`](decisions/) | Ce qui a été tranché en cours de route et pourquoi, un fichier par jour | Avant de refaire un choix déjà fait |
| [`modele.md`](modele.md) | Les modifications apportées à `docs/database/`, avec leur raison | Avant de toucher au SQL |
| [`api.md`](api.md) | Le suivi des prompts B et les obligations d'API relevées en écrivant les écrans | Au démarrage du back |
| [`environnement-local.md`](environnement-local.md) | Ce qui a été vérifié sur les services locaux, et les écarts avec `ENVIRONNEMENT_LOCAL.md` | Quand l'environnement ne démarre pas comme annoncé |
| [`pieges.md`](pieges.md) | Ce qui a déjà coûté une erreur, hors écran particulier | À relire avant de commencer un écran |
| [`points-bloques.md`](points-bloques.md) | Ce qui attend une réponse du commanditaire | Avant de trancher à sa place |

**Écrire ici, pas ailleurs.** Le fichier d'entrée ne reçoit que l'état général et la ligne de suivi du prompt ; tout le reste descend dans ces fichiers. Le mode d'emploi de fin de session est à la fin de [`../PROGRESSION.md`](../PROGRESSION.md).
