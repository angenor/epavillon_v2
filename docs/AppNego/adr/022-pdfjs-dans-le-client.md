# ADR-022 — pdf.js dans le client, et la copie gardée porte le PDF

**Statut** : accepté — 24/09/2026, après l'essai sur le vrai guide de la CdP30 ; issue A retenue par le commanditaire, avec quatre décisions. **Remplace** la règle « le PDF ne va jamais sur le téléphone » d'[ADR-021](021-pdfium-dans-le-worker.md), dont l'extraction reste en vigueur.

## Contexte

Le commanditaire a lu le texte recomposé de l'étape 1 et a décidé que le lecteur montre le PDF d'origine ([spec 012](../../../specs/012-guide-nego-lecteur-pdf/spec.md)). Le texte extrait reste pour la recherche, le sommaire et un second mode, « Texte agrandi ». Afficher un PDF dans une application installée, sans réseau et sur un téléphone de milieu de gamme, demande un moteur de rendu dans le client. C'est une dépendance d'ampleur (constitution, contraintes techniques).

Le choix du moteur et de ses réglages est dans [research.md](../../../specs/012-guide-nego-lecteur-pdf/research.md), R1 à R7. L'essai ([essai-lecteur.md](../../../specs/012-guide-nego-lecteur-pdf/essai-lecteur.md)) a lu le vrai guide dans Chromium, en profil Android avec le processeur bridé, et dans WebKit. Le guide fait 90 pages et 2,9 Mo, avec des tableaux de 1 300 à 2 400 pointillés par page.

## Décision

**Le moteur.**
- **`pdfjs-dist` 6.3.289** (Apache-2.0), version exacte, **build `legacy`**, avec son visionneur `PDFViewer`. On n'écrit ni la virtualisation, ni le plafond des canevas, ni le redessin net de la zone visible.
- pdf.js n'entre que par `utils/guide-nego/pdf/charger.ts`. La coquille garde le module, le travailleur, les `wasm`, les profils ICC et **les polices standard, les quatre Liberation comprises**. Un bulletin ou un guide de l'IISD ou de l'IIED peut ne pas embarquer ses polices, et hors connexion une police manquante ne se rattrape pas. La coquille grossit d'environ **1,3 Mo compressé**, une seule fois.

**La copie gardée porte le PDF.**
- Elle contient le PDF entier et la forme lisible, sous un numéro de format (R5). Le téléchargement lit **le fichier entier**, jamais par plages.
- Les images de pages de PDFium ne vont plus au téléphone : elles restent pour l'aperçu du back-office.

**La lecture en ligne se fait par plages, et elle se voit.**
- pdf.js lit par un **transport de plages écrit ici** (`PDFDataRangeTransport`), avec `disableAutoFetch`. Sans ces réglages, il télécharge le guide entier avant la première page (2,8 Mo, mesuré). Le transport sait ce qui est demandé et ce qui est reçu : il alimente la progression et le délai de la bascule.
- La forme lisible (~74 Ko compressés) se lit **avant** le PDF, pour ne pas partager le débit avec lui.
- L'attente de la première page **montre sa progression** : ce qui est reçu sur ce qui est demandé.
- **Au bout de 3 s sans page**, elle offre deux sorties :
  - « Lire le texte en attendant » : « Texte agrandi », à la même page ;
  - « Télécharger pour lire sans réseau ».

  La page du PDF prend la place du texte quand elle est prête, **sauf si la personne a choisi de rester sur le texte**.
- Sur un réseau très lent, la première page de ce guide met 23 à 28 s : la couverture est une photo de 396 Ko, et le reste est la table des objets en fin de fichier, les polices et le surplus des morceaux. C'est **accepté par le commanditaire**. Une couverture allégée est écartée : elle ne descendrait pas sous 8 s. SC-003 le dit désormais : 8 s sur un réseau ordinaire et sur une copie gardée ; **une première lecture en moins de 5 s, par le texte**, sur un réseau très lent.

**La bascule ne sanctionne que l'échec de l'affichage, jamais le réseau.**
- La détection des fonctions manquantes ouvre « Texte agrandi » avant tout chargement (FR-012 bis). Elle teste les **blocs `static {}` de classe** (une sonde de syntaxe), le **module dans un `Worker`**, `structuredClone` et `Path2D`, jamais la version lue dans l'identifiant du navigateur.
- La seconde sécurité bascule sur une erreur de pdf.js, ou sur une première page non dessinée après **8 s de travail sans attente du réseau** : le délai ne court que lorsqu'aucune plage demandée n'est en route, et s'arrête dès qu'une nouvelle part. Il compte donc le temps de pdf.js, pas celui du réseau. Sur une copie gardée, rien ne passe par le réseau et les 8 s courent depuis l'ouverture.

**iOS 16.4 et au-delà.**
- Le build `legacy` contient des blocs `static {}` de classe, une syntaxe d'iOS 16.4 : en dessous, le fichier entier est refusé. La détection ouvre alors « Texte agrandi ».
- Il appelle `Promise.withResolvers` (iOS 17.4) sans l'émuler. Un remplacement de quelques lignes l'installe **dans la page et dans le travailleur**, avant pdf.js. Le seuil passe ainsi d'iOS 17.4 à 16.4 : l'iPhone 8 et le X (iOS 16.7) affichent les pages. `ReadableStream.prototype[Symbol.asyncIterator]` s'émule de même.
- Un test simule l'absence de ces fonctions dans les deux contextes. La mesure sur un vrai téléphone reste à T084 : sans Xcode, aucun simulateur iOS n'a tourné sur le poste.

**Trois défauts de l'essai, corrigés par construction.**
- **Le dimensionnement de la page.** pdf.js compte en `content-box` ; tout ce qui est sous `.pdfViewer` y revient, par une règle de la feuille bornée à Guide Négo, que `check:guide-nego` exige. Sans elle, la bordure de la page rétrécit l'image (301 px au lieu de 319) et la couche de texte, restée à 319, décale surlignages et sélection d'une ligne ou plus.
- **Le texte de la page affichée se lit dans le DOM.** pdf.js 6 ne publie plus sa couche de texte (`textLayer.textLayer`, `textDivs`). Le repérage lit les éléments de `getTextContent()` et marque par les éléments de la couche affichée, dans le même ordre.
- **Le repérage cherche en deux temps** : le texte tel quel, puis sans puces, numéros de liste ni appels de note seuls sur leur élément. Il cherche sur la page, puis la suivante, puis la précédente. À l'essai : 50 recherches sur 50, 10 notes sur 10. Chaque cas manqué au premier temps devient un test.

## Conséquences

- **ADR-021** garde PDFium, l'extraction dans le worker et l'absence de service Python. Seule sa règle « le PDF ne va jamais sur le téléphone » tombe.
- Une route de l'API sert le PDF par plages, l'accès vérifié à chaque morceau, sans compression, relais compris (R3). Le transport de plages l'appelle avec la session.
- La feuille du visionneur est une **copie bornée**, engendrée par la construction et par le contrôle (R4). La règle `content-box` en fait partie.
- **Point ouvert, non corrigé ici** : la règle qui décalait pdf.js vient de la **réinitialisation générale du site** — la préface de Tailwind, `* { box-sizing: border-box; margin: 0; padding: 0; border: 0 solid }`, importée par `assets/css/main.css` — et `base.css` de Guide Négo la reprend pour lui-même. Elle atteint toute l'application, contrairement à FR-026 de l'étape 0a. Elle est inscrite dans [progress.md](../progress.md).
- Les mesures du poste ont deux limites : le bridage du processeur n'atteint pas le travailleur, et WebKit ne se bride pas. Un Android de milieu de gamme et un iPhone réels tranchent, à T084.
