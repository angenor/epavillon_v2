# B8 — Le module Vitrine (`content`)

> Extrait de la [progression](../../PROGRESSION.md).

**État** : ✅ 24/08

---

## Ce qui a été livré

**Le crate `content` existe** — 2 201 lignes, 11 routes. Le schéma `content` était en base depuis le 19/08 (tables, vue, permission, module déclaré) ; aucun code ne le servait, et c'est ce qui faisait qu'une diapositive retirée du back-office réapparaissait au rechargement : l'écran écrivait en mémoire.

| Chemin | Ce qu'il sert |
|--------|---------------|
| `GET /home` | La vitrine publique — le bandeau d'ouverture, dans l'ordre de défilement |
| `GET /admin/showcase` | La liste, ses facettes et ses référentiels |
| `GET /admin/showcase/new` · `GET /admin/showcase/{id}/form` | L'écran de formulaire, avec son aperçu |
| `GET /admin/showcase/{id}` | Les valeurs seules |
| `GET /admin/showcase/sessions` | Les séances d'une édition — la cascade du formulaire |
| `POST /admin/showcase` · `PATCH /admin/showcase/{id}` | Créer, modifier |
| `POST /admin/showcase/{id}/status` | Publier, retirer, archiver |
| `POST /admin/showcase/{id}/order` | Monter, descendre |
| `POST /admin/showcase/{id}/duplicate` | Dupliquer, en brouillon |

**Le site est branché** : `composables/api/home.ts` et `composables/api/admin-showcase.ts` passent de `pending` à `call`/`send`. Les routes en attente d'API tombent de 19 à 8.

## Ce qui a été tranché

1. **L'aperçu ne passe pas par `content.v_showcase`.** La vue ne rend que le publié dans sa fenêtre ; l'éditeur, lui, compose un brouillon. `repo::showcase::apercu()` résout donc les mêmes jointures sur les **valeurs en cours de saisie** — c'est ce qui permet à l'aperçu d'être rendu par le composant qui sert l'accueil, et non par une seconde mise en page.
2. **`broadcast_state` n'est pas `status`.** Une diapositive publiée dont la fenêtre s'ouvre plus tard est `scheduled` ; close, elle est `expired`. La liste dit ce que le public voit, pas seulement ce que l'éditeur a décidé.
3. **L'ordre se renumérote, il ne s'échange pas.** Monter d'un cran échange deux rangs, puis l'emplacement entier repart à 10, 20, 30. Sans cette remise à plat, une insertion finit par tomber sur un rang déjà pris et l'ordre devient celui de `id` — le défaut du carrousel de la v1.
4. **`event_id` nul est la contrainte la plus forte, pas l'absence de contrainte** : un contenu de plateforme n'est visible et modifiable qu'en portée globale. Le refus le **dit** (403 avec message), là où un hors-périmètre ordinaire rend 404 — l'écran doit pouvoir expliquer pourquoi une ligne visible n'est pas modifiable.
5. **Les refus de validation sortent en 200**, avec leur champ et leur code, comme le contrat du site le prévoit. Neuf codes, dont cinq doublent une contrainte de `115_content.sql` : la base reste l'autorité, le code rend le refus exploitable par le formulaire.

## Le semis

**Trois diapositives réelles en base, deux images et une vidéo dans le stockage.** Les textes sont ceux des données d'exemple du dépôt — écrits, relus et éprouvés à l'écran depuis le 19/08 : les migrer plutôt que d'en inventer d'autres garde ce qui a été validé. `event_id` nul, la base ne portant aucune édition : ce sont des contenus de plateforme, ce qui est exact.

Les fichiers sont passés par **l'API réelle** (`POST /media/assets` puis `POST /media/attachments`), pas par une insertion : le worker a fabriqué les trois déclinaisons de chaque image, et les objets sont `ready`. Une quatrième image sert d'affiche à la vidéo — c'est la vignette du rail.

**Un compte de service a été créé pour cela** (`semis.vitrine@ifdd.francophonie.org`), activé et promu à la main, motif inscrit dans `identity.role_assignments.note` : le relais de courriel passe par le serveur du site, arrêté pendant le semis, et le jeton d'activation est haché en base.

## Ce que le semis a révélé

**Aucun média téléversé n'était visible en local.** Le modèle compose l'URL d'un objet en chemin — `<base>/<bucket>/<clé>` —, ce que sert n'importe quel stockage en « path-style » et ce que fera le domaine média en production. Garage, lui, n'ouvre ses objets à la lecture anonyme que par **sous-domaine**, et son API S3 exige une signature.

D'où `ops/media-proxy.conf` et le service `media-proxy` du compose : un relais qui traduit le chemin en sous-domaine, et rien d'autre. `media.public_base_url` pointe dessus **dans la base de développement seulement** — la valeur du modèle reste celle de production.

## Vérifié

- `cargo fmt --check`, `cargo clippy --workspace -D warnings`, **858 tests au vert**, `npm run typecheck`, `npm run build`, `make check-db-safe`, `make check-api-contract`.
- **Contre l'API réelle** : la liste, le retrait, l'ordre (renumérotation vérifiée en base), la duplication en brouillon, et quatre refus de validation en une requête — français manquant, libellé de lien sans lien, couleur mal formée, fenêtre inversée.
- **Au navigateur, connecté** : l'accueil affiche les trois diapositives réelles, la vidéo joue en fond, le rail montre les vraies vignettes ; le back-office liste les trois lignes, et **le retrait tient au rechargement** — ce qui était le point de départ.
- Le menu d'actions ne réagit pas au clic **simulé** par l'outil d'automatisation ; il répond au clavier, chemin par lequel le retrait a été éprouvé. À revérifier à la souris.

## Ce qui reste

- **Aucun test d'intégration Rust** pour ce module : les 858 tests sont ceux des modules livrés. Les onze routes ont été éprouvées contre l'API réelle, à la main.
- Le **téléversement depuis le formulaire** n'existe pas encore : les emplacements de média sont rendus avec leurs contraintes et ce qui y est rattaché, mais joindre un fichier depuis l'écran reste à écrire.
- Les deux autres écrans en attente d'API — messages d'incident (`live`), tableau de bord (`analytics`) — n'ont pas bougé.
