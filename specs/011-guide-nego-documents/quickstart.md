# Recette — Guide Négo, étape 1

Ce qui prouve, de bout en bout, que le guide se publie en une journée et se lit en salle sans réseau. Les détails sont dans les contrats ([api-documents.md](contracts/api-documents.md), [api-admin-documents.md](contracts/api-admin-documents.md), [forme-lisible.md](contracts/forme-lisible.md), [hors-connexion.md](contracts/hors-connexion.md)) et dans le [modèle](data-model.md) ; on ne les recopie pas ici.

## Préalables

- **La base locale migrée, pas détruite** :
  - `psql "$DATABASE_URL" -f specs/011-guide-nego-documents/migration.sql`, joué **deux fois** ;
  - `pg_dump --schema-only` comparé au modèle chargé sur une base jetable ;
  - **jamais `make check` ni `down -v`**.
- **Le bucket privé** : `make garage-init`, qui le crée sans l'ouvrir au web.
  - Contrôle : `curl -I <base média>/epavillon-prive/<clé>` doit rendre une erreur, pas l'objet.
- **La dette de R4** : avant la bascule, aucun objet `private` ne doit rester dans un bucket ouvert. La requête doit rendre **0** ; sinon, déplacer ces objets avant la mise en ligne.

  ```sql
  SELECT count(*) AS objets_prives_dans_un_bucket_ouvert
    FROM media.assets a
   WHERE a.visibility = 'private'
     AND a.purged_at IS NULL
     AND a.bucket <> (SELECT s.value #>> '{}' FROM platform.settings s
                       WHERE s.key = 'media.private_bucket');
  ```

  Jouée en local le 23/09 : **0** (cinq objets, tous publics).
- **PDFium** : `make pdfium`, puis `PDFIUM_LIB_PATH` renseigné dans `.env`.
- **Les services** :
  - `cargo run -p api` et `cargo run -p worker` ;
  - le front **construit** et servi sur l'adresse exacte d'`APP_PUBLIC_URL`, parce que le service worker n'existe pas en développement (ADR-019).
- **Le PDF du guide de la CdP30** dans `.essais/`, hors Git.
- **Quatre comptes** :
  - une administratrice (`admin`, portée globale) ;
  - un expert (rôle `expert`, attribué depuis l'écran des utilisateurs) ;
  - une négociatrice admise ;
  - une personne inscrite sans accès.
- Et un navigateur **sans compte**.

## § 0 — L'essai (phase 1)

La grille d'[essai-extraction.md](essai-extraction.md) est remplie, ses mesures relevées, et l'issue A, B ou C écrite et relue avec le commanditaire.

## § 1 — Publier le guide (récit 1)

1. L'administratrice crée un brouillon, dépose le PDF et suit l'extraction jusqu'à « prête », **en chronométrant**.
2. Elle feuillette l'aperçu **page par page**, image et texte côte à côte : ordre de lecture, en-têtes absents, notes, tableaux en « page d'origine », césures, *global goal on adaptation* marqué. **Moins de dix minutes** pour tout le guide (SC-001 bis).
3. Elle choisit le type « Guide », aucune thématique, la COP31, la version, la date, l'éditeur, « remplace » l'ancien guide, public, marqueur coché. Elle publie. **Moins de quinze minutes au total** (SC-001).
4. Elle publie aussi le Bulletin, en **lien externe**. Elle tente d'y ajouter un fichier : c'est refusé, avec le message.
5. Elle tente de désigner un **second** remplaçant pour l'ancien guide : refusé, le remplaçant existant est nommé.
6. Elle bascule un document en « ouvrir tel quel », puis revient au mode recomposé, **sans republier**.
7. Un administrateur **d'événement** ouvre `/admin/negociations/documents`, puis forge l'adresse d'un document : il ne voit rien.

**Déroulé le 23/09 (T062)**, au navigateur piloté, API et worker branchés, sur le vrai guide (90 pages, 2,9 Mo) :

- **Dépôt → « Prête » : 35 s**, relecture de la fiche comprise ; forme lisible 3,6 Mo, 17 tableaux et 7 figures en page d'origine, 43 notes, 78 entrées de sommaire.
- **Feuilletage** : 89 tournes de page au clavier en **4 s**, image chargée à chaque page. L'outil ne compte pas dans les dix minutes de SC-001 bis : elles laissent 6,7 s de lecture par page.
- **Métadonnées, enregistrement, publication** : une minute au navigateur piloté. Le temps d'une personne reste à relever par l'IFDD à la recette de clôture ; rien, côté machine, n'approche les quinze minutes de SC-001.
- Étapes 4 à 7 conformes : lien publié, fichier refusé (`NEGOTIATION_DOCUMENT_FILE_LOCKED`, message tel quel) ; second remplaçant refusé en nommant le premier ; « tel quel » aller et retour, date de publication inchangée ; l'administrateur d'une édition reçoit 403 sur les douze routes et « Accès refusé » à l'écran, l'expert lit tout et n'écrit rien.

## § 2 — Trouver sans compte (récit 2)

1. Sans compte, ouvrir « Documents de négociation » : titre, compte, recherche, trois filtres.
2. Filtrer « Type : Bulletin » et « Thématique : Genre » : « Aucun bulletin sur le genre », puis « Retirer les filtres ».
3. Ouvrir la note technique de la CdP30 : le bandeau « Remplacé par » mène au guide de la CdP31.
4. Ouvrir le résumé réservé : titre et taille visibles ; le résumé, les thématiques et le téléchargement sont cachés ; trois sorties.
5. **Sécurité (SC-007)** :
   - sans compte, puis avec le compte sans accès, `GET /negotiation/documents/<réservé>/reading` et `…/pages/1/image` rendent `403` ;
   - l'objet du PDF n'est pas lisible par l'adresse du proxy média ;
   - la recherche `?q=` ne rend ni page ni extrait du réservé.
6. Chercher un mot présent **seulement dans le texte** du guide : trouvé, avec sa page.
7. « Nouveau » est jaune sur le guide ; ouvrir le guide ; revenir : la marque a disparu **sur ce téléphone**.

## § 3 — Lire en salle (récits 3 et 4) — le critère de sortie

1. **Sans compte**, télécharger le guide. La progression affiche « Téléchargement — n % ». Relever le poids réel de la copie, et le comparer à la mesure de l'essai.
2. Lire jusqu'à la page 59. Passer **en mode avion**. Fermer l'application, la rouvrir.
3. La bibliothèque dit « Hors connexion — lu à … » et « 1 lisible maintenant ». Les autres documents portent « Non téléchargé — disponible au retour du réseau » ; le Bulletin porte « Lien externe — réseau nécessaire ».
4. Ouvrir le guide : « Reprise à la page 59 — lue … ». Le pied de page dit « Page 59 sur 92 · 3.6 Adaptation ».
5. Sommaire, puis 3.6.1. Rechercher « progres collectifs » **sans accent** : « 5 passages dans 92 pages — sans réseau », avec « vous êtes ici ». Aller d'occurrence en occurrence.
6. Taille « Très grande » : le texte se recompose sans défilement horizontal et la page reste la 59. Thème sombre : l'application entière passe en sombre.
7. Toucher *global goal on adaptation* : la feuille s'ouvre, titrée du terme, **sans traduction ni définition**.
8. Ouvrir « Au nom de ma délégation », non téléchargé : « Ce document n'est pas sur votre téléphone », puis « Télécharger au retour du réseau ». Rétablir le réseau : le téléchargement part **une fois**, même après une fermeture.

## § 4 — La note de l'expert (récit 6)

1. L'expert ouvre l'aperçu du guide, page 59, sélectionne le passage sur les indicateurs et pose une note. Il ne peut ni publier ni modifier le document.
2. Sur le téléphone **qui a déjà la copie**, avec le réseau : la note paraît **sans retéléchargement**. Mode avion : elle est toujours là, filet rouge et triangle, puis dépliée avec la signature.
3. L'expert retire la note. Au retour du réseau, elle disparaît. Le back-office la montre retirée, avec l'auteur et la date du retrait.

## § 5 — Mes documents et ce qui s'efface (récit 5)

1. La négociatrice télécharge le guide et le résumé réservé, et met deux documents en favori, **dont un en mode avion**. Au retour du réseau, le favori part une seule fois.
2. Sur un second appareil : les mêmes favoris.
3. « Mes documents » montre les deux copies et leurs dates, la place utilisée et libre, et les favoris. « Tout retirer » : la confirmation nomme les documents et la place ; après « Retirer », **la place baisse aussitôt** (SC-005), et l'application s'ouvre toujours en mode avion.
4. Retélécharger les deux, puis **se déconnecter** : le message nomme les réservés. Après la déconnexion, le résumé réservé n'est plus lisible ; le guide public l'est (SC-006).
5. Retirer l'accès de la négociatrice depuis le back-office de 0b. À la relecture de son accès, le réservé s'efface.
6. **Couper l'API** (arrêter `cargo run -p api`) avec une copie réservée : **rien ne s'efface**.
7. Dépublier le Bulletin, puis une copie d'un autre document : à la relecture, elle s'efface.
8. « Ma journée », bloc « Documents récents » : le guide, avec sa dernière page lue.

## § 6 — Écrans et non-régression

- Chaque écran à 320, 360 et 390 px, dans les deux thèmes, et le lecteur dans ses trois tailles, sans défilement horizontal (SC-010).
- Les quatre états — chargement, vide, erreur, accès refusé — de la bibliothèque, de la fiche, de « Mes documents » et du lecteur.
- Traductions `en` relues. La planche des composants complétée des huit composants nouveaux.
- « 02 Ouverture » (écart 41), « À propos » et la déconnexion portent leurs nouveaux textes.
- **Le site** :
  - le dépôt d'une image par `ImageField` marche comme avant, puisque le dépôt en est sorti ;
  - les médias publics s'affichent toujours ;
  - `make check-api-contract` compte zéro route en attente.

## Les contrôles

```bash
cd frontend && npm run test:guide-nego && npm run check:guide-nego && npm run typecheck
make check-api-contract
cd backend && cargo test -p negotiation && cargo test -p media && cargo test -p api
make check-safe        # en fin de cycle seulement, API arrêtée — jamais make check
```

`grep` des libellés de types, de thématiques et de COP dans `frontend/i18n` et `frontend/app` : **zéro occurrence** (SC-011).

## Ce qui ne se fait pas depuis un poste

- Le téléchargement du guide en 3G bridée.
- La reprise après une nuit de veille.
- L'installation sur un Android et un iPhone réels, puis le guide lu en mode avion. C'est **la vraie preuve du critère de sortie**, et elle rejoint T112 de 0b, toujours due.
