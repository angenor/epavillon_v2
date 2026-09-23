# ADR-021 — PDFium dans le worker, et le téléphone garde tout

**Statut** : accepté — 23/09/2026, après l'essai d'extraction sur le vrai guide de la CdP30 ; la copie gardée tranchée par le commanditaire. **Confirme** R1 et R2 de [specs/011-guide-nego-documents/research.md](../../../specs/011-guide-nego-documents/research.md) ; **précise** [ADR-004](004-rust-en-facade-python-au-sidecar.md) pour l'étape 1.

## Contexte

Le lecteur de Guide Négo recompose le texte du PDF publié : taille réglable, termes anglais touchables, notes de correction posées sur une page. Tout dépend d'une extraction qui rende l'ordre de lecture, les notes, les tableaux, les césures et les italiques. Deux voies étaient ouvertes : **PDFium dans le worker Rust**, ou **pdfplumber** dans le service Python d'ADR-004, qu'il aurait fallu monter dès cette étape (trois à quatre jours). PyMuPDF, le meilleur extracteur, est sous AGPL.

L'essai ([essai-extraction.md](../../../specs/011-guide-nego-documents/essai-extraction.md)) a porté sur le vrai fichier : 90 pages, 2,9 Mo, sans signets ni étiquettes de page.

## Décision

- **L'extraction se fait par PDFium**, au travers de `pdfium-render` 0.9.4 (MIT ou Apache-2.0), avec le binaire chromium/7881 de `bblanchon/pdfium-binaries` (BSD-3 ou Apache-2.0), versions épinglées l'une sur l'autre. **Dans le worker seulement** : l'API sert, elle n'extrait pas. La bibliothèque native se charge depuis `PDFIUM_LIB_PATH` ; `make pdfium` l'installe sur un poste.
- **Aucun service Python à l'étape 1.** ADR-004 reste la voie de l'assistant (étape 7).
- **Le téléphone garde la forme lisible et les images des pages à tableau ou à figure** (R2 tel qu'écrit), pour que le guide se lise **en entier** sans réseau. Le PDF ne va jamais sur le téléphone.

## Conséquences

- L'essai a tenu les huit critères, et l'extraction complète dure 1,8 s en version optimisée. Les règles de recomposition, ajustées, sont dans R7.
- **La copie gardée pèse plus que le PDF** : 74 Ko de texte, plus 4,1 Mo d'images pour les 19 pages concernées. L'encodage des images se règle en phase 4 pour descendre vers 3 Mo. Le téléchargement se fait complet ou pas du tout.
- **Un bloc `origin` porte le texte de sa zone** : 10 % du texte du guide, dont le tableau des sigles, reste cherchable. Le lecteur l'affiche replié sous « Voir la page d'origine ».
- L'image du worker embarque la bibliothèque PDFium : une ligne de plus au Dockerfile, et au § 15 de [DEPLOIEMENT.md](../../DEPLOIEMENT.md).
- `.essais/` reste hors de Git : le PDF ne se commite pas. Les tests de l'extraction portent sur un petit PDF fabriqué, `negotiation/tests/fixtures/petit.pdf`.
