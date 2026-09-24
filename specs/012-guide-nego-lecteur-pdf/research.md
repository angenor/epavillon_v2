# Recherche — Guide Négo, le lecteur montre le PDF d'origine (étape 1b)

**Entrées** : [spec.md](spec.md), la consigne de plan sous l'étape 1b de [04-roadmap.md](../../docs/AppNego/04-roadmap.md), et trois états des lieux menés le 24/09 : le back livré à l'étape 1, le front livré à l'étape 1, et `pdfjs-dist` 6.3.289 lu dans son archive.

Chaque décision porte son **pourquoi** et ce qui a été **écarté**. Les renvois `R…` de l'étape 1 désignent [../011-guide-nego-documents/research.md](../011-guide-nego-documents/research.md).

---

## R1 — Le rendu : pdf.js, son visionneur de pages, et le build « legacy »

**Décision.**
- **`pdfjs-dist` 6.3.x** (Apache-2.0), épinglé à une version exacte. Il y a une version par mois, et l'API refuse de tourner avec un visionneur d'une autre version.
- **Le build `legacy`** partout. Le build moderne appelle `Math.sumPrecise` sans polyfill au rendu des polices, et `Math.sumPrecise` n'existe qu'à partir de Safari 26.2. Le legacy embarque ses polyfills pour 40 Ko compressés de plus, et couvre Safari 18 et Chrome 125.
- **`PDFViewer` et ses composants** (`pdfjs-dist/web/pdf_viewer.mjs`), plutôt qu'un rendu écrit ici. Il règle déjà ce qu'on écrirait mal :
  - il ne garde rendues que les pages visibles et leurs voisines (`PDFPageViewBuffer`) ;
  - il plafonne un canevas à 5,2 Mpx sur iOS et Android (`maxCanvasPixels`) ;
  - au-delà du plafond, il étire la page puis redessine net **la seule zone visible** (`enableDetailCanvas`) — c'est ce qui tient « net à tout grossissement » sans dépasser les 16,7 Mpx par canevas d'iOS ;
  - il pose la couche de texte, sélectionnable.
- **Le pincement** passe par le `TouchManager` exporté, relié à `updateScale({ scaleFactor, origin })`. **Le double toucher** et **le toucher simple** sont écrits ici (R6).
- **Un polyfill de quelques lignes** : `ReadableStream.prototype[Symbol.asyncIterator]`. `getTextContent()` itère un flux en `for await`, sans polyfill même en legacy, et l'itération n'est sûre qu'à partir de Safari 27 (un échec est signalé sur iOS 26.5).
- **La décision se consigne dans l'ADR-022**, qui remplace « le PDF ne va jamais sur le téléphone » d'[ADR-021](../../docs/AppNego/adr/021-pdfium-dans-le-worker.md), après l'essai (R2). Une dépendance d'ampleur l'exige (constitution, contraintes techniques).

**Écarté.**
- **Le visionneur du téléphone** : une application installée sur iPhone n'en revient pas proprement, et ni les notes ni le passage cherché n'y paraissent (spec, *Assumptions*).
- **Un rendu écrit ici**, page par page sur canevas : il faudrait refaire la virtualisation, la libération des canevas, l'étirement pendant le geste puis le redessin net de la zone visible. Les retours d'usage (pdf.js #5342, react-pdf #1601 et #1691) montrent qu'on y perd la mémoire d'iOS.
- **EmbedPDF** (PDFium en WebAssembly, MIT) : fidèle, puisque c'est le moteur de Chrome, mais 4,6 Mo de WebAssembly (2,1 Mo compressés), un projet jeune, et la même contrainte de canevas sur iOS. Aucun avantage décisif.
- **`vue-pdf-embed`** rend toutes les pages, ce qu'on ne peut pas faire sur 90 pages. **`@tato30/vue-pdf`** laisse la mémoire et le zoom à notre charge.
- **Le build moderne** : il faudrait deux livraisons, ou exclure tout iPhone sous iOS 26.2.

**Le plancher des appareils.** Le legacy exige **Safari 18** (iOS 18, disponible depuis l'iPhone XS) et **Chrome 125**. Un Android 8 ou 9 reste à Chrome 138 et passe.
- Un iPhone bloqué en iOS 16 ou 17 — iPhone 8 et X — **n'affiche pas les pages**.
- Le lecteur le détecte à l'ouverture, par l'échec du chargement de pdf.js ou d'un rendu. Il ouvre alors « Texte agrandi » si le document l'offre, et le dit en une ligne. Sinon, il dit que ce téléphone n'affiche pas les pages de ce document.
- **Point à porter au commanditaire** ; voir *Points ouverts*.

---

## R2 — L'essai d'abord, sur le vrai guide

**Décision.** Rien ne s'écrit avant **un prototype jetable** : une page de Guide Négo, construite (`nuxt build`), qui lit le vrai guide de la CdP30 par `PDFViewer`, depuis une copie en Cache Storage, en mode avion. Il se mesure sur une grille, comme l'essai d'extraction de l'étape 1 ([essai-lecteur.md](essai-lecteur.md)).

| Critère | Seuil | D'où |
|---|---|---|
| Défilement des 90 pages, CPU bridé ×4, profil mobile | Aucune tâche longue de plus de 100 ms pendant le geste ; aucune page blanche plus d'une seconde après l'arrêt | SC-001, SC-002 |
| Mémoire après un aller-retour complet | Stable : les canevas sont libérés (`width = height = 0`) et le tas ne croît pas d'un tour à l'autre | Edge case mémoire, FR-010 |
| Grossissement ×4 sur le tableau des sigles | Net en moins d'une seconde après le geste, sans erreur de canevas | FR-004, SC-002 |
| Première page en ligne, réseau « 3G lente » | Moins de 8 s ; les requêtes sont des `206` de 256 Ko | FR-009, SC-003 |
| Dix expressions du guide | Chaque passage repéré sur sa page par la couche de texte (R7) | SC-004 |
| Couche de texte, page 59 | Sélection d'un paragraphe à deux colonnes, collé dans l'ordre | SC-006 |
| Hors connexion | Le travailleur de pdf.js, ses `wasm` et ses polices sont servis par la coquille, **y compris le XHR synchrone qu'émet le travailleur** — vérifié sous Chrome, **à vérifier sous WebKit** | FR-043 |
| Polices et images du guide | `pdffonts` et `pdfimages -list` : polices non incorporées, JPEG 2000, JBIG2, polices CID | R4 |

**Où.**
- Chrome sur poste, en émulation mobile, pour les mesures.
- Safari du Mac, puis le simulateur iOS s'il est installé, pour WebKit.
- **Sur appareil réel** : un Android de milieu de gamme et un iPhone, par le commanditaire, sur une construction servie, comme T112. Ce qui ne se fait pas sur poste est écrit tel quel dans la grille.

**Les issues.**
- **A** — tout passe : ADR-022, et la suite.
- **B** — WebKit ne sert pas les `wasm` au XHR synchrone hors connexion. Le lecteur passe `wasmUrl` à `null` hors connexion. Les couleurs ICC s'approximent, un JPEG 2000 ne se rend plus, et l'essai dit si le guide en a.
- **C** — la mémoire ou la fluidité ne tiennent pas sur Android milieu de gamme : on baisse `maxCanvasPixels` et le tampon de pages, puis on remesure. Si rien ne tient, on revient au commanditaire avant d'aller plus loin.

---

## R3 — Servir le PDF : une route de l'API, par plages, pour le public comme pour le réservé

**Constat.**
- **Tous** les PDF de documents sont dans le bucket privé, publics compris : la garde média l'impose (`media/src/domain/guards.rs:145-172`).
- Le relais `/v2/media/` ne sert que le bucket public.
- Aucune lecture par plage n'existe : `ObjectStore::get` rend l'objet entier (`kernel/src/storage/mod.rs:96`), et rien dans le dépôt ne gère `Range`, `206` ni `Accept-Ranges`.
- Seul le back-office sert le fichier, en entier (`GET /admin/negotiation/documents/{id}/file`).

**Décision.**
- **Une route publique** `GET /negotiation/documents/{id}/file`, pour tout document fichier publié, et le même `HEAD`. Le contrat est dans [contracts/api-lecture.md](contracts/api-lecture.md).
- **L'accès se vérifie à chaque requête**, morceau par morceau, par `lisible()`, qui existe (`service/documents.rs:181`) : publié, sinon `404` ; réservé sans accès, `403` ; lien, `409`. Un réservé répond `Cache-Control: private, no-store`, un public `private, max-age=3600` (FR-029, FR-030).
- **Les plages** :
  - `Range: bytes=a-b` rend `206` avec `Content-Range` ;
  - sans `Range`, `200` et `Content-Length` : c'est le téléchargement de la copie ;
  - une plage hors du fichier rend `416`, avec un code nouveau ;
  - toujours `Accept-Ranges: bytes`, jamais de `Content-Encoding`, toujours `Cache-Control: no-transform`. pdf.js coupe les plages en silence si la réponse est compressée ;
  - `ETag` fort, tiré de l'identifiant du fichier : un fichier publié ne change jamais en place (FR-007 de l'étape 1).
- **`kernel::storage` gagne `get_range(key, debut, fin)`**, qui s'appuie sur le `GetObject` avec `Range` de S3, et sur le système de fichiers pour les tests. `head` existe déjà pour la taille.
- **En local**, le site et l'API sont sur deux origines. Le contrôle d'origine maison (`api/src/lib.rs:252`) accepte l'en-tête `Range` en préflight et expose `Content-Range`, `Accept-Ranges` et `Content-Length`. En production, le site et l'API partagent l'hôte.
- **Apache** ne compresse pas `application/pdf` sous `/v2/api/`. La configuration de Debian ne le fait pas, mais on le vérifie au § 15 de DEPLOIEMENT.md.

**Écarté.**
- **Servir le public par le média**, comme la feuille de route l'envisageait : il faudrait une seconde copie dans le bucket public, à déplacer chaque fois qu'un document passe de public à réservé. Ce serait un invariant de plus pour un gain nul, avec quelques dizaines de documents et des lectures par morceaux de 256 Ko.
- **Une adresse signée vers Garage** : le relais public ne la connaît pas, et elle vivrait après une déconnexion.

---

## R4 — Ce que pdf.js demande hors connexion, et comment la coquille le garde

**Constat.**
- La coquille est un service worker **écrit ici**, sans Workbox. Sa liste de garde est la fermeture du manifeste Vite depuis les racines de Guide Négo, plus une courte liste de fichiers publics (`frontend/guide-nego/liste-de-garde.ts:21-104`).
- Un fichier importé par `?url` y entre. Un fichier désigné par une adresse construite à l'exécution n'y entre pas.

**Décision.**
- **Le module et son travailleur** s'importent depuis un utilitaire de Guide Négo : `pdf.min.mjs` dynamiquement, `pdf.worker.min.mjs` par `?url`. Ils entrent dans la garde par le manifeste, sans rien écrire de plus.
- **Les ressources désignées par adresse** sont copiées au build dans `public/guide-nego/pdfjs/`, par un module Nuxt, `modules/guide-nego-pdfjs.ts`, sous une adresse stable, et ajoutées à `FICHIERS_PUBLICS` :
  - `wasm/qcms_bg.wasm` et `iccs/` : les couleurs ;
  - `wasm/openjpeg.wasm` : le JPEG 2000 ;
  - `standard_fonts/` : les quatorze polices standard, qu'Android ne remplace pas.
- `cmaps/` (1,6 Mo) et `jbig2.wasm` n'entrent **que si l'essai les trouve dans le guide** ; `quickjs` jamais.
- **La coquille grossit d'environ 1,1 Mo compressé.**

| Fichier | Poids compressé |
|---|---|
| Module et travailleur | environ 540 Ko |
| Visionneur et sa feuille de style | environ 110 Ko |
| `wasm` et ICC | environ 250 Ko |
| Polices standard | environ 200 Ko |

  L'installation reste en tout ou rien : `addAll`, ADR-019.
- **La feuille de style de `pdf_viewer.css`** déclare des variables sur `:root` et des sélecteurs globaux, que la constitution interdit (XIII) et que `check:guide-nego` refuse. Le module en écrit donc au build une **copie bornée** : chaque sélecteur est préfixé par `[data-app="guide-nego"]`, et `:root` devient la borne. Le contrôle vérifie la copie comme toute autre feuille.
- **`FICHIERS_PUBLICS` et le test `liste-de-garde`** gagnent les chemins de `pdfjs/`, et **`verifier-garde:guide-nego`** prouve qu'ils sont servis.

**Écarté.**
- **Charger pdf.js depuis un CDN** : il ne serait pas là en mode avion.
- **Importer la feuille du visionneur telle quelle** : elle sortirait de la borne et toucherait le site.

---

## R5 — La copie gardée : le PDF et la lecture, sous un numéro de format

**Constat.**
- La copie de l'étape 1 est une entrée `DocumentReading` (le JSON de lecture), plus des images de page, dans `gn-documents-publics` ou `gn-documents-reserves`.
- Sa fiche est rangée dans IndexedDB `guide-nego` v3, magasin `copies` (`utils/guide-nego/copies.ts:32-46`). Elle n'a **aucun numéro de format**.

**Décision.**
- **Contenu** : l'entrée de lecture (`…/reading`) et le PDF (`…/file`), sous leur adresse d'API, dans le même cache qu'aujourd'hui. **Plus aucune image.**
  - La lecture est toujours gardée : même sans texte extrait, elle porte les numéros imprimés des pages et leur nombre.
  - Le PDF se lit par un `GET` entier, en `200`. `cache.put` refuse un `206`.
- **La fiche `Copie` gagne `format: 2`**, et perd `pages_images`. `cles` vaut `[lecture, pdf]`, et `octets` la somme des deux.
  - **Une fiche sans `format`, ou d'un format autre que celui que l'application lit, s'efface** à la vérification d'ouverture (`verifierLesCopies`, lancée par la mise en page) avec ses entrées. Le document redevient « Non téléchargé » (FR-031).
  - Le prochain changement de copie se fera de même : on monte la constante `FORMAT_DE_COPIE`, et les anciennes copies s'effacent seules.
- **IndexedDB reste en v3.** Le magasin ne change pas de forme : un champ de plus dans une valeur ne demande pas de migration de base.
- **Le téléchargement** lit la lecture, puis le PDF en flux. La progression vient de la somme des `Content-Length`. Tout est écrit sous le verrou existant, et seulement une fois tout reçu : « entière ou absente », R12 de l'étape 1, inchangée.
- **Lire la copie** : `getDocument({ data })`, avec les octets tirés de `caches.match(pdf)`. Le tableau passe au travailleur. On ne garde pas de `blob:`, qui se lirait par XHR, sans plage.
- **La taille annoncée** (`reading_bytes`) devient, côté serveur, la taille du PDF plus celle du JSON de lecture. Elle est calculée à la fin de l'extraction.

**Écarté.**
- **Garder le PDF seul** et recalculer le texte sur le téléphone : la recherche, le sommaire et « Texte agrandi » reposent sur la forme lisible de PDFium, qui ne se refait pas avec pdf.js.
- **Migrer les copies de l'étape 1** : elles n'existent que sur des téléphones d'essai. L'arbitrage du 24/09 est de les effacer.

---

## R6 — Le lecteur des pages : gestes, repère de page, thème

**Décision.**
- **Un composant `GnLecteurPages`**, rendu côté client, enveloppe `PDFViewer` :
  - le conteneur est en `position: absolute`, comme `PDFViewer` l'exige ;
  - un `EventBus` et un `PDFLinkService` : un lien interne du PDF mène à sa page, un lien externe s'ouvre dans le navigateur ;
  - l'échelle de départ est la largeur de la page (`page-width`) ;
  - le grossissement va de la largeur à **quatre fois la largeur** (FR-003) ;
  - `maxCanvasPixels` garde le réglage mobile de pdf.js, sauf si l'essai le baisse.
- **Les gestes**, distingués par le composant :

| Geste | Effet |
|---|---|
| Pincement | `TouchManager` → `updateScale` autour du point |
| Double toucher (deux touchers à moins de 300 ms et à moins de 24 px l'un de l'autre) | Double l'échelle autour du point, jusqu'au plafond. Au-delà de la largeur, revient à la largeur |
| Toucher simple | Bascule la barre, **après** les 300 ms qui l'écartent d'un double toucher (FR-008) |
| Appui long | Laissé au navigateur, pour la sélection de la couche de texte |

- **La page en cours** vient de l'événement `pagechanging` de `PDFViewer`. Elle alimente le pied — numéro imprimé tiré de la lecture, repli sur la position — et la progression écrite toutes les deux secondes, comme à l'étape 1 (`useGnLecteur`). Le recalage après un changement d'orientation se fait par `currentScaleValue = 'page-width'`, en gardant la page.
- **Le thème** : le fond autour des pages, la barre et les feuilles prennent les rôles de Guide Négo. La page garde ses couleurs : aucun filtre, aucun mode de couleurs forcées de pdf.js (FR-011).
- **La grande largeur** : sur une tablette, la page tient la colonne de lecture, limitée par la mesure existante du lecteur.

**Écarté.**
- **Le zoom du navigateur** (celui du mode « tel quel » de l'étape 1) : il agrandit une image, sans redessin net, et emporte la barre avec lui.
- **Recolorer la page en thème sombre** : ce ne serait plus la page de l'IFDD, et les figures en perdraient le sens.

---

## R7 — Repérer un passage sur la page : par la couche de texte de pdf.js

**Constat.** Le worker tire des positions de PDFium (`Cadre` de chaque segment, `pdf.rs:184-266`), mais **ne les garde pas** : ni les blocs ni `document_pages` n'en portent.

**Décision.**
- **Le repérage se fait sur le téléphone, par la couche de texte** que pdf.js pose sur chaque page rendue. Rien ne s'ajoute au modèle.
- **Un utilitaire pur**, `utils/guide-nego/pdf/reperer.ts`, prend les chaînes de la couche (`textContentItemsStr`) et un texte cherché. Il replie les deux par `replier()` de l'étape 1, qui existe (sans accents, sans casse, blancs réduits), et rend les intervalles de caractères par élément. Le composant les change en rectangles par `Range.getClientRects()` sur les éléments de la couche, **en pourcentage de la page**, pour qu'ils suivent le grossissement sans recalcul.
- **Un passage de recherche** se repère par son **contexte** : l'extrait de l'index de l'étape 1, quelques mots avant et après.
  - À défaut, par l'expression seule **si elle est unique sur la page**.
  - Sinon, toutes les occurrences de la page sont marquées en clair, et le lecteur dit qu'il ne peut pas marquer celle-ci. Jamais un autre endroit n'est marqué plein (FR-015).
- **Le passage d'une note** : ses premiers mots, puis le passage entier. Introuvable, la note va en tête de page (FR-032).
- **Un renvoi « Tableau » de « Texte agrandi »** : le texte du bloc `origin`, s'il en a, repère l'endroit. Sinon, la page s'ouvre en haut.
- **La liste des passages** reste celle de l'étape 1 (`chercherDansLeDocument` sur la forme lisible), avec ou sans réseau. On ne demande pas le texte de chaque page à pdf.js pour chercher : ce serait 90 pages décodées à chaque recherche.

**Écarté.**
- **`PDFFindController`** : il cherche dans tout le document par pdf.js, qui ne connaît ni les sections ni l'index replié de l'étape 1. Il aurait donné deux listes de passages, qui ne concorderaient pas.
- **Garder les positions de PDFium en base** : cela coûterait une colonne de coordonnées par segment, un contrat de plus et une migration, pour une précision que la couche de texte donne déjà sur la page affichée. **À rouvrir** si l'essai montre que la couche de pdf.js ne retrouve pas les passages du guide.

---

## R8 — « Texte agrandi » : le lecteur de l'étape 1, extrait de la page

**Décision.**
- **Le rendu recomposé de l'étape 1** sort de `lire.vue` (603 lignes) dans un composant, `GnLecteurTexte`. Il porte les pages lues, les occurrences, les termes et les notes par bloc, **sans changement de comportement**. `lire.vue` ne garde que l'état, le choix du mode et les feuilles. Le garde-fou des mille lignes est tenu.
- **Le bloc `origin`** ne charge plus d'image. Il porte un renvoi, « Tableau — page 59 » ou « Figure — page 59 », qui passe en mode « Pages » à cet endroit (R7).
- **Le mode** est rangé en `localStorage` sous `gn.lecture-mode` (`pages` ou `texte`), avec les réglages de l'appareil (`appareil-lecture.ts`). Son défaut est `pages`. Un document qui n'offre pas « Texte agrandi » s'ouvre en pages, sans toucher au mode gardé (FR-021).
- **Changer de mode garde la page** : on passe l'index de page, et le mode d'arrivée s'y place — `sauter()` pour le texte, `currentPageNumber` pour les pages.

---

## R9 — Le choix du mode se voit : la barre, « Réglages », la ligne d'annonce, le pictogramme

**Décision.**
- **`GnBarreLecture`** gagne une action `mode`, placée **entre « Rechercher » et « Réglages »**, à l'emplacement de « Marquer » (écart 43). Elle rend `GnChoixMode` : deux segments, « Pages » et « Texte », de 48 px de haut, le mode en cours marqué. Elle n'apparaît que si « Texte agrandi » est offert.
- **`GnReglagesLecture`** porte le même `GnChoixMode`, puis le thème, puis la taille, **en mode texte seulement** (FR-024).
- **La ligne d'annonce** : `GnAnnonce` (0a), une fois par téléphone.
  - Condition : un document qui offre « Texte agrandi », et la largeur de la fenêtre sous 600 px en portrait.
  - Clé : `gn.lecture-mode-annonce`.
  - Elle se ferme au premier geste ou par sa croix. Elle ne revient plus.
- **Le pictogramme de « Réglages »** : `text-size` dessine deux A et ressemble au « Aa » du lexique, présent dans l'en-tête du lecteur (`GnEntete.vue:67-75`). Un pictogramme `sliders` — trois curseurs — entre dans `pictogrammes.svg` et dans `pictogrammes.ts`, au trait de la famille (24 px, trait de 2), et `GnBarreLecture` l'emploie (écart 44).
  - `text-size` reste pour le choix de taille, **à l'intérieur de la feuille**, où il désigne bien la taille du texte.
  - `lexique.vue:19` l'emploie aujourd'hui pour son titre. Ce sera **à vérifier** : si c'est le lexique, c'est son propre signe, et il reste.

---

## R10 — Les notes de correction : en marge de la page, dépliées sans la cacher

**Décision.**
- **`GnMargeNote`**, posé dans la vue de page de `PDFViewer` à l'événement `textlayerrendered` :
  - un filet rouge de 3 px au bord gauche de la page, à la hauteur du passage (R7), ou en tête ;
  - le triangle, et une cible de 48 px ;
  - positionné en pourcentage de la page, donc stable au grossissement.
- **Dépliée**, la note s'ouvre dans un **panneau non modal**, sans voile, en bas de l'écran, sur 40 % de la hauteur au plus. Il porte le contenu de `GnNoteCorrection` : texte, signature et bouclier.
  - Le lecteur fait défiler pour que le passage se tienne dans la moitié haute : la page reste visible (FR-033).
  - Un toucher sur la ligne, ou la croix, replie la note.
- **En « Texte agrandi »**, rien ne change : `ancrerLesNotes` et `GnNoteCorrection` de l'étape 1.
- **Les notes** viennent toujours de `notesDe(id)`, relues avec la liste et gardées avec elle : elles paraissent sur la copie sans la retélécharger (FR-034).

---

## R11 — « Texte agrandi » remplace « ouvrir tel quel » dans le modèle

**Décision.**
- **`serve_as_is` devient `large_text_choice boolean NULL`.** NULL veut dire « suit le verdict ». Vrai ou faux, c'est le choix de l'administratrice.
- **Deux règles de lecture**, écrites une seule fois, dans une fonction SQL du schéma : `negotiation.document_reading_modes(document_id)` rend `has_text` et `large_text`.
  - **Le texte est présent** si une page au moins a du texte (`plain_text <> ''`). Il commande la recherche et le sommaire (FR-018).
  - **« Texte agrandi » est offert** si le texte est présent et si `coalesce(large_text_choice, is_reflowable, false)` est vrai (FR-022). Un document sans texte ne peut pas l'offrir, même si l'administratrice l'a coché.
- **Relancer l'extraction garde le choix** (`repo/renditions.rs:52-84`). Un nouveau fichier fait une nouvelle version, donc une nouvelle ligne, qui suit le verdict.
- **Les images de page restent pour le back-office seul.** L'aperçu montre l'image rendue par PDFium à côté du texte, et l'expert y pose ses notes, comme aujourd'hui.
  - La route publique `GET …/pages/{i}/image` disparaît, et `pages[].image` quitte la lecture (FR-039 : « pour le téléphone »).
  - `image_key`, `image_bytes` et `has_origin_block` restent pour l'aperçu.

**Écarté.**
- **Rendre l'aperçu par pdf.js** : le back-office emploie les composants du site. Il faudrait y porter pdf.js pour un aperçu que les images de PDFium rendent déjà, en 1,5 s pour 90 pages.
- **Supprimer les images** : l'aperçu les perdrait sans gain pour le téléphone.
- **Un booléen non nul posé à la fin de l'extraction** : une relance écraserait le choix de l'administratrice.

---

## R12 — Le back-office : l'aperçu dit ce que sert le texte

**Décision.**
- **`PreviewVerdict.vue`** remplace l'interrupteur « Ouvrir tel quel » par « Proposer « Texte agrandi » ». C'est un `UiSwitch` qui montre le choix effectif, avec la mention « par défaut, selon le verdict : … » et « Revenir au verdict » quand un choix est posé.
- La route `PUT …/as-is` devient `PUT …/large-text`, avec `{ "choice": true | false | null }` ([contracts/api-lecture.md](contracts/api-lecture.md)).
- **L'en-tête de l'aperçu** dit que le lecteur montre la page d'origine, et que le texte sert à la recherche, au sommaire et à « Texte agrandi » (FR-036).
- **Un PDF illisible** — protégé, endommagé, sans page — finit déjà en `failed`, avec un motif lisible (`jobs/extract.rs:269-274`), et `publish` le refuse par `NEGOTIATION_DOCUMENT_NOT_READY` (FR-038).
  - Le message de l'aperçu gagne le motif. Pour un PDF protégé par mot de passe, le motif le nomme : le repli de `pdf.rs:59` distingue l'erreur de mot de passe de PDFium.
  - **Aucun code nouveau.**

---

## R13 — Le PDF lu en ligne ne reste pas

**Décision.**
- En ligne, pdf.js lit par plages. Les réponses ne passent **ni** par la coquille, qui n'intercepte pas l'API (ADR-019), **ni** par un cache Workbox, qui n'existe pas.
- Un réservé est servi en `no-store` : **le cache HTTP ne le garde pas** (FR-029).
- Les octets vivent dans le travailleur de pdf.js le temps de la lecture. La déconnexion ferme le lecteur et appelle `loadingTask.destroy()`, qui libère le travailleur, ce que fait déjà le démontage du composant.
- Un document lu en ligne n'est pas une copie : ni fiche, ni place, ni « Téléchargé ».

---

## R14 — Tests et preuves

| Preuve | Où |
|---|---|
| `206`, `416`, `200` sans plage, `ETag`, `Accept-Ranges`, pas de `Content-Encoding` | `negotiation/tests/documents_fichier.rs` sur base réelle |
| Réservé refusé **à chaque morceau** — sans session, session sans accès, adresse forgée, brouillon | Même fichier (SC-009) |
| La règle « Texte agrandi » : verdict, choix, pas de texte | `negotiation/tests/documents_admin.rs` et un test SQL de la fonction |
| `get_range` | `kernel` sur le stockage de fichiers de test |
| Repérage : contexte, expression unique, ambiguïté, césure, accents, élément coupé | `frontend/tests/guide-nego/reperer.test.ts` |
| Copie : format 2, effacement d'un format ancien, entière ou absente avec le PDF | `copies.test.ts` étendu |
| Mode, annonce unique | `appareil-lecture` dans `stockage.test.ts` ou un fichier neuf |
| Garde : les chemins `pdfjs/` sont dans la liste | `liste-de-garde.test.ts`, `sw-garde.test.ts` |
| Feuille bornée | `check:guide-nego` |
| Fluidité, netteté, mémoire, hors connexion | L'essai (R2), puis la recette sur la version construite ([quickstart.md](quickstart.md)) |

---

## Points ouverts

- **Le plancher des appareils (R1)** : un iPhone en iOS 16 ou 17 ne peut pas afficher les pages. Le plan lui ouvre « Texte agrandi » quand le document l'offre, et sinon lui dit que ce téléphone n'affiche pas ce document. **À confirmer par le commanditaire**, avec la part de ces téléphones dans les délégations si elle est connue.
- **WebKit hors connexion (R2, issue B)** : se tranche par l'essai.
