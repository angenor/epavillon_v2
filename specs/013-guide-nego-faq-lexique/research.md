# Recherche — étape 2 : FAQ, parcours et lexique

Chaque décision : ce qui est choisi, pourquoi, ce qui a été écarté.

## R1 — Où vit le savoir

**Décision** : schéma et crate `negotiation`, nouvelle section « §10 Savoir » en fin de `docs/database/100_negotiations.sql`. Côté Rust, un domaine `savoir` : `repo/savoir*.rs`, `service/savoir*.rs`, `routes/savoir.rs`, `routes/admin_savoir.rs`, `routes/admin_file.rs`, sans toucher aux fichiers de l'étape 1.

**Pourquoi** : 02-domaine.md range FAQ et lexique dans Négociations. Un nouveau crate serait une frontière sans consommateur. Les sources de FAQ pointent vers `negotiation.documents` : c'est une clé du même schéma.

**Écarté** : `tool` (outils) ou un crate `knowledge`.

## R2 — Un seul paquet « savoir », relu par différence

**Décision** : une route publique rend en un bloc la FAQ, le parcours et le lexique publiés, plus les deux vocabulaires et « les plus lues » : `GET /negotiation/knowledge`. Elle suit la forme de l'étape 1 — `ETag`, `If-None-Match` → `304`, `served_at` — et **ajoute `?since=`**, que 03-api.md demande et que l'étape 1 n'a pas eu besoin d'écrire :

- sans `since` : tout le corpus publié, `complete: true` ;
- avec `since` : les entrées changées depuis, et les identifiants de celles qui ne sont plus publiées (`removed`), `complete: false` ; le parcours, les vocabulaires et « les plus lues », petits, reviennent entiers ;
- **chevauchement de cinq minutes** : le serveur renvoie ce qui a changé depuis `since − 5 min`. Une transaction ouverte avant `served_at` et validée après aurait sinon un `updated_at` antérieur à `since`, et serait perdue. La fusion étant idempotente, un doublon ne coûte rien.

L'empreinte se calcule en SQL, sans sérialiser le corpus : `max(updated_at)` et le nombre de lignes de chaque table, plus le jour (pour « les plus lues »).

**Pourquoi un bloc** : 03-api.md (« le lexique et la FAQ se téléchargent d'un bloc ») et 01-stack.md (« en entier, rafraîchis à chaque ouverture avec réseau »). Quelques centaines d'entrées : ~300 Ko compressés au pire.

**Une entrée qui sort** : dépubliée, elle repasse en brouillon et son `updated_at` bouge ; elle part donc dans `removed`. **Un brouillon jamais publié se supprime ; une entrée publiée une fois ne se supprime jamais**, elle se dépublie (`ck_…` en base) — sinon le téléphone ne saurait pas qu'elle a disparu.

**Les enfants touchent le parent** : sources, questions liées, termes liés portent un déclencheur qui avance `updated_at` du parent. Sans cela, une source corrigée ne partirait pas dans la différence.

**Écarté** : trois routes (FAQ, lexique, parcours) — trois gardes et trois empreintes pour un seul geste de relecture ; la liste de l'étape 1 sans `since` — acceptable aujourd'hui, mais 03-api.md le demande et le corpus grossira.

## R3 — La garde sur le téléphone

**Décision** : `useGnSavoir()` bâti sur `useGnLecture('savoir', …)`. La valeur gardée porte le corpus fusionné, `served_at` et l'empreinte (comme `BibliothequeGardee`). La fusion d'une différence est une fonction pure, `utils/guide-nego/savoir.ts::fusionner(garde, difference)`, testée par `node --test`. Relu à chaque ouverture avec réseau, comme la bibliothèque.

**Hors connexion jamais lu** : la règle de 0a — l'écran dit que le contenu se chargera à la première connexion.

**Écarté** : un magasin IndexedDB par table — rien à y gagner pour quelques centaines d'entrées, et la garde existante sait déjà dire l'heure de lecture.

## R4 — La recherche sur le téléphone : maison, sans dépendance

**Décision** : `utils/guide-nego/recherche-floue.ts`, pur, sur le repli existant (`utils/guide-nego/repli.ts::replier` : casse, accents, apostrophes, tirets). Pour chaque entrée, un index préparé une fois à la lecture de la garde : terme, sigle, variantes, traduction (et, pour la FAQ, question puis réponse).

Rang, du meilleur au moins bon :
1. égalité exacte avec le terme, le sigle ou une variante ;
2. début du terme ou d'un mot du terme ;
3. sous-chaîne ;
4. **approché** : chaque mot saisi trouve un mot de l'index à une distance de Damerau-Levenshtein ≤ 1 (≤ 2 pour un mot de huit lettres ou plus), bornée et arrêtée dès le seuil dépassé.

« Vous cherchiez peut-être » paraît quand rien ne tombe dans les rangs 1 à 3 mais qu'un résultat approché existe. « Aussi dans les traductions françaises » paraît quand un résultat vient de la traduction.

**Coût** : 500 entrées × une dizaine de mots, distance bornée : quelques millisecondes par frappe sur un téléphone moyen. SC-004 se mesure au quickstart sur un lexique gonflé à 500 entrées.

**Écarté** : Fuse.js, MiniSearch — une dépendance pour 150 lignes, et leur normalisation ne serait pas celle du lecteur.

## R5 — La recherche côté serveur

**Décision** : `pg_trgm` et `unaccent` sont chargés (`000_bootstrap.sql`), et `platform.normalize_label()` existe déjà, `IMMUTABLE`, employée en colonne générée avec index `gin_trgm_ops` (`reference.countries`). Le lexique et la FAQ portent chacun une colonne générée `*_norm` indexée ainsi. Elle sert :
- la recherche du back-office ;
- **le rapprochement d'un terme proposé** : la file montre à l'expert les entrées existantes proches (`similarity ≥ 0,4`) ;
- **le regroupement des propositions** : une proposition dont la forme normalisée égale celle d'une proposition en attente s'y ajoute (index unique partiel).

Le téléphone ne questionne jamais le serveur pour chercher dans la FAQ ou le lexique : il a tout.

## R6 — La désignation stable et la résolution d'un texte

**Désignation** : `slug`, posé **une fois** à la création depuis le terme anglais par `platform.slugify(term)`, qui existe : `contact-group`, `bracketed-text`, `global-goal-on-adaptation`. Unique, jamais recalculé quand le terme est corrigé ; l'administratrice ne le modifie pas. Il sert d'adresse : `/guide-nego/lexique/contact-group`.

**Résolution d'un texte vers une entrée**, dans cet ordre, sur les formes normalisées (même règle des deux côtés : minuscules, sans accents, tout ce qui n'est ni lettre ni chiffre devient une espace, espaces réduites) :
1. le terme ;
2. le sigle ;
3. une variante d'écriture ;
4. le `slug` lui-même (un texte déjà désigné).

Pas d'approché : ouvrir la mauvaise entrée serait pire que dire « pas encore dans le lexique ».

**Le contrat, pour 1b et 3a** — [contracts/resolution-lexique.md](contracts/resolution-lexique.md) :
- **l'adresse** `/guide-nego/lexique?terme=<texte anglais>` : résout sur le téléphone, sans réseau ; ouvre l'entrée si elle existe, sinon la recherche préremplie avec « Proposer « … » » ;
- **la fonction** `resoudreLeTerme(texte, lexique)` de `utils/guide-nego/lexique.ts`, pour qui veut savoir avant d'ouvrir (la feuille du lecteur) ;
- **en base**, `negotiation.glossary_resolve(text) RETURNS uuid`, `STABLE`, même ordre : pour 3a si une ligne doit être rattachée côté serveur.

La normalisation client (`normaliserTerme`) et SQL (`platform.normalize_label`) sont vérifiées l'une contre l'autre par un test qui passe les mêmes vingt chaînes des deux côtés.

## R7 — Coches et favoris : un geste par ligne, pas un ensemble

**Décision** : coches du parcours et favoris du lexique s'écrivent **ligne à ligne**, `PUT` / `DELETE /negotiation/me/pathway/{step_id}` et `/negotiation/me/glossary-favorites/{entry_id}`, idempotents, par la file d'écritures de 0c en famille — le patron des favoris de documents (`useGnFavoris`). Les listes personnelles se relisent sous `ETag`.

**Pourquoi pas `If-Match` sur l'ensemble**, comme les thématiques : avec un ensemble et une empreinte, le premier appareil arrivé gagne et le second voit son geste rejeté par un `412` — la spec veut l'inverse (« le dernier geste reçu l'emporte »). Ligne à ligne, deux appareils qui touchent deux étapes différentes n'entrent jamais en conflit, et deux gestes sur la même étape se résolvent par le dernier arrivé. Ce qui reste à dire : après la relecture, si l'état du serveur diffère de ce que montrait l'écran, une ligne l'annonce (« Vos coches ont été mises à jour depuis un autre appareil »).

**Sans compte** : coches et favoris vivent dans la garde du téléphone (`localStorage`, clés `gn.parcours`, `gn.lexique.favoris`). **À la connexion**, chaque ligne locale absente du compte part en `PUT` ; rien n'est effacé ni d'un côté ni de l'autre. **À la déconnexion**, l'état du compte reste sur le téléphone comme état local — ce ne sont pas des données sensibles.

**Écarté** : `If-Match` sur l'ensemble (voir ci-dessus).

## R8 — Retours, signalements, questions, termes : une fois et une seule

**Décision** : chaque écriture porte une **référence choisie par le téléphone** (`client_ref uuid`), unique par auteur en base. Rejouée après un délai dépassé, elle rend `200` avec la ligne existante au lieu d'en créer une seconde. Elles passent par la file de 0c (`poser`, sans empreinte).

- « Oui / Non » : `PUT /negotiation/faq/{id}/feedback` — une ligne par personne et par entrée, écrasée ; pas de `client_ref`, le couple suffit.
- « Non » + « Dépassée ou fausse » : la même transaction ouvre un signalement marqué `from_feedback`, sans motif, une fois par personne et par entrée.
- « Dépassé ou faux » : `POST /negotiation/faq/{id}/reports`.
- Question : `POST /negotiation/me/questions`.
- Terme : `POST /negotiation/glossary/proposals`.

**Plafond** : vingt signalements et dix termes proposés par personne et par jour, compté par le service (ce n'est pas un invariant du modèle) ; au-delà, `429` à code stable.

**À la déconnexion**, les écritures en attente de la personne sont abandonnées (la file de 0c le fait déjà par personne) ; l'écran de déconnexion le dit s'il y en a.

## R9 — Anonymat

**Décision** : les tables gardent l'auteur (plafond, une voix par personne, courriel de la réponse), mais **aucune route du back-office ne le rend** : ni identifiant, ni nom, ni adresse. Le courriel part du worker, qui lit l'adresse dans la charge du travail, jamais l'écran. Un test d'intégration relit chaque réponse de la file et y cherche l'identifiant et le nom de l'auteur (SC-009). Une question promue donne un brouillon sans lien vers son auteur ; seul `origin_question_id` relie la question, dont l'auteur reste invisible.

## R10 — Permissions

**Décision** : deux permissions nouvelles, portée globale (`Requires<P>`), sur le patron de l'étape 1 :

| Permission | Geste | Rôles |
|---|---|---|
| `negotiation.knowledge.publish` | rédiger, publier, dépublier, mettre « À revoir » : FAQ, parcours, lexique | `admin`, `expert` |
| `negotiation.knowledge.review` | dater une vérification ; file des experts : signalements, questions (répondre, promouvoir), termes proposés | `expert` |

**Pourquoi l'expert seul pour `review`** : la date « Vérifié le » et la réponse à une question engagent un expert, et le modèle dit déjà que `admin` ne reçoit pas les permissions de l'expert (commentaire du §9). Une administratrice qui doit valider se fait attribuer le rôle `expert`. **Écart à la spec** : FR-037 disait « expert et administrateurs » pour la file — corrigé dans la spec.

Les lectures du back-office ouvertes à l'une **ou** l'autre passent par un extracteur sur le patron de `LectureDocuments`. Poser une question exige `negotiation.space.access` (l'accès négociateur) ; les autres écritures, une session (`Actor`).

## R11 — Les courriels

**Décision** : deux travaux dans `platform.jobs`, **file par défaut** comme les quatre travaux existants (une file que personne n'écoute s'empile sans erreur), mis en file dans la transaction qui les cause, idempotents, sur le patron de `jobs/emails.rs` de 0b : `negotiation.expert_question.answered_email` (à la réponse d'une question, clé `question_id`) et `negotiation.glossary_proposal.published_email` (à chaque auteur d'une proposition acceptée, quand l'entrée qui en naît est publiée, clé `(proposal_id, person_id)` : une republication ne renvoie rien). Gabarits dans `mail.rs`, `fr` et `en` selon la langue du compte, lien vers `/guide-nego/ressources/faq/mes-questions` ou `/guide-nego/lexique/<slug>`. **Enregistrés dans `job_handlers()`**, sinon ils ne tournent jamais.

## R12 — « Les plus lues »

**Décision** : `POST /negotiation/faq/{id}/read`, public, sans corps, hors de la file (une lecture perdue ne coûte rien), appelé à l'ouverture d'une entrée avec réseau, une fois par entrée et par jour et par téléphone (`localStorage`). En base, un compteur par entrée et par jour (`faq_reads`), sans auteur. Le paquet rend les trois plus lues sur trente jours ; sans lecture comptée, l'ordre éditorial (`editorial_rank`).

## R13 — L'accès à la recherche globale

La maquette dessine l'écran « 09 Recherche » mais pas le chemin qui y mène. **Décision** : une loupe dans l'en-tête, à gauche de « Aa », sur les écrans racines des onglets (« Ma journée », « Ressources ») ; ailleurs, l'en-tête garde sa forme. Écart à inscrire dans 05-design.md. La ligne de ce qui n'est pas couvert : « Pas encore ici : les sessions de négociation, les réunions de la Francophonie et le Pavillon. »

**Les documents** : avec réseau, `GET /negotiation/documents?q=` de l'étape 1 (titre, résumé, texte) ; sans réseau, la bibliothèque gardée (titre, résumé, éditeur) et `chercherDansLeDocument` sur le texte des copies gardées, bornée à cinq passages par document.

## R14 — La feuille du lecteur

`lire.vue` fait 806 lignes. La feuille du terme en sort dans `GnFeuilleTerme.vue`, qui prend le texte touché, le résout (R6) sur le savoir gardé, et montre l'entrée ou « pas encore dans le lexique » avec « Proposer ». `lire.vue` y perd des lignes au lieu d'en gagner.

## R15 — Les données d'essai

**Décision** : un fichier SQL rejouable, `specs/013-guide-nego-faq-lexique/donnees-essai.sql`, écrit à la main depuis `donnees-savoir.md` et `donnees-lexique.md` : quatre rubriques de FAQ publiées dont une entrée complète, les autres en brouillon si leur réponse manque ; 19 termes, les quatre « rédigés » sans source en brouillon — 15 publiés ; 18 étapes en quatre groupes. `INSERT … ON CONFLICT DO NOTHING`, joué par `psql -v ON_ERROR_STOP=1 -f`. **Hors de `docs/database/`** : il ne part jamais en production.

**Le vérificateur** : l'entrée publiée « Vérifié le » demande un vérificateur ; le script prend le premier compte portant le rôle `expert`, et s'arrête avec un message clair s'il n'y en a pas.

**Écarté** : lire le Markdown par un script — fragile, pour un chargement unique.

## R16 — La migration de la base locale

Comme aux étapes 0b à 1b : `specs/013-guide-nego-faq-lexique/migration.sql`, rejouable, blocs recopiés mot pour mot du modèle, joué deux fois puis `pg_dump --schema-only` comparé à une base chargée depuis `docs/database/`. Consigné dans `docs/progression/modele.md` (ADR-017). Jamais `down -v`.

## R17 — Découpage des fichiers

- `useApi.ts` (905 lignes) : deux lignes de montage ; les méthodes vont dans `composables/api/guide-nego-savoir.ts` et `admin-negotiation-savoir.ts`.
- Types : `types/negotiation-savoir.ts`.
- Mocks : `mocks/negotiation-savoir.ts`, tiré des mêmes données d'essai.
- i18n : un fichier par écran, `pages/guide-nego.faq.json`, `guide-nego.faq-entree.json`, `guide-nego.question.json`, `guide-nego.parcours.json`, `guide-nego.lexique.json` (existe), `guide-nego.lexique-entree.json`, `guide-nego.lexique-liste.json`, `guide-nego.recherche.json` ; back-office `admin.negociations.faq.json`, `.lexique.json`, `.parcours.json`, `.file.json`.
