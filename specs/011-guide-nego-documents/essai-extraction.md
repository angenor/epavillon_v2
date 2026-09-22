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

| # | Critère | Comment on le juge | Seuil | Résultat |
|---|---|---|---|---|
| 1 | **Ordre de lecture** | Le texte des pages témoins se lit dans l'ordre du PDF, colonne après colonne | Aucune inversion de paragraphe | |
| 2 | **En-têtes et pieds** | Titre courant et numéro de page absents du corps | Zéro ligne parasite sur les pages témoins | |
| 3 | **Notes** | La note de bas de page sort du corps et se rend en fin de page | La note est entière et hors du paragraphe | |
| 4 | **Tableaux et figures** | Repérés en bloc `origine`, jamais éclatés en paragraphes | Tous ceux des pages témoins | |
| 5 | **Césures** | Les mots coupés en fin de ligne se recollent ; les vrais tirets restent | Aucun mot coupé ; « États-Unis » intact | |
| 6 | **Italiques** | *global goal on adaptation* (p. 59) est marqué `terme` ; on compte les faux termes | Le vrai terme présent ; moins de 20 % de faux, sinon filtre anglais (R7) | |
| 7 | **Sommaire** | Signets du PDF ou titres repérés, confrontés au sommaire de référence | Chapitres et sous-parties à la bonne page | |
| 8 | **Pages** | « 59 » désigne la page imprimée 59, pas l'indice du fichier | Étiquettes justes | |

## Les mesures

| Mesure | Résultat |
|---|---|
| Poids du PDF | |
| Poids de la forme lisible, brute puis compressée (gzip) | |
| Nombre de pages à bloc `origine` | |
| Poids moyen d'une image de page (1080 px, JPEG 75) | |
| **Poids de la copie gardée** selon R2 : forme lisible, plus les pages à bloc `origine` | |
| Durée de l'extraction, images comprises, sur un poste de développement | |

## La conclusion

À remplir : **issue A, B ou C** ([R1](research.md)), ce qu'on garde sur le téléphone ([R2](research.md)), et les règles de R7 ajustées. Elle se relit avec le commanditaire avant la phase 2.
