# Écarts entre « 01 — Système » et les pages

Relevé mécanique (styles en ligne de toutes les pages, 20 septembre 2026) confronté aux sections 1 à 7 du système. Listés, pas tranchés : chaque ligne appelle une décision — corriger la page, ou compléter le système.

## Couleurs

1. **#2E3F0E** (squelette de chargement en sombre, filets de la planche 00 en sombre, page 04 sombre) est utilisé dans la logique du système et sur trois pages, mais absent de la liste écrite des nuances de la section 1 ; la section C le donne à 1,3:1 comme filet à ne pas garder. Squelette : légitime (pas un filet) ; à ajouter à la liste. Filets sombres de la page 00 : la planche date d'avant C, qui a retenu #6E9A2A.
2. **#FFD500** sur **#233400** : le système le réserve à « accent sur vert foncé seulement » (compteur, message éphémère). Les badges de maquette (« 1a », « 2b »…) l'emploient sur toutes les pages, avec un rayon 6 px hors échelle — ce sont des repères de maquette, pas de l'interface. À ne pas reprendre dans l'application.
3. **#8FBF2F** (vert de charte) : la table de rôles le donne comme couleur « succès » du thème sombre ; les pages 00 et 04 en sombre emploient **#B5D66A** pour les marques d'état vertes. Deux verts sombres pour un même rôle.
4. Le thème sombre n'est montré que sur 00 · 3a, 04 · 09, 09 · 4a (écran verrouillé), 14 · 1h et 15 (vignette vidéo). Les rôles **pressé, focus, désactivé, bandeau « Remplacé par… », bulle envoyée, voile** n'ont pas de valeur sombre dans la maquette ; `theme.css` garde leur valeur claire.
5. Pictogramme **Document** : #557607 (`succes`) dans la ligne de document, #233400 dans le menu de la feuille basse et l'export (`docG24`). Le système ne dit pas quand l'un ou l'autre.

## Tailles de texte

6. **16 px** apparaît 18 fois sur la page 00, 17 fois sur 01 (sections A et B, antérieures à E), et sur 02, 04, 07 et 10 : titres de ligne de session (avant le passage à 17 décidé en E) et onglets de filtre (« Mes thématiques · Toutes · Salle »). L'échelle de la section 2 n'a pas de 16. À trancher : soit les onglets de filtre passent à 17, soit 16 entre dans l'échelle.
7. **14 px** italique : page 00 (12 fois, titre anglais d'avant E) et exemple barré de la section E. Sous le seuil de 15 px ; la page 00 est une planche d'archive.
8. **18 px** : le « Aa » du bouton de lexique (toutes les pages), le « GN » du monogramme. Absent de l'échelle ; c'est une taille de glyphe dans un bouton 48, pas une taille de texte.
9. **22 px** : une occurrence sur 01 (« Aa » de la planche des pictogrammes). Idem.
10. **13 px** est écrit « libellés d'onglets seulement » dans l'échelle, mais sert aussi à la marque de rôle, au compteur, au jour de la bande des jours, aux libellés de la feuille de partage et au sous-texte de la table de couleurs. Compléter la ligne 13 de l'échelle.

## Mesures

11. **Rayon 6 px** : uniquement les badges de maquette (voir 2). Hors application.
12. **Barre d'onglets** : la section 5 dit encore « quatre onglets » dans son en-tête de composant ; 4 septdecies et la page 12 disent cinq, avec onglets à largeur de libellé. La page 02 · 14a a été corrigée, 14b écarté ; les autres pages (00, 03, 05, 06, 07, 09, 10, 15, 16) montrent toujours quatre onglets de largeur égale.
13. **Colonne d'heure 62 px** : partout dans les listes, mais la section 3 la dit « seule largeur fixe » alors que les pages ajoutent le rail alphabétique (24), les champs d'heure (110), le compteur (22), l'avatar (40), l'interrupteur (52 × 32). Ce sont des composants, pas des colonnes ; la phrase mérite d'être précisée.
14. **Ligne de liste 48 px minimum** (section 3) contre **56 px** pour ligne de réglage, option de feuille basse, lignes à cocher ou à cercle (4 bis et suivants). Deux minimums, cohérents entre eux mais à écrire ensemble.
15. **Onglets de filtre** (sous l'en-tête, « Mes thématiques · Toutes · Salle ») : hauteur et air (12 / 9 px) ne sont écrits nulle part dans le système ; seule la pilule l'est.

## Pictogrammes

16. **pause** est défini seulement sur la page 15 (lecteur vidéo), absent de la famille de la section 4. À ajouter au système ou à remplacer.
17. **chevUp** et **minus** sont définis dans la logique du système et employés (04, 13, 14, 15, 17) mais n'ont pas de case ni de libellé dans la planche 4.
18. La planche 4 annonce « 44 » pictogrammes ; la famille en compte 56 (57 avec `pause`). Les ajouts des pages (Lien externe, Sommaire, Réglages de lecture, Marquer, Joindre, Répondre, Copier…) sont bien dans la planche, le compte ne l'est pas.
19. L'état **« Vérifié le … »** (FAQ) porte le bouclier coché, le même que « Relu par un expert » et « Expert ». Trois mots pour un signe ; à confirmer.
20. **Annulée** : croix (`close`) dans la section 6 et la feuille « Signaler » ; l'exemple d'erreur « Lecture impossible » emploie le triangle (`warn`) en rouge. Deux pictogrammes rouges, deux sens ; à écrire dans la section 6.

## Données et copie

21. **Annonces de l'IFDD** : la liste des canaux (12 · 1a) donne l'annonce de 09:00 comme dernière ; la conversation 13 · 1i a une seconde annonce à 11:20 (« Guide à jour »). Une seule chronologie à retenir (journal, « En attente »).
22. **Document proposé** : 12 et 13 disent « Nouvelle itération GGA — 12 nov., 10:05 » et « Proposé au classement — en attente d'un expert » ; le brief de la page 12 disait « Nouvelle itération GGA, 12 novembre (EN) » et « Proposé — en attente de classement ». La page 13 a été gardée (décision du 20/09).
23. **Marque de rôle** : corrigée sur 01, 02, 12, 13 (étiquette bordée). La légende « Bulle de message · avatar et rôle » de la section 5 a été mise à jour ; vérifier qu'aucune autre page ne montre encore le rôle sur jaune (relevé : aucune occurrence de `background:#D3B011` avec « Coordonnatrice »).
24. **Sans code** : le verrou du Socle (02 · 13) dit « Les échanges du réseau s'ouvrent avec le code reçu de l'IFDD ou de votre groupe » ; celui de la page 12 · 1k dit « …reçu dans votre groupe WhatsApp ou de l'IFDD ». Même sens, deux phrases.
25. **Synchronisé à** : le système (section 6) montre « Synchronisé à 11:35 » ; les pages 07 et 12 montrent 11:35 et 11:40 selon l'écran. L'instant unique est 11:40 ; 11:35 est l'heure de la dernière lecture hors connexion — à distinguer explicitement dans la section 6.

## Ce que le système ne couvre pas encore

26. Icône de l'application et écran d'installation (le monogramme « GN » tient la place).
27. Thème sombre des composants ajoutés après la page 04 (bulles, réactions, canaux, annuaire, feuille de partage).
28. Comportement au clavier physique / lecteur d'écran des composants à cercle et pilules décochables (ordre de focus, annonce d'état).
29. Largeur d'onglet à 390 px (vérifiée seulement à 360 px sur la page 12).
