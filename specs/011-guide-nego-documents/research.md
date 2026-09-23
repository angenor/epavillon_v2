# Recherche — Guide Négo, la bibliothèque de documents et le lecteur (étape 1)

**Date** : 2026-09-22 · **Spécification** : [spec.md](spec.md)

Ce fichier tranche ce que la spécification a laissé ouvert, et ce que la relecture du commanditaire du 22/09 a confié au plan. Chaque décision porte son *pourquoi*, et ce qui a été écarté.

**L'essai du 23/09 a rendu l'issue A** ([essai-extraction.md](essai-extraction.md)) : R1 est confirmé, R7 ajusté, et **R2 maintenu par le commanditaire** bien que la copie pèse plus que le PDF ([ADR-021](../../docs/AppNego/adr/021-pdfium-dans-le-worker.md)).

---

## R1 — L'outil d'extraction : PDFium dans le worker Rust

**Décision, confirmée par l'essai (issue A)** : **PDFium**, par la caisse `pdfium-render` 0.9.4 et le binaire chromium/7881, appelé par un travail différé du worker Rust. Rien de Python à cette étape. Sur le vrai guide, les huit critères tiennent, et l'extraction complète prend 1,8 s.

**Pourquoi.**
- **Il rend ce que l'essai doit vérifier.** Par caractère ou par segment : le texte, la **police** (nom, graisse, drapeau italique), et le **cadre** (position, donc les colonnes). Par document : le **sommaire** du PDF (signets) et les **étiquettes de page** (« 59 » imprimé contre l'indice 63 du fichier). Par page : un **rendu en image**, qui sert l'aperçu page par page (FR-005) et le mode « ouvrir tel quel » (FR-005 bis) sans bibliothèque de plus.
- **Vu à l'essai** : la graisse qu'il rend est inutilisable (220, 235…), le gras se lit au nom de la police ; le trait d'union de fin de ligne arrive en U+0002, jamais perdu ; les filets et les pointillés des encadrés arrivent comme objets graphiques, ce qui suffit à repérer tableaux et figures. Le guide n'a ni signets ni étiquettes : R7 s'en passe.
- **Licences compatibles.** `pdfium-render` est MIT ou Apache-2.0. PDFium est BSD-3 ou Apache-2.0. Les binaires précompilés (`bblanchon/pdfium-binaries`) héritent de ces licences.
- **Aucune porte nouvelle.** Le worker existe, sa file de travaux aussi (`kernel/src/jobs.rs`, patron `negotiation/src/jobs/emails.rs`). L'extraction relève du domaine des négociations : la forme lisible, ses pages et les notes posées dessus sont des notions de `negotiation`, pas de l'assistant.

**Ce que cela coûte.**
- Une bibliothèque native à fournir au worker, par `PDFIUM_LIB_PATH` :
  - en développement, une cible `make pdfium` qui télécharge le binaire du poste (macOS arm64, Linux x64) ;
  - en production, une ligne dans l'image du worker.
- L'API n'en a pas besoin : elle ne fait que servir.

**Écarté.**
- **PyMuPDF** : sous **AGPL**. Un service réseau qui l'emploie doit publier son code source, et la licence commerciale d'Artifex est payante. Il n'est pas retenu, quelle que soit la qualité de son extraction.
- **`pdf-extract`, `lopdf`** (MIT) : ils rendent le texte, mais ni un italique fiable ni un cadre exploitable, et aucun rendu d'image. L'essai ne pourrait rien vérifier.
- **Bindings `mupdf`** : AGPL, même motif.
- **pdf.js dans le navigateur de l'administratrice** : le texte serait produit par un client, que rien ne vérifie côté serveur. L'étape 7 devrait extraire une seconde fois. Et l'essai se mènerait dans un navigateur, pas dans un test.

### Si l'essai conclut à une bibliothèque Python

L'alternative est **pdfplumber** (MIT, sur pdfminer.six, MIT), qui rend la police et la position de chaque caractère, avec **pypdfium2** (Apache-2.0 ou BSD-3) pour les images. Elle impose de monter **maintenant** le service interne d'ADR-004. Ce qu'il coûte, **trois à quatre jours**, avant la moindre ligne d'écran :

| Pièce | Travail |
|---|---|
| Le service | FastAPI minimal, sans route publique : un conteneur, son Dockerfile, sa santé, son réseau interne en développement et en production (`ops/docker-compose.*`, `deploy.sh`, § 14-15 de DEPLOIEMENT.md) |
| L'entrée | ADR-004 : il reçoit ses travaux **par la file d'événements**. Il faut le relais qui les lui porte, et sa reprise après panne |
| La sortie | ADR-004 : il **n'écrit que dans `tool`**, or la forme lisible appartient à `negotiation`. Il déposerait son résultat en objet de stockage et émettrait « extrait », et un travail Rust l'importerait. Cela fait deux travaux au lieu d'un, et un format d'échange à tenir |
| Les secrets | Une clé S3 en lecture seule, distincte de celle de l'API |
| Les tests | Un second langage dans `make check-safe` |

Ce coût serait avancé, pas perdu : l'étape 7 monte ce service de toute façon. Mais il ferait de l'étape 1 une étape d'infrastructure. **On ne le paie que si PDFium échoue là où pdfplumber réussit**, et l'essai le dit.

### Les trois issues de l'essai

| Issue | Suite |
|---|---|
| **A** — PDFium tient les cinq critères, ou les tient après un traitement écrit en Rust (R7) | Le plan tel qu'écrit |
| **B** — PDFium échoue sur un critère où pdfplumber réussit | Le service Python d'ADR-004 monte en phase 2, avec son coût ci-dessus ; le commanditaire est prévenu avant |
| **C** — Aucun outil ne recompose le guide proprement (colonnes imbriquées, tableaux partout) | Le guide se publie **« tel quel »** (FR-005 bis). La recomposition reste pour les documents simples, et le commanditaire est prévenu du jour même : la taille du texte et les termes touchables ne vaudront pas pour le guide |

---

## R2 — Ce que garde le téléphone : la forme lisible, plus les seules pages d'origine utiles

> **Mesuré le 23/09, tranché le jour même par le commanditaire : on garde tout** ([ADR-021](../../docs/AppNego/adr/021-pdfium-dans-le-worker.md)). Le vrai guide pèse **2,9 Mo**, pas 9. Sa forme lisible pèse **74 Ko** compressée. Mais **19 pages sur 90** portent un tableau ou une figure, et leurs images pèsent **4,1 Mo** (217 Ko chacune en moyenne, 1080 px, JPEG 75) : la copie décrite ci-dessous pèse donc **4,2 Mo, plus que le PDF**. Le seuil du tiers n'est pas atteint (21 %). Les trois voies sont dans la conclusion d'[essai-extraction.md](essai-extraction.md). **Retenue** : la copie telle qu'écrite, pour que le guide se lise **en entier** sans réseau (SC-002), avec un encodage des images plus léger, à régler en phase 4 (environ 3 Mo).

**Décision pressentie** :
- **Document recomposé** : on garde **la forme lisible**, c'est-à-dire le texte recomposé en JSON compressé. On y ajoute **les images des seules pages qui portent un bloc non recomposable** (tableau, figure) : le lecteur y renvoie par « Voir la page d'origine ».
- **Document « tel quel »** : on garde **les images de toutes ses pages**.
- **Le PDF lui-même ne va jamais sur le téléphone.**

**Pourquoi.**
- Sur un réseau de COP, le poids compte : 74 Ko de texte, contre 2,9 Mo de PDF — mais les pages d'origine en ajoutent 4,1.
- Le PDF n'apporterait rien au lecteur, qui ne l'affiche pas.
- Les images de page, que l'extraction produit déjà pour l'aperçu, rendent ce que le texte perd, **là seulement où il le perd**.

**À mesurer par l'essai** :
- le poids de la forme lisible du guide ;
- le nombre de pages à bloc non recomposable ;
- le poids d'une image de page à 1080 px de large en JPEG qualité 75.

Si ces pages sont trop nombreuses, par exemple plus d'un tiers, on garde tout le document en images.

**Écarté.** **Garder le PDF et le texte** : la copie serait plus lourde que le PDF seul, pour le seul cas des tableaux. **Le texte seul, sans images** : un tableau éclaté en paragraphes ne se lit pas, et ne pas le dire tromperait (XII).

---

## R3 — Où vit la forme lisible : en base, page par page

**Décision** : une table `negotiation.document_pages`, une ligne par page. Elle porte :
- l'étiquette imprimée de la page ;
- ses blocs, en JSON de grammaire close ([forme-lisible.md](contracts/forme-lisible.md)) ;
- son texte brut et son vecteur de recherche ;
- la clé de son image dans le stockage privé.

Le sommaire et le verdict de l'extraction vivent dans `negotiation.document_renditions`, une ligne par document fichier.

**Pourquoi.**
- **FR-014** : la recherche de la bibliothèque porte sur *le texte* des documents. Un vecteur par page permet de dire en ligne « trouvé page 59 », avec un index GIN, sans dépôt d'objets à parcourir.
- **FR-046** : une note se pose sur une page. Une ligne par page donne une clé à laquelle se rattacher, et une borne que la base vérifie.
- L'API sert la forme lisible **en un seul corps**, recomposé des pages, sous une empreinte (R9). Pour 92 pages de texte, c'est une requête.

**Écarté.** **La forme lisible en objet de stockage** : la recherche entre documents devient impossible, et une note n'a plus d'ancre que la base connaisse. **Écrire les images en `media.renditions`** : ce serait l'écriture d'un module dans le schéma d'un autre. Les images de page appartiennent à `negotiation`, qui tient leurs clés.

---

## R4 — Les fichiers privés sont aujourd'hui publics : un bucket privé, et un contrat de stockage partagé

**Constat.**
- Le bucket unique `epavillon` est **ouvert en lecture web** (`Makefile:250`, `ops/init-garage-prod.sh:71`), et `ops/media-proxy.conf` le sert sans droits.
- La visibilité `private` existe dans `media.asset_visibility`, mais tout est déposé dans ce même bucket (`media/src/repo/assets.rs:264-273`).
- **Aucune adresse signée n'existe** (`media/src/domain/asset.rs:46-48`).

Un PDF réservé, déposé aujourd'hui, serait donc lisible par quiconque obtient son adresse.

**Décision.**
1. **Un second bucket, `epavillon-prive`**, créé par `make garage-init` et `init-garage-prod.sh`, **sans** `bucket website --allow`. `media` y dépose tout objet de visibilité `private` (réglage `media.private_bucket`). La déduplication reste par bucket, comme aujourd'hui.
2. **Tout PDF de document se dépose `private`**, qu'il soit public ou réservé. Faire passer un document de public à réservé ne déplace alors aucun objet : **l'API sert seule le fichier**, après avoir vérifié l'accès. Les images de page suivent la même règle.
3. **Le contrat de stockage passe de `media` à `kernel`** (`kernel::storage` : le trait, S3, système de fichiers), exactement comme `kernel::mail`, que le module de stockage cite lui-même pour patron. `media` garde toute sa logique : dépôt, analyse, déclinaisons. `negotiation` peut alors lire un objet pour l'extraire ou le servir, sans arête vers `media`.
4. **Une fonction SQL `media.object_location(asset_id)`** rend le bucket, la clé, l'état, la taille et le type d'un objet. C'est un contrat de lecture posé par le schéma propriétaire, comme `media.object_url()` l'est déjà.

**Pourquoi.** C'est la plus petite correction qui rende vrai « réservé » (FR-022, SC-007). Elle ne crée aucune dépendance entre crates de module (principe II) et n'invente pas d'adresse signée, qu'il faudrait expirer, révoquer et tester.

**Écarté.**
- **Une route de `media` qui servirait les objets privés** : elle ne sait pas si un document est réservé ni si la personne a l'accès, sauf à lire le schéma `negotiation`.
- **Des adresses signées à durée limitée** : une adresse copiée reste valable jusqu'à son terme, et le téléphone doit de toute façon passer par l'API pour connaître l'accès.
- **Un préfixe refusé par le proxy nginx** : la sécurité dépendrait d'une règle de configuration que rien ne teste.

**Dette relevée.** Tout objet déjà déposé en `private` est dans le bucket public. À vérifier en base avant la mise en ligne : si la requête rend zéro ligne, il n'y a rien à déplacer.

---

## R5 — Déposer le PDF : la garde média, et un champ de fichier tiré de celui des images

**Décision.**
- **Côté API** :
  - la garde de `media` (`media/src/domain/guards.rs`) reçoit le couple `("negotiation","documents")`, gardé par **`negotiation.document.publish`**, qui existe déjà (`100_negotiations.sql:1162-1192`), et restreint au type `application/pdf` ;
  - le test qui en vérifiait le refus (`guards.rs:160-161`) s'inverse ;
  - la visibilité imposée est `private`.
- **Le brouillon d'abord.**
  - L'invariant « fichier **ou** lien » se dédouble : **au plus un** des deux pour un brouillon, **exactement un** pour un document publié.
  - Un brouillon naît donc sans source, et le PDF se dépose avec ce brouillon pour propriétaire.
  - La règle « jamais les deux » reste tenue par la base à tout instant.
- **Côté back-office** : réutiliser avant d'écrire (CLAUDE.md).
  - `components/media/ImageField.vue` est le seul champ qui dépose un fichier, et il impose le recadrage et le texte alternatif.
  - Sa partie « dépôt » — contrôle préalable, envoi, progression, erreurs — sort dans un composable partagé (`composables/useDepotMedia.ts`).
  - `ImageField` le garde tel qu'il est, et un `media/FileField.vue` nouveau l'emploie pour les fichiers qui ne sont pas des images.
  - `UploadPayload.altText` devient facultatif quand le type n'est pas une image. Le serveur, lui, ne l'exige que pour les images : à vérifier au premier dépôt, et à corriger s'il l'exige pour tout type.

**Écarté.** **Recopier le dépôt dans une page** : c'est exactement le défaut que CLAUDE.md cite, deux comportements pour un même geste. **Créer le document au moment du dépôt** : le formulaire perdrait le brouillon, et un dépôt échoué laisserait une ligne orpheline.

---

## R6 — L'extraction : un travail différé, qui attend l'analyse du fichier

**Décision.**
- Attacher un PDF à un document met en file, **dans la même transaction**, la tâche `negotiation.document.extract`. Sa clé d'idempotence est l'identifiant du document suivi de celui de l'objet.
- Le travail lit `media.object_location()` :
  - tant que l'objet n'est pas `ready` — analyse en cours —, il se replanifie avec un délai croissant ;
  - `quarantined` ou `failed` le font conclure « échec », en disant pourquoi ;
  - `ready` lance l'extraction.
- L'état se tient dans `negotiation.document_renditions.status`, une machine à états (ENUM `negotiation.rendition_status` : `pending`, `extracting`, `ready`, `failed`), vue par l'aperçu du back-office.
- **Budget** : moins d'une minute pour cent pages, images comprises. L'essai le mesure.

**Publier exige** une extraction `ready`, ou le choix « ouvrir tel quel ».
- Si l'extraction a échoué **sur le texte**, les images de page existent le plus souvent : le mode « tel quel » est alors proposé d'office.
- Si les images elles-mêmes ont échoué, le fichier est illisible : la publication est refusée, avec le motif.

**Écarté.** **Extraire dans la requête de dépôt** : quatre-vingt-douze pages et leurs images dépassent le délai d'une requête, et l'analyse antivirale doit passer d'abord.

---

## R7 — La forme lisible : ce que fait le traitement après PDFium

**Décision** : un traitement écrit en Rust, pur et testé sur des pages d'essai, ordonne ce que PDFium rend. **Ses règles ont été ajustées par l'essai du 23/09** sur le vrai guide ([essai-extraction.md](essai-extraction.md)), qui les a toutes éprouvées :

| Critère | Règle ajustée |
|---|---|
| **Ordre de lecture** | L'ordre du flux que rend PDFium. Une ligne écrite **après** une ligne qu'elle surplombe, dans les mêmes abscisses, est flottante : les flottantes voisines forment une zone, replacée à sa hauteur. Les marges ne comptent pas, car un pied non écarté ouvre souvent le flux. **Le regroupement en colonnes par abscisse n'a pas servi** : il n'est pas écrit |
| **En-têtes et pieds** | Une ligne dans les 12 % du haut ou du bas, au même texte à un nombre près — une suite de chiffres vaut un joker —, à 4 pt près en hauteur, sur **trois pages et le cinquième du document** au moins (et non la moitié). Le numéro écarté devient l'étiquette de la page |
| **Notes** | Sous le **filet court et isolé** qui part de la marge gauche, dans le bas de page — le bord d'un aplat n'en est pas un (p. 51, 59). La marge gauche est l'abscisse où commencent le plus de lignes, pas la plus petite : une puce peut déborder (p. 77). Sans filet, les lignes en petit corps qui ferment la page. Une note commence par sa marque. L'appel devient un chiffre en exposant (« ¹ ») |
| **Tableaux et figures** | **Tableau** : deux filets fins horizontaux et deux verticaux au moins, qui se touchent. **Figure** : une image de plus de 40 pt, ou une zone flottante qui porte un aplat autre que son simple cadre. **Encadré** : une zone flottante sans dessin, recomposée à sa place. Les pointillés — des centaines d'images d'un point (1 088 sur la p. 59) — sont ignorés |
| **Césures** | Le trait d'union de fin de ligne arrive en U+0002. Il se **garde**, sauf si le mot recollé est attesté en milieu de ligne ailleurs dans le document, et pas le mot à tiret. La condition « la ligne suivante commence par une minuscule » tombe. Une adresse coupée se recolle sans blanc |
| **Italiques** | Au-delà de 20 % de faux, seul l'italique **maigre** reconnu anglais : un mot vide anglais, ou deux mots capitalisés au moins, sans accent ni mot vide français ; ni chiffre ni guillemet ; douze mots au plus. Le gras italique est un titre, jamais un terme |
| **Titres** | Un corps d'au moins 1,25 fois le texte courant ; ou une ligne toute en gras, plus grande ou numérotée ; ou une ligne courte toute en gras qui ne finit pas une phrase. Niveau 1 dès 1,4 fois le texte ; sinon la profondeur du numéro (« 3.6.1. » → 3, « A.2. » → 2, « II. » → 2) ; 3 sans numéro. Les lignes consécutives de même corps se fondent en un titre. **Une page à points de conduite est un sommaire imprimé** : ni titres ni termes |
| **Sommaire** | Les signets s'il y en a. Sinon les titres de niveau 1 et 2, et ceux de niveau 3 qui sont numérotés |
| **Pages** | L'étiquette du PDF s'il la porte ; sinon le numéro imprimé, lu dans le pied écarté ; sinon l'indice |
| **Paragraphes** *(nouveau)* | Un paragraphe se ferme sur un écart de plus de 1,25 interligne ; sur une ligne qui s'arrête avant la marge droite en finissant une phrase (3 pt quand la page est justifiée, 1,5 corps sinon) ; sur une puce ; sur le retour à la marge d'un retrait suspendu, comme en bibliographie |

Le résultat porte un **verdict** : recomposable ou non, avec ses indicateurs (part des pages avec du texte, blocs `origin` par raison, notes, titres repérés). L'aperçu les affiche.

**Un bloc `origin` porte le texte de sa zone** (`text`), ajouté au contrat par l'essai : 10 % du texte du guide, dont tout le tableau des sigles, échapperait sinon à la recherche ([forme-lisible.md](contracts/forme-lisible.md)).

La grammaire est close, comme celle de `GnTexteLong` : aucun HTML ne traverse ([forme-lisible.md](contracts/forme-lisible.md)).

---

## R8 — La recherche : la liste se filtre sur le téléphone, le texte se cherche en ligne

**Décision.**
- **La liste entière** arrive au téléphone. Avec quelques dizaines de documents, elle pèse quelques dizaines de kilo-octets. Les trois filtres, leurs compteurs, « Afficher n documents » et la recherche sur le titre, le résumé et l'éditeur tournent **sur le téléphone**, donc sans réseau.
- **« Un mot du texte »** (FR-014) : `GET /negotiation/documents?q=` cherche dans les vecteurs des pages et rend, pour chaque document, ses pages trouvées. Hors connexion, la bibliothèque le dit : « La recherche dans le texte demande le réseau ; les titres et résumés restent cherchables ». Les documents téléchargés restent cherchables dans le lecteur.
- **Dans le document** (FR-041) : sur le téléphone, dans la forme lisible, **sans tenir compte des accents ni de la casse** — « progres collectifs » trouve « progrès collectifs ». Le compteur « 5 passages dans 92 pages » et le « vous êtes ici » en découlent.

**Pourquoi.** SC-004 et SC-009 se tiennent sans réseau. Au volume d'une bibliothèque de COP, la liste filtrée côté serveur n'apporterait rien, sinon de perdre le hors-connexion.

**Écarté.** **Un index de recherche plein texte embarqué** (lunr, minisearch) : une dépendance de plus, pour chercher dans des documents que le téléphone n'a pas.

---

## R9 — L'empreinte, sans « modifié depuis »

**Décision.** Trois lectures portent un `ETag` de contenu (`kernel/src/empreinte.rs`) et répondent `304` sur `If-None-Match` (`routes/mod.rs:42-47`) :

| Lecture | Empreinte | Remarque |
|---|---|---|
| La liste | Sur le JSON rendu, qui **dépend de la personne** : champs réservés visibles ou non | `Cache-Control: private, no-cache` |
| Les notes de correction de tous les documents | Sur leur liste | Une seule lecture tient à jour toutes les copies gardées (FR-050) |
| La forme lisible d'un document | Figée tant que le fichier ne change pas (FR-007) | Les notes n'y sont pas, pour ne jamais la faire retélécharger |

Côté client, `lireEtiquete` (`composables/api/http.ts`) apprend à **envoyer** `If-None-Match` et à rendre « inchangé » sur `304`. C'est une extension, pas une seconde primitive.

**Pourquoi pas « modifié depuis »**, que [03-api.md](../../docs/AppNego/03-api.md) prévoit et que le plan de 0c renvoyait à cette étape (R4 de 0c) :
- un delta exige des **pierres tombales** — document dépublié, devenu réservé, accès perdu — dont l'absence rend une copie fausse sans le dire ;
- au volume attendu, cent documents, la liste compressée pèse environ quinze kilo-octets ;
- le `304` coûte une requête et zéro octet quand rien n'a changé, ce qui est le cas le plus fréquent en salle.

**Seuil de révision** : cinq cents documents, ou une liste compressée au-delà de cent kilo-octets.

---

## R10 — Réservé : la règle, et où elle est tenue

**Décision.**
- L'accès est celui de 0b : `identity.has_permission(personne, 'negotiation.space.access', global)`.
- **La liste** rend un document réservé à toute personne, mais sans son résumé ni ses thématiques quand l'accès manque (FR-022). La fiche hors connexion en découle : le téléphone ne reçoit jamais ce qu'il ne doit pas montrer.
- **La forme lisible, les images de page, le PDF et la recherche dans le texte** refusent en `403` avec un code nouveau, `NEGOTIATION_DOCUMENT_RESTRICTED`, dont le message dit ce qu'il faut pour ouvrir.
- La recherche plein texte n'exclut pas les réservés, mais ne rend pour eux **ni passage ni page** : dire « il existe un document réservé sur ce sujet » est permis, en citer le texte ne l'est pas.

**Tests** : chaque route refusée est essayée sans compte, avec un compte sans accès, et par identifiant forgé (SC-007).

---

## R11 — Le téléphone : deux caches, un magasin, et ce qui les vide

**Décision.**
- **Deux caches** : `gn-documents-publics` et `gn-documents-reserves`. Ils portent la forme lisible et les images, sous l'adresse d'API qui les sert. Ils n'ont pas le préfixe `gn-coquille-` : le ménage du service worker ne les touche pas et ils survivent aux déploiements (ADR-019). Le test `sw-garde` l'affirme déjà pour tout autre préfixe, et gagne un cas nommé.
- **Un magasin IndexedDB `copies`**, qui porte la base `guide-nego` à sa version 3. On y range, par document : la version, l'empreinte, la date, la place, le public ou le réservé, et les pages d'origine gardées. C'est lui que lit « Sur le téléphone ».
- **Le service worker n'intercepte rien de plus.** Le lecteur lit ses caches lui-même (`caches.match`). ADR-019 reste vrai : aucune réponse d'API ne passe par le cache de la coquille.
- **Ce qui vide** :

| Événement | Effet |
|---|---|
| « Tout retirer » | Les deux caches et le magasin `copies`, **rien d'autre** (FR-032). La place baisse aussitôt : le cache se libère à la suppression, ce qu'IndexedDB ne fait qu'au compactage, comme mesuré le 22/09 |
| Déconnexion | Le cache `gn-documents-reserves` et ses entrées dans `copies`, **avant** l'appel qui ferme la session (`useGnSession.ts`, à côté de la vidange de la file) |
| Accès perdu, lu par `useGnAcces` | Même effacement que la déconnexion |
| Relecture de la liste | Une copie dont le document a disparu, ou est devenu réservé pour une personne sans accès, s'efface (FR-034) |
| Panne de réseau, API muette | **Rien** : une API muette n'est pas une déconnexion, comme l'a posé la reprise 1 de 0c |

- **Les réglages de l'appareil**, en `localStorage` protégé par `try/catch` : la taille du texte ; la progression par document et par version ; les documents déjà ouverts, qui servent à « Nouveau » ; les derniers ouverts, qui servent à « Ma journée ». Tout cela pèse quelques kilo-octets. Ce n'est pas une lecture, et « Tout retirer » ne le vide pas.

**Écarté.**
- **Un seul cache, dont on trierait les réservés** : l'effacement à la déconnexion deviendrait un parcours au lieu d'une suppression.
- **Ranger les documents dans IndexedDB** : la place ne baisse qu'au compactage, comme mesuré en 0c.

---

## R12 — Télécharger : complet ou rien, et la demande faite sans réseau

**Décision.**
- **Le téléchargement lit la forme lisible en flux** : la progression vient de `Content-Length`, « 3,3 Mo sur 9,4 Mo ». Il lit ensuite les images utiles.
- **Il n'écrit dans le cache qu'une fois tout reçu.** `AbortController` sert « Annuler », et une coupure ne laisse rien de lisible (FR-026).
- La taille annoncée sur la fiche est celle **de la copie**, pas du PDF : elle est calculée à l'extraction et servie avec la liste.
- **« Télécharger au retour du réseau »** : un magasin `a-telecharger`, qui partage les déclencheurs de la file de 0c (ouverture, `online`, retour au premier plan) sans être cette file. Ce sont des lectures, pas des écritures, et elles ne portent aucun compte. Une entrée ne part qu'une fois, et se retire au succès.
- **Le compteur** : `POST /negotiation/documents/{id}/downloads`, sans compte, envoyé une fois par copie réussie. Il n'enregistre rien de la personne (FR-035) : c'est `negotiation.register_document_download()`, qui existe.

---

## R13 — Les favoris : la table existe, la file de 0c les porte

**Décision.**
- `GET /negotiation/me/bookmarks` porte une empreinte. `PUT` et `DELETE /negotiation/me/bookmarks/{document_id}` sont **idempotents par forme** : poser deux fois ne crée rien, retirer deux fois non plus. La table `negotiation.document_bookmarks` existe, et sa clé est le couple personne-document.
- Hors connexion, l'intention part par **la file de 0c** (`utils/guide-nego/file.ts`), sous la clé `favori-<id>`. Son corps est l'état voulu, et elle ne porte **pas** d'`If-Match` : pour un favori, la dernière intention est la bonne, et il n'y a pas de choix plus récent à protéger.
- La file porte déjà le compte de la personne, et se vide à la déconnexion.

---

## R14 — Le lecteur : texte recomposé, pages repérées, mode « tel quel »

**Décision.**
- **Le texte** se recompose à la largeur de l'écran, en 17, 20 ou 24 px avec un interligne de 1,5. Les titres ne bougent pas (`01-systeme.html:486`).
- **Le repérage.** Chaque page s'ouvre par un repère invisible. Un observateur d'intersection tient « Page 59 sur 92 · 3.6 Adaptation » et la jauge. Changer de taille recale sur la page en cours (FR-042).
- **La progression** s'écrit au plus toutes les deux secondes, par document et par version. Une autre version s'ouvre au début (FR-039).
- **Le mode « tel quel »** montre les images de page empilées. Le zoom est celui du navigateur. Il n'y a ni taille du texte, ni termes, ni recherche, et la fiche le dit.
- **La barre** : un toucher au centre la bascule, défiler la replie (`composants.md:157-159`). Pas de barre d'onglets dans le lecteur.
- **Le terme touché** ouvre `GnFeuilleBasse`, titrée du terme et vide (FR-043). L'étape 2 la remplira.
- **La note de correction** s'ancre par sa page et, s'il est donné, par un **extrait cité** du passage. Le lecteur cherche l'extrait dans les blocs de la page. Introuvable, la note se place en tête de page, sans erreur.

**Composants nouveaux**, dans le dossier de Guide Négo et sur la planche :

| Composant | Rôle |
|---|---|
| `GnLigneDocument` | Une ligne de la liste |
| `GnBandeauRemplace` | « Remplacé par… » |
| `GnBarreLecture` | Repliée et dépliée |
| `GnLigneSommaire` | Une entrée du sommaire |
| `GnNoteCorrection` | Repliée et dépliée |
| `GnOccurrence` | Barre « Occurrence 3 sur 5 » |
| `GnProgression` | Barre de téléchargement de 6 px, distincte de `GnJauge`, qui dit une place |
| `GnFeuilleFiltre` | La feuille à cases, avec « Afficher n documents » |

---

## R15 — Les notes de correction et le rôle `expert`

**Décision.**
- **Le modèle** : une table `negotiation.correction_notes`. Elle porte la page, un extrait cité facultatif, le texte en `i18n_text` (français exigé), l'auteur et la date. Un retrait se marque par la date et l'auteur du retrait, et **jamais par une suppression** (FR-049). Un déclencheur d'audit y est posé.
- **La borne** : un déclencheur vérifie que la page existe dans la forme lisible du document (principe VIII).
- **Le rôle `expert`** :
  - semé dans `100_negotiations.sql`, à côté de `space_lead`, avec la portée **globale** seulement ;
  - deux permissions, une par geste : `negotiation.correction.post` et `negotiation.correction.withdraw` ;
  - il paraît de lui-même dans l'écran des utilisateurs (A12), qui lit `identity.roles` (`repo/rbac.rs:210-240`) ;
  - `super_admin` reçoit les permissions par le déclencheur existant ;
  - le commentaire du rôle nomme les étapes qui y ajouteront les leurs : **2** (répondre aux questions posées à l'expert), **7** (état des sources de l'assistant), **8** (notes sur un intervalle de vidéo).
- **L'accès au back-office.** L'expert doit voir la liste et l'aperçu d'un document pour y poser une note :
  - ces deux lectures s'ouvrent à qui détient `negotiation.document.publish` **ou** `negotiation.correction.post`, testés dans le gestionnaire par `has_permission`, en portée globale ;
  - les écritures du document restent à `publish` seul ;
  - le menu du back-office montre l'entrée à qui a l'une ou l'autre.
- **Aucun événement émis** (principe IV sans objet) : l'assistant lira la table à l'étape 7. Émettre un événement que personne ne consomme serait du bruit.

---

## R16 — Les thématiques d'un document, par `reference.entity_terms`

**Décision (arbitrée le 22/09).**
- Un dépôt `negotiation/src/repo/document_themes.rs` suit le patron de `programme/src/repo/themes.rs:100-150`. Le couple `('negotiation','documents')` y est **écrit en littéral**, jamais reçu. L'écriture efface les liens de la taxonomie `negotiation_theme`, puis insère depuis `unnest(...) JOIN reference.taxonomy_terms` filtré sur la taxonomie **et `is_active`**, puis compare le nombre posé au nombre reçu. Un écart nomme le code refusé : `NEGOTIATION_DOCUMENT_UNKNOWN_THEME`.
- **La suppression d'un brouillon efface ses liens dans la même transaction**, puisque `platform.purge_term_links()` n'existe pas (écart n° 94). Un document publié ne se supprime pas : il se dépublie (FR-009).
- **La lecture** écarte les termes désactivés. On ne s'appuie pas sur `reference.terms_of()`, qui ne les filtre pas (dette relevée en 0c).
- **`track_term_id`** (climat, biodiversité, désertification) reste en place et **n'est pas exposé** à cette étape. Il n'est pas une thématique, et aucun écran ne le demande.

---

## R17 — La COP, la date, l'éditeur, les types

**Décision.**
- **La COP** : une colonne `event_id`, facultative, liée à `event.events` par `xmod_fk_documents_event`, sur le patron de `negotiation.meetings.event_id`. Le formulaire propose les éditions existantes, par la lecture publique des éditions déjà servie. Le filtre « COP » ne liste que les éditions citées par un document publié, sous le libellé `edition_label`.
- **La date du document** : une colonne `issued_on date`, distincte de `published_at`. La fiche montre la première, « Nouveau » se calcule sur la seconde.
- **L'éditeur** : `external_publisher` se renomme `publisher`, et vaut pour toute source. Aucune ligne de code ne le cite aujourd'hui, et le vecteur de recherche suit le renommage.
- **Les types** : `summary` (Résumé) et `bulletin` (Bulletin) rejoignent `document_type` dans `020_reference.sql`. `negotiation_guide` s'affiche « Guide ».

---

## R18 — Le remplacement : un seul successeur, pas de boucle, le bout de la chaîne

**Décision.**
- `supersedes_id` garde son sens : la nouvelle version pointe vers l'ancienne.
- Un index unique partiel sur `supersedes_id` interdit deux successeurs. Il est traduit en `NEGOTIATION_DOCUMENT_ALREADY_SUPERSEDED`, avec le nom du successeur (FR-006).
- Un déclencheur remonte la chaîne et refuse une boucle : `NEGOTIATION_DOCUMENT_SUPERSEDE_CYCLE`.
- **« Remplacé »** ne se dit qu'une fois le successeur **publié**. Un brouillon ne remplace encore rien.
- La liste porte, pour chaque document remplacé, **le bout publié de sa chaîne**, lu par une requête récursive : le bandeau y mène directement.
- **Changer le fichier d'un document publié** est refusé (`NEGOTIATION_DOCUMENT_FILE_LOCKED`). Le back-office propose « Publier une nouvelle version », qui crée un brouillon prérempli, désigné comme remplaçant.

---

## R19 — Le back-office : quatre écrans, les composants du site

**Décision** : sous `pages/admin/negociations/documents/`, avec les composants `ui/` du site et le menu existant.

| Écran | Contenu |
|---|---|
| `index.vue` | La liste : état (brouillon, publié, dépublié), type, version, remplacement, extraction |
| `nouveau.vue` | Le formulaire, qui crée un brouillon |
| `[id].vue` | Le même formulaire en modification, la source (fichier par `FileField` ou lien), l'état de l'extraction, publier et dépublier, « Publier une nouvelle version » |
| `[id]/apercu.vue` | **Page par page** : l'image à gauche, la forme lisible à droite, le verdict et les indicateurs en tête, « ouvrir tel quel », et la pose d'une note : on choisit la page, on sélectionne un passage, on écrit la note ; la liste des notes et leur retrait sont en marge |

L'aperçu se parcourt au clavier (← →) et au toucher. Sur tablette, les deux colonnes s'empilent. Cent pages se feuillettent en moins de dix minutes (SC-001 bis).

---

## R20 — Où vont les méthodes d'API, et le garde-fou des mille lignes

**Décision.**
- Les méthodes publiques vont dans un fichier nouveau, `composables/api/guide-nego-documents.ts`, et celles du back-office dans `composables/api/admin-negotiation-documents.ts`. `useApi.ts`, à 896 lignes, ne gagne que les lignes de branchement, une dizaine.
- Les types vont dans `types/negotiation-documents.ts` et `types/admin-negotiation-documents.ts`, les jeux d'exemple dans `mocks/negotiation-documents.ts`, qui reprend les cinq documents de la maquette.
- Côté Rust, le crate `negotiation` gagne ses fichiers par couche (`domain/documents.rs`, `repo/documents.rs`, `repo/document_pages.rs`, `repo/corrections.rs`, `service/…`, `routes/documents.rs`, `routes/admin_documents.rs`, `jobs/extract.rs`), et l'extraction pure vit dans `domain/extraction/`, découpée par règle de R7. Aucun fichier n'approche 1000 lignes.

---

## Dettes trouvées en chemin

- **Les objets `private` déjà déposés sont dans le bucket public** (R4) : requête à jouer avant la mise en ligne.
- **L'écran A12 renvoie une liste vide d'espaces de négociation** (`identity/src/service/admin_users.rs:485-487`). Sans effet ici, puisque `expert` est de portée globale. À reprendre quand un rôle d'espace servira.
- **[02-domaine.md](../../docs/AppNego/02-domaine.md) l.29** rattache encore les thématiques à `negotiation_track`, alors que 0c a créé `negotiation_theme`. À corriger dans la documentation.
