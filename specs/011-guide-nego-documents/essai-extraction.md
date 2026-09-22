# Essai d'extraction — le *Guide des négociations — CdP30*

**Première phase de l'étape.** Rien du modèle, de l'API ni des écrans ne s'écrit avant que cette grille soit remplie et que le commanditaire en ait lu la conclusion. Elle confirme ou renverse R1 et R2 de [research.md](research.md).

## Le matériau

- **Le PDF réel du guide de la CdP30**, fourni par le commanditaire. Ses droits sont acquis par l'IFDD ([donnees-lecteur.md](../../docs/AppNego/design/donnees-lecteur.md)).
- **Le sommaire de référence**, pages 8 à 88, et le passage de la page 59, tous deux dans `donnees-lecteur.md` : ils servent d'étalon.
- **Il ne se commite pas.** Le PDF reste hors du dépôt, sous `.essais/` (ignoré par Git). Les résultats — grille, mesures, extraits courts — se commitent dans ce fichier.

## Le montage

- **Un binaire d'essai jetable**, `backend/crates/modules/negotiation/examples/essai_extraction.rs` :
  - il lit le PDF avec `pdfium-render` et applique les règles de R7 ;
  - il écrit dans `.essais/sortie/` la forme lisible en JSON, une image JPEG par page à 1080 px et en qualité 75, et un rapport texte ;
  - on le lance par `cargo run --example essai_extraction -- <pdf>`.
- **Si PDFium bute sur un critère**, le même contrôle se rejoue avec pdfplumber et pypdfium2, dans un script Python jetable sous `.essais/`. Il ne vit que le temps de la comparaison.
- **PDFium** s'installe par `make pdfium`, écrit dans cette phase et gardé ensuite.

## La grille

Chaque critère se juge sur les **pages témoins** : 8 et 9 (sommaire), 14, 21, 26, 59 et 60 (texte courant et italiques), une page à tableau et une page à note de bas de page repérées au premier passage, et 88 (bibliographie).

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
