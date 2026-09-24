# Recette — le lecteur montre le PDF d'origine (étape 1b)

Se déroule sur la **version construite** (`nuxt build` puis `node .output/server/index.mjs`), jamais sur le serveur de développement : la coquille et sa garde n'existent qu'une fois construites. Le guide réel est `.essais/guide-cdp30.pdf`, hors de Git.

## Préalables

- Base locale migrée : `migration.sql` de l'étape 1, puis celle de ce cycle, jouée deux fois ([data-model.md](data-model.md)). **Jamais `make check`.**
- API et worker lancés ; le guide publié depuis le back-office (étape 1, § 1 de sa recette), public ; un bulletin à colonnes publié ; un scan publié ; un document réservé publié.
- Un compte admis (accès négociateur), un compte sans accès, et aucune session.
- Le site ouvert sur l'adresse exacte d'`APP_PUBLIC_URL`, fenêtre de 360 × 780.

## § 1 — Le back-office

1. L'aperçu du guide montre chaque page à côté de son texte, dit à quoi sert le texte, donne le verdict et « Proposer « Texte agrandi » » coché par défaut. « Ouvrir tel quel » n'y est plus.
2. Sur le bulletin, retirer « Texte agrandi » ; le choix tient après une relance d'extraction ; « Revenir au verdict » le rend.
3. Le scan : l'aperçu dit que le texte ne s'est pas extrait ; « Texte agrandi » n'est pas proposable.
4. Un PDF protégé par mot de passe : l'extraction échoue, l'aperçu le nomme, « Publier » est refusé.

**Attendu** : FR-036 à FR-039.

## § 2 — La route du fichier

```bash
curl -sI  "$API/negotiation/documents/$GUIDE/file"                      # 200, Accept-Ranges: bytes, pas de Content-Encoding
curl -s -o /dev/null -w '%{http_code}\n' -H 'Range: bytes=0-262143' "$API/negotiation/documents/$GUIDE/file"   # 206
curl -s -o /dev/null -w '%{http_code}\n' -H 'Range: bytes=99999999-' "$API/negotiation/documents/$GUIDE/file"  # 416
curl -s -o /dev/null -w '%{http_code}\n' -H 'Range: bytes=0-1023'    "$API/negotiation/documents/$RESERVE/file" # 403 sans session
```

Avec la session du compte admis, le réservé rend `206` et `Cache-Control: private, no-store`. **Attendu** : FR-030, SC-009.

## § 3 — Lire en ligne, sans compte

1. Ouvrir le guide sans le télécharger, réseau ordinaire : la première page paraît en moins de 3 s, avant le fichier entier (onglet Réseau : la lecture, puis des `206` de 256 Ko, jamais un `200` du PDF). Puis en « 3G lente » : la jauge avance ; au bout de 3 s, « Lire le texte en attendant » et « Télécharger pour lire sans réseau » ; le texte s'ouvre à la même page en moins de 5 s après l'ouverture ; la page du PDF prend sa place quand elle arrive (23 à 28 s pour le guide), **sauf** après « Rester sur le texte ». Le lecteur ne bascule jamais pendant l'attente du réseau (`gn.lecture-bascules` inchangé).
2. La page tient la largeur ; défiler jusqu'à la page 61 ; le pied suit (« Page 61 sur 90 · … »).
3. Pincer sur le tableau des sigles jusqu'au plus fort : net après le geste ; double toucher : retour à la largeur.
4. Toucher une fois : la barre se déplie sur quatre emplacements ; toucher deux fois vite : grossit, sans déplier.
5. Appui long sur un paragraphe à deux colonnes de la page 59, « Copier », coller : l'ordre est gardé.
6. Passer en thème sombre : l'interface change, la page non.

**Attendu** : FR-001 à FR-012, SC-002, SC-003, SC-006.

## § 4 — Télécharger, lire en mode avion

1. Télécharger le guide : la fiche annonce ≈ 3,2 Mo ; « Mes documents » compte la même taille.
2. Couper le réseau au milieu : aucune copie ; relancer, laisser finir.
3. Mode avion, fermer l'application, rouvrir : « Reprise à la page 61 — … ».
4. Parcourir les 90 pages sans saccade ni page blanche qui dure ; grossir, revenir.
5. « Rechercher » « progrès collectifs » : n passages sur n pages, sans réseau ; en choisir un : sa page s'ouvre, le passage est surligné plein à sa place.
6. Le sommaire mène à la page de la section.

**Attendu** : SC-001, SC-004, SC-005.

## § 5 — « Texte agrandi »

1. Premier document ouvert sur ce téléphone : la ligne d'annonce paraît, une seule fois — rouvrir, ouvrir un autre document : elle ne revient pas.
2. Page 59, « Texte » dans la barre : la page 59 recomposée ; « Réglages » : le mode, le thème, les trois tailles ; Très grande.
3. Un renvoi « Tableau — page 59 » ramène aux pages, à l'endroit du tableau.
4. Fermer, rouvrir un autre document : il s'ouvre en texte. Ouvrir le bulletin : il s'ouvre en pages et le dit ; le mode gardé reste « Texte ».
5. Le scan : ni recherche, ni sommaire, ni choix du mode ; sa fiche le dit.

**Attendu** : FR-019 à FR-024, SC-012.

## § 6 — Notes de correction

1. L'expert pose une note sur un passage de la page 59 et une note sans passage sur la page 60.
2. Relire la bibliothèque avec le réseau, puis mode avion : sur la page 59, le filet en marge à la hauteur du paragraphe ; sur la page 60, en tête.
3. Déplier : texte et signature, la page reste visible, le passage à l'écran. En texte agrandi : la note borde le bloc.
4. Retirer une note, relire : elle disparaît de la copie sans retéléchargement.

**Attendu** : FR-032 à FR-035, SC-010.

## § 7 — Ce qui s'efface

1. Copie d'étape 1 simulée (fiche sans `format`) : à l'ouverture, elle s'efface, la place est rendue, « Non téléchargé ».
2. Le compte admis lit le réservé en ligne sans le télécharger, puis en télécharge un autre ; se déconnecter ; mode avion : aucun des deux ne s'ouvre.
3. « Tout retirer » : la place baisse d'au moins la taille des copies.

**Attendu** : FR-025 à FR-031, SC-007, SC-008.

## § 8 — Tailles, thèmes, garde

- Lecteur, barre, feuille « Réglages », annonce, note dépliée à 320, 360 et 390 px, clair et sombre, en « Très grande » : aucun débordement de l'interface.
- `npm run verifier-garde:guide-nego` sur la version construite : les fichiers `pdfjs/` sont servis.
- `npm run check:guide-nego` : aucun sélecteur hors de la borne, y compris la feuille du visionneur.
- Non-régression du site : l'accueil, un média, une édition.
- `make check-safe`, **API arrêtée**.

## Ce qui ne se fait pas depuis un poste

Le guide lu en mode avion sur un **Android de milieu de gamme** et sur un **iPhone réels**, après une nuit : par le commanditaire, au § 15 de [DEPLOIEMENT.md](../../docs/DEPLOIEMENT.md), avec T112 (0b) et T116 (étape 1).
