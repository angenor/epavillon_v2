# Essai d'extraction — le *Guide des négociations — CdP30*

**Première phase de l'étape.** Rien du modèle, de l'API ni des écrans ne s'écrit avant que cette grille soit remplie et que le commanditaire en ait lu la conclusion. Elle confirme ou renverse R1 et R2 de [research.md](research.md).

## Le matériau

- **Le PDF réel du guide de la CdP30**, fourni par le commanditaire. Ses droits sont acquis par l'IFDD ([donnees-lecteur.md](../../docs/AppNego/design/donnees-lecteur.md)).
- **Le sommaire de référence**, pages 8 à 88, et le passage de la page 59, tous deux dans `donnees-lecteur.md` : ils servent d'étalon.
- **Le fichier reçu le 22/09** : `.essais/guide-cdp30.pdf` — *Guide des négociations, édition 2025, CdP30*, **90 pages, 2,9 Mo** (et non 92 pages et 9 Mo comme la maquette l'annonçait), SHA-256 `9af73eb2…34164eb`. Les mesures de R2 se rapportent donc à 2,9 Mo.
- **Il ne se commite pas.** Le PDF reste hors du dépôt, sous `.essais/` (ignoré par Git). Les résultats — grille, mesures, extraits courts — se commitent dans ce fichier.

## Le montage

- **Un binaire d'essai jetable**, `backend/crates/modules/negotiation/examples/essai_extraction.rs` :
  - il lit le PDF avec `pdfium-render` et applique les règles de R7 ;
  - il écrit dans `.essais/sortie/` la forme lisible en JSON, une image JPEG par page à 1080 px et en qualité 75, et un rapport texte ;
  - on le lance par `cargo run --example essai_extraction -- <pdf>`.
- **Si PDFium bute sur un critère**, le même contrôle se rejoue avec pdfplumber et pypdfium2, dans un script Python jetable sous `.essais/`. Il ne vit que le temps de la comparaison.
- **PDFium** s'installe par `make pdfium`, écrit dans cette phase et gardé ensuite.

## La grille

Chaque critère se juge sur des **pages témoins**, choisies d'après une première lecture du fichier :

| Page | Ce qu'elle éprouve |
|---|---|
| 6 et 7 | Le sommaire imprimé, à confronter aux signets du PDF ; ses titres en italique gras ne sont **pas** des termes |
| 9 | Un tableau à trois colonnes, avec des puces |
| 11 | Une note de bas de page ; le pied « © GUIDE DES NÉGOCIATIONS… » et son numéro, sur chaque page |
| 12 | *Global Goal on Adaptation*, *Global Stocktake*, *New Collective Quantified Goal* : les vrais termes anglais en italique |
| 13 | Une frise dessinée : une figure, à rendre en bloc `origine` |
| 14 | Quatre encadrés côte à côte (CdP29, CRP19, CRA6, SB61) |
| 15 et 47 | **Du texte sur deux colonnes** suivi d'une liste pleine largeur |
| 21 à 25 | Des tableaux sur plusieurs pages |
| 48 | Des encadrés en pointillés et une liste en anglais en italique |
| 50 à 64 | Des tableaux « Questions de négociations / Points de convergence / Points de divergence » |
| 59 | 3.6.1 GGA, la page du lecteur de la maquette |
| 66 et 67 | Le tableau des sigles, où l'anglais est entre parenthèses et **pas** en italique |
| 88 | La bibliographie, pleine d'adresses coupées sur deux lignes |

**Déjà vu à la lecture** : la couche de texte perd le trait d'union d'un mot coupé en fin de ligne — « Convention-cadre » y devient « Conventioncadre », « socio-économiques » « socioéconomiques ». La règle des césures de R7 doit donc **réinsérer** un tiret attesté, et pas seulement en retirer.

**Mené le 23/09** avec `pdfium-render` 0.9.4 et PDFium chromium/7881 (`make pdfium`), par `cargo run -p negotiation --example essai_extraction -- ../.essais/guide-cdp30.pdf`. La sortie — `brut.json`, `forme-lisible.json`, `rapport.md` et les 90 images — reste sous `.essais/sortie/`, hors de Git.

| # | Critère | Comment on le juge | Seuil | Résultat |
|---|---|---|---|---|
| 1 | **Ordre de lecture** | Le texte des pages témoins se lit dans l'ordre du PDF, colonne après colonne | Aucune inversion de paragraphe | ✅ **Aucune inversion.** p. 15 : le paragraphe pleine largeur, la colonne gauche, la colonne droite, puis la liste. p. 59 : « Questions de négociations à la CdP30 », puis ses quatre enjeux, puis « Points de convergence »… **Le flux du PDF est déjà l'ordre de lecture** — Word écrit colonne après colonne —, sauf pour les formes flottantes, écrites en fin de page : la frise de la p. 14 venait après la note de bas de page. Elles sont repérées et replacées à leur hauteur |
| 2 | **En-têtes et pieds** | Titre courant et numéro de page absents du corps | Zéro ligne parasite sur les pages témoins | ✅ **Zéro, sur les 90 pages.** « © GUIDE DES NÉGOCIATIONS, ÉDITION 2025, OIF/IFDD, 2025. » et le numéro sont écartés sur 87 pages. Deux pièges levés : les annexes ont leur pied plus bas (797 pt contre 782), donc le seuil « la moitié des pages » ne les prenait pas ; et « 9 » et « 59 » doivent être la même ligne au numéro près |
| 3 | **Notes** | La note de bas de page sort du corps et se rend en fin de page | La note est entière et hors du paragraphe | ✅ **42 notes sur 42**, entières, en fin de page. p. 11 : « Aucune CdP n'a eu lieu en 2020 en raison de la pandémie de COVID-19 ; la CdP26 initialement prévue à Glasgow cette année-là a été reportée à 2021. » L'appel reste dans le texte, en exposant : « … de la CCNUCC¹. Les principales étapes… » |
| 4 | **Tableaux et figures** | Repérés en bloc `origine`, jamais éclatés en paragraphes | Tous ceux des pages témoins | ✅ **16 tableaux** (p. 9, 21 à 25, 66 à 71, 83) et **8 figures** (couverture, crédits, p. 13, 14, 82, dos) en bloc `origin`. Les encadrés en pointillés de la p. 48 **ne sont pas des figures** : du texte flottant sans autre dessin que son cadre. Ils se recomposent à leur place, en deux listes. Les pages 50 à 64 (« Questions / Points de convergence / Points de divergence ») n'ont pas de grille : elles se recomposent en titres et paragraphes |
| 5 | **Césures** | Les mots coupés en fin de ligne se recollent ; les vrais tirets restent | Aucun mot coupé ; « États-Unis » intact | ✅ **35 traits d'union de fin de ligne, tous justes** : Convention-cadre, socio-économiques, porte-parole, seront-elles, « (+/-0,13 °C) », et les adresses de la bibliographie. « États-Unis » est intact (5 fois). **Le tiret n'est pas perdu** : PDFium le remplace par un caractère de contrôle (U+0002) en fin de ligne, que la règle lit. Le guide, sorti de Word sans coupure automatique, n'a **aucune** césure typographique : la branche « recoller » ne s'y déclenche pas, et se vérifiera sur `fixtures/petit.pdf` (T036) |
| 6 | **Italiques** | *global goal on adaptation* (p. 59) est marqué `terme` ; on compte les faux termes | Le vrai terme présent ; moins de 20 % de faux, sinon filtre anglais (R7) | ✅ **avec le filtre anglais.** **La p. 59 n'a aucun italique** dans le vrai PDF : le terme de la maquette n'y est pas. *Global Goal on Adaptation* est en italique p. 12 et p. 67, et il est marqué. Sur 31 italiques maigres, **18 ne sont pas des termes** (58 %) — *Le poids du temps*, *a minima*, *ad hoc*, « *ouvert* »… Le filtre en garde **13, dont un faux** (*Annexe II*) : 8 %. Les 48 italiques gras sont tous des titres, jamais des termes |
| 7 | **Sommaire** | Signets du PDF ou titres repérés, confrontés au sommaire de référence | Chapitres et sous-parties à la bonne page | ✅ **Le PDF n'a aucun signet.** Les titres repérés retrouvent **toutes les entrées du sommaire de référence, à la bonne page**, de « Résumé exécutif » (8) à « Bibliographie » (88), sur trois niveaux. En plus : les pages liminaires, « 1. Principaux résultats par thématiques » (16), les sous-parties de l'annexe A.3 et les présentations A.7 à A.9. La table des matières imprimée (p. 6-7) est reconnue à ses points de conduite et n'engendre aucun titre |
| 8 | **Pages** | « 59 » désigne la page imprimée 59, pas l'indice du fichier | Étiquettes justes | ✅ **Le PDF ne porte pas d'étiquettes de page.** Le numéro se lit dans le pied écarté : 87 pages numérotées, et l'indice pour les trois qui ne le sont pas (1, 2, 79). **Dans ce guide, numéro imprimé et indice coïncident** : la couverture est la page 1 |

**T007 est sans objet** : aucun critère n'échoue avec PDFium, pdfplumber n'a pas été essayé.

## Les mesures

| Mesure | Résultat |
|---|---|
| Poids du PDF | 2 884 088 octets — **2,9 Mo** |
| Poids de la forme lisible, brute puis compressée (gzip) | 309 010 octets, puis **74 108 octets** (gzip -9) — **74 Ko** pour 219 462 caractères |
| Nombre de pages à bloc `origine` | **19 sur 90** (21 %) : 1, 2, 9, 13, 14, 21 à 25, 66 à 71, 82, 83, 90 |
| Poids moyen d'une image de page (1080 px, JPEG 75) | 247 936 octets — **242 Ko** (1080 × 1527 px) ; 217 Ko sur les seules pages à bloc `origin` |
| **Poids de la copie gardée** selon R2 : forme lisible, plus les pages à bloc `origine` | 74 Ko + 4 124 842 octets = **4,2 Mo — plus lourd que le PDF** (2,9 Mo). Mesuré à titre d'indication : environ 3,1 Mo en JPEG 4:2:0 de qualité 60, 2,8 Mo en PNG de 16 couleurs |
| Durée de l'extraction, images comprises, sur un poste de développement | **1,8 s** en version optimisée (texte 0,23 s, images 1,56 s), sur un Mac M ; 18 s en version de débogage. Le budget d'une minute est tenu de très loin |

## La conclusion

### Issue A — PDFium, dans le worker Rust

PDFium tient les huit critères, après un traitement écrit en Rust. **R1 est confirmé** : ni pdfplumber, ni le service Python d'ADR-004 à cette étape. Le texte, la police, le cadre de chaque caractère, les objets graphiques et le rendu des pages suffisent ; aucune autre bibliothèque n'a manqué.

Deux faits du vrai fichier corrigent ce que la recherche supposait :
- **ni signets ni étiquettes de page** : le sommaire vient des titres repérés, et le numéro de page du pied imprimé ;
- **le poids de la graisse que rend PDFium est inutilisable** (220, 235, 265…) : le gras se lit au nom de la police (« Bold », « Black »), et les titres de chapitre, dessinés en gras sur une police maigre, se reconnaissent à leur corps.

### Ce que garde le téléphone — une décision à prendre

R2 supposait un PDF de 9 Mo, et une copie « trente fois moins lourde ». Le vrai guide pèse 2,9 Mo, et **les 19 pages à tableau ou figure pèsent à elles seules 4,1 Mo en images**. La copie de R2 pèse donc **4,2 Mo, plus que le PDF**. Le seuil de R2 — tout en images au-delà d'un tiers de pages — n'est pas atteint (21 %).

Trois voies, présentées au commanditaire (T009) :

| Voie | Poids | Ce qu'on perd |
|---|---|---|
| **R2 tel qu'écrit** : le texte et les 19 pages d'origine | 4,2 Mo, environ 3 Mo avec un encodage plus léger | Rien : le guide se lit **en entier** sans réseau, tableaux compris (SC-002) |
| Le texte seul ; les pages d'origine avec le réseau | 74 Ko | Sans réseau, un tableau ne se voit pas : « Page d'origine disponible avec le réseau ». Son texte reste lisible à plat |
| Le PDF sur le téléphone | 2,9 Mo | Un moteur de rendu PDF dans l'application (pdf.js, plus d'un mégaoctet) : écarté par R2 |

**Tranché le 23/09 par le commanditaire : on garde tout** — R2 tel qu'écrit, avec l'encodage plus léger à régler en phase 4 ([ADR-021](../../docs/AppNego/adr/021-pdfium-dans-le-worker.md)). SC-002 demande le guide lisible « en entier » en mode avion, et la maquette annonce déjà 9,4 Mo à retirer du téléphone.

### Le contrat de la forme lisible, étendu

Un bloc `origin` porte **le texte de sa zone**, dans l'ordre du flux (`text`, des segments). Sans lui, **22 276 caractères** — 10 % du texte, dont tout le tableau des sigles — échappaient à la recherche. Le lecteur l'affiche replié sous « Voir la page d'origine » : l'invariant « rien ne se cherche qui ne s'affiche pas » tient. Une lectrice d'écran y trouve aussi de quoi lire un tableau. Le contrat s'étend sans se redéfinir : un lecteur qui ignore `text` reste juste.

### Les règles de R7, ajustées

Elles sont reportées dans [R7](research.md), qui fait foi. Ce qui a changé :
- **l'ordre du flux fait foi** ; seules les formes flottantes se replacent. Les colonnes par abscisse ne sont pas écrites ;
- **les pieds répétés** sur le cinquième des pages, au numéro près ;
- **les notes** sous un filet court et isolé, depuis la marge gauche la plus fréquente ;
- **l'encadré**, troisième sorte de zone, qui se recompose ;
- **le tiret de fin de ligne gardé** sauf preuve contraire, sans condition de minuscule ;
- **l'italique maigre seul**, filtré en anglais ;
- **le sommaire imprimé** reconnu à ses points de conduite ;
- **le numéro de page lu dans le pied** ;
- **les paragraphes**, qui n'avaient pas de règle.

### Un écart avec la maquette

`docs/AppNego/design/donnees-lecteur.md` donne *global goal on adaptation* comme terme touchable de la page 59. **Dans le vrai guide, la p. 59 n'a aucun italique** : l'objectif y est nommé en français. Le lecteur de la maquette montre donc un terme que le vrai texte ne portera pas. Le comportement ne change pas ; seule la démonstration change de page (p. 12).

### Limites connues, sans effet sur l'issue

- En bibliographie, une adresse seule sur sa ligne, après une phrase close, devient son propre paragraphe. Aucun texte n'est perdu.
- Un italique anglais d'un seul mot capitalisé (*Guide*) n'est pas marqué ; un intitulé en chiffres romains (*Annexe II*) l'est à tort.
