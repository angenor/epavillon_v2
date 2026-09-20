# Guide Négo — composants

Extrait de « 01 — Système » (sections 4 bis à 4 septdecies et 5), état du 20 septembre 2026. Pour chaque composant : rôle, variantes, états, mesures, règles d'usage, et l'écran où le voir — `page · data-screen-label`. Les valeurs renvoient aux jetons de `tokens.json` / `theme.css` ; les hexadécimaux sont rappelés entre parenthèses pour la lecture.

États communs à tout ce qui se touche (section 5) : **repos** · **pressé** (fond `fond-2` #ECFFD3, ou assombrissement de l'aplat) · **focus** (anneau 3 px `focus` #3C5404 à 2 px du bord) · **désactivé** (texte `gris` sur `gris-pale` #D9D8D6, jamais de transparence) · **chargement** (arc qui tourne, ou squelette). Chaque composant est montré en clair ; le sombre applique les jetons de la table 1.

---

## Navigation et en-tête

### Barre d'onglets
- **Rôle** : cinq onglets — Accueil · Négociations · Francophonie · Échanges · Ressources — pictogramme 26 au-dessus du libellé 13.
- **Mesures** : 68 px de haut, filet 2 px `filet-fort` dessus. Largeur d'onglet = libellé + 3 px d'air de chaque côté, 48 px au moins (mesuré police chargée : 50 · 85 · 87 · 64 · 74 = 360). À 72 px égaux, « Négociations » (78) et « Francophonie » (81) débordent.
- **États** : actif = filet 3 px dessus + graisse 700 + `accent` ; pressé = fond `fond-2` ; focus = anneau intérieur. Libellé jamais tronqué ni abrégé, jamais sous 13 px.
- **Compteur** : carré 22 px `attention-aplat`, chiffre 13/700 noir, posé en haut à droite du pictogramme ; il totalise les non-lus (Échanges).
- **Voir** : `12 · 1a Liste des canaux` (mesure vive sous l'écran) ; `02 · 14b Échanges en en-tête` (écarté, pour mémoire) ; pas de barre sous une conversation ni dans le lecteur.

### En-tête d'écran avec bouton « Aa »
- **Rôle** : titre 28/700 `titre`, sous-titre 17/600 `accent`, filet 3 px `filet-fort` dessous ; à droite, le bouton « Aa » du lexique (48 px, bord 2 px, texte 700 18 px) — ouvert : `titre` plein, texte blanc. Retour 48 px à gauche sur un écran secondaire ; autres actions 48 px (Notifications avec compteur, Mon compte, Marquer).
- **Variantes** : ligne de connexion au-dessus du titre (« Synchronisé à 11:40 » 15/600 `succes` avec picto 16 ; « Hors connexion — lu à 11:35 » 15/600 gris) ; en lecture, réduit à une ligne de 48 px (retour, titre du document 15/600 gris, « Aa »).
- **Voir** : `02 · 07 Accueil` (en ligne / hors connexion), `04 · 01 Lecture`, `12 · 1a Liste des canaux`.

### Bandeau de connexion
- **Rôle** : première ouverture hors ligne — bandeau plein `attention-aplat` #D3B011, texte noir 15/700, picto Hors connexion 18, 48 px min, pleine largeur, sous l'en-tête ; puis rappel discret dans l'en-tête. Dit ce qui reste lisible et quand la donnée date.
- **Voir** : `02 · 07 Accueil`, `07 · 1b Hors connexion`, `12 · 1l Hors connexion`, `13 · 1f`.

---

## Listes

### Ligne de session
- **Anatomie** : colonne d'heure 62 px (début 20/700 `titre`, fin 15/600 gris dessous) · type 15/700 gris au-dessus du titre (Coordination de groupe · Consultations informelles · Groupe de contact · Aparté · Plénière) · titre FR 17/600 · titre anglais 15/400 droit préfixé « EN » 600 · salle 20/700 `accent` + accès (Ouverte / Accès limité, picto 18) · thématique 15 gris · marque d'état. Filet 1 px, 10 px d'air.
- **En cours** : l'aplat `attention-aplat` couvre l'heure de début seule (option B1 retenue), la fin reste dessous. **Terminée** : tout passe en gris. **Déplacée** : ancienne heure 15/600 barrée (trait 2 px) au-dessus de la nouvelle ; ancienne salle dans la marque. **Sans fin annoncée** : début seul. **Chargement** : squelette.
- **Pressée** : fond `fond-2`.
- **Voir** : `01 · A1`, `07 · 1a Nominal`, `09 · 1c` (Chevauchement), `17 · 1a` (case à cocher devant la colonne d'heure).

### Ligne d'activité
- **Rôle** : une réunion de la Francophonie ou une activité du Pavillon ; porte son agenda d'origine en 15/700 gris (Réunions de la Francophonie · Pavillon de la Francophonie · Sessions de négociation). Même anatomie que la ligne de session.
- **Voir** : `02 · 07 Accueil`, `10 · 1a`.

### Ligne de document
- **Anatomie** : picto Document 24 `picto-document` #557607 (ou Lien externe 24 à sa place) · type 15/700 gris · titre 17/600 · pages, poids, éditeur 15 gris · marques en dessous (Téléchargé · À jour · Nouveau · Réservé · Lien externe · Remplacé par…) · chevron.
- **Téléchargement** : barre 6 px `accent` sur `gris-pale`, pourcentage écrit (« Téléchargement — 35 % »). Dans une bulle : bordée 1 px, état de classement dessous (Proposé au classement → Classé).
- **Voir** : `03 · 01 Bibliothèque`, `03 · 05c Téléchargement en cours`, `13 · 1a`, `12 · 1i Proposé, en attente`.

### Ligne de canal (Échanges)
- **Anatomie** : picto du type (Échanges · cloche = annonce · cadenas = réservé ; un privé porte l'avatar 40) · nom 17/600 · dernier message 15 gris sur une ligne (ellipse) · heure 15 à droite. « Réservé au réseau » : marque Réservé.
- **Non-lu** : nom 700, aperçu 600 noir, heure 700, carré jaune 10 px + nombre 15/700 sous l'heure. Hors connexion : « n message à envoyer » (horloge `attention`).
- **Voir** : `12 · 1a Liste des canaux`, `12 · 1l Hors connexion`.

### Ligne d'annuaire · membres
- **Anatomie** : avatar 40 · nom 17/600 · « Pays · thématiques » 15 gris · marque de rôle s'il y en a · chevron (absent pour qui n'est pas dans l'annuaire).
- **Règle** : n'y figurent que les personnes qui ont activé « Apparaître dans l'annuaire ». Ni adresse ni téléphone.
- **Voir** : `12 · 1b Annuaire du réseau`, `12 · 1j Réglages du canal Adaptation`.

### Ligne de réglage
- **Anatomie** : picto 24 `picto` · libellé 17/600 · valeur ou état 15 · chevron ; ou interrupteur à droite. 56 px min, filet 1 px. Pressée : fond `fond-2`.
- **Voir** : `02 · 11 Profil`, `12 · 1b` (« Apparaître dans l'annuaire »).

### Ligne de question à l'expert
- **Anatomie** : question 17/600, état en seconde ligne — En attente (horloge `attention`, heure d'envoi) · Répondue (coche `succes`, heure et expert) · Ajoutée à la FAQ (picto Quiz `accent`, date, sortie « Lire dans la FAQ » ; le texte passe en gris). La réponse tient sous sa question : avatar, nom + marque Expert, bulle reçue, heure, retour Oui / Non.
- **Voir** : `12 · 1e Questions aux experts`, `05 · 03b Question envoyée`.

### Ligne de module (Formations)
- « Module n · groupe » 15/700 gris au-dessus du titre · « Vidéo · durée » · progression en mot et en chiffre (« Vu à 26 % » `accent` + jauge · « Terminé » gris, titre grisé · « Non commencé » 600 gris) · marque du quiz sur la même ligne (Quiz relu par un expert · Quiz réussi 8/10 · Pas encore de quiz).
- **Voir** : `15 · 1a`.

### Ligne de quiz · en-tête de question · choix
- Ligne : la ligne de document avec le picto de l'origine, « n questions · durée », marque de relecture, dernier résultat (« 8/10 · 12 nov. » coche, ou « Jamais fait » gris). En-tête : progression à segments + « Question n sur N », énoncé 20/700, sans passage d'origine. Choix : ligne 48 px, cercle 24 bord 2 px ; bonne réponse cercle plein `succes` + coche + mot ; mon erreur cercle plein `danger` + croix + « Votre réponse ». Résultat 32/700, « Réussi » (7 sur 10 au moins) ou « À refaire » (gris, jamais rouge), jauge 6 px. Progression : une ligne par passage, date dans la colonne 62.
- **Voir** : `16 · 1a`, `16 · 2a`, `16 · 3a`, `16 · 4a`.

### Ligne de sommaire · ligne de transcription
- Sommaire : chapitre 17/600, 56 px, chevron plie/déplie (compte en gris replié) ; sous-partie 15/400, 48 px, retrait 16 px par niveau ; numéro de page 20/700 à droite, colonne alignée ; en cours : 700 + filet 3 px `filet-fort` à gauche. Transcription : colonne d'heure 62 (15/600 gris), texte 17 ; ligne courante filet gauche 3 px `accent` + 700 ; toucher place la vidéo.
- **Voir** : `04 · 04 Sommaire`, `15 · 2a`.

### Ligne de lexique · en-tête d'entrée · rail alphabétique
- Ligne : terme anglais italique 17/600 `titre`, traduction 17, première phrase de la définition 15 gris (résultats seulement). Correction : « Vous cherchiez peut-être : » + terme en italique 600. En-tête d'entrée : filtre d'origine 15/700 gris, terme 28/700 italique, traduction 20/600 `accent` ; lignes clé-valeur (Sigle si présent, sources) ; définition 17 ; « Entendu en salle » dans la citation ; termes liés en pilules ; Favori et Partager. Rail : colonne 24 px au bord droit, 26 lettres 15/700, interligne 20 ; lettre avec entrées `titre`, sans entrée gris, en cours filet 3 px à droite.
- **Voir** : `06 · 02 Recherche à la frappe`, `06 · 03 Liste alphabétique`, `06 · 04`.

### Ligne clé-valeur · ligne « avant → après »
- Clé 15/700 gris, valeur 17 (salle 20/700 `accent`), filet 1 px. Avant → après : ancienne valeur 15/600 gris barrée (trait 2 px) au-dessus, flèche « Déplacée » `information` devant la nouvelle 17/600, précision 15 gris dessous.
- **Voir** : `08` (fiche de session), `12 · 1c Fiche Mariam Traoré`.

### Ligne de file à décision (Validation)
- Qui · quoi · quand 15 gris · contenu 17/600 · contexte dans la citation de source · deux boutons demi-largeur (secondaire à gauche, jamais rouge ; principal à droite). Accepter = un geste, « Annuler » six secondes dans le message éphémère. Refuser ouvre une feuille à trois motifs + texte facultatif. « Suggéré par l'IA » : étiquette sans pictogramme devant des pilules décochables.
- **Voir** : `11 · 1a`, `11 · 1c`.

### En-tête de groupe
- Nom en capitales 15/700 gris + compteur à droite ; filet 3 px `filet-fort` dessous ; 16 px d'air au-dessus. Ordre fixe des résultats : Lexique · FAQ · Documents · Sessions de négociation. Sert aux canaux (Messages privés · Par thématique · Par promotion · IFDD · Réservé · Experts), à l'annuaire (lettres), aux membres.
- **Voir** : `02 · 09 Recherche`, `12 · 1a`.

### Séparateur de non-lus
- Filet 1 px de part et d'autre, carré jaune 10 px + « n nouveaux messages » 15/700, centré.
- **Voir** : `13 · 1a`.

### Bande des jours
- Une case par jour ayant au moins une session, défilement horizontal, 48 px de large min, 56 px de haut ; jour 13/600 (« jeu. ») au-dessus du numéro 20 ; actif 700 + filet 3 px `accent` dessous ; aujourd'hui non actif `accent` ; pressé `fond-2` ; passé gris.
- **Voir** : `07 · 1a`.

---

## Marques, étiquettes, avatars

### Marque d'état
- Pictogramme 20 px + mot 15/700 + couleur du rôle (`tokens.json › couleurs.etats`). Jamais la couleur seule ; jamais un fond coloré derrière le mot. Picto 18 pour Accès limité / Ouverte / Réservé / En attente / Hors connexion, 16 pour Synchronisé et Expert.
- **Voir** : `01 · A1`, `07 · 1a` (les six états de session), section 6 du système.

### Marque « Nouveau » · carré non-lu
- Carré jaune 10 px `attention-aplat` + mot noir ; le même signe que le non-lu et que le compteur (règle A1 : le jaune = ce qui vous concerne à l'instant).
- **Voir** : `03 · 01 Bibliothèque`, `12 · 1a`.

### Marque de rôle
- Étiquette bordée 1 px `filet-fort`, texte 13/700 `titre`, 1 × 6 px d'air, à côté du nom (« Coordonnatrice Adaptation »). « Expert » : bouclier coché 16 `succes` + mot 15/700. Plus jamais sur jaune (corrigé en 4 septdecies).
- **Voir** : `12 · 1b`, `12 · 1c`, `13 · 1a`.

### Étiquettes
- Bord 1 px `filet`, 15 px gris, pictogramme 18 (Source officielle · Traduction automatique · Se tient aussi au Pavillon · cercle de restitution) ; « Suggéré par l'IA » et « Synthèse automatique » sans pictogramme. Sous un titre traduit, l'anglais reste visible.
- **Voir** : `07 · 1a`, `11 · 1c`, `17 · 3a`.

### Avatar
- 40 px, rayon 24, initiales 15/700 ; plein `titre` pour le réseau, bord 2 px pour les autres (moi). Icône d'application provisoire : carré 40 px `titre` au monogramme « GN » 15/700 blanc (48 px dans la feuille de partage).
- **Voir** : `13 · 1a`, `12 · 1g Partage du téléphone`.

### Compteur · étape numérotée · indicateur d'étapes
- Compteur : 22 px `attention-aplat`, chiffre 13/700 noir, sur Notifications ou sur l'onglet Échanges. Étape : carré 32 px `titre`, chiffre 20/700 blanc. Indicateur : segments 4 px, faits `accent`, à venir `gris-pale`, toujours avec « Étape n sur N » / « Question n sur N ».
- **Voir** : `02 · 01 Installation`, `16 · 2a`.

---

## Commandes

### Boutons
- Principal `accent` #3C5404 plein, texte blanc 17/700 · secondaire bord 2 px `filet-fort`, texte `titre` · discret souligné (`accent`, soulignement décalé 4 px) · dangereux `danger` #C7101D, toujours après une confirmation. 48 px de haut, rayon 4, verbe d'action, pleine largeur en bas d'écran (16 px au-dessus de la barre), ou deux demi-largeurs (Annuler · Envoyer). Pressé : assombrissement ; désactivé : texte gris sur `gris-pale`.
- Favori / M'inscrire / Dans mon agenda : secondaire dont l'état actif = `titre` plein, texte blanc ; le mot ne change pas, l'état est dans l'aplat et dit à voix haute.
- **Voir** : `02 · 03b Connexion`, `03 · 05a Fiche`, `10 · 1b`.

### Champ de recherche · champ de formulaire · zone de texte
- Recherche : 48 px, bord 2 px `filet-fort`, loupe 24, texte 17, croix pour effacer ; repos · focus avec saisie · chargement. Formulaire : libellé toujours au-dessus (jamais seulement en texte indicatif), aide 15 gris ; repos · focus · erreur (bord `danger` + picto + phrase qui dit quoi faire) · désactivé. Zone de texte : 120 px, texte aligné en haut, compteur « n / 600 » (ou « n / 1 200 ») 15 gris à droite. Champ d'heure : 110 px.
- **Voir** : `02 · 03b Connexion`, `02 · 04b Code inconnu`, `05 · 03 Question à un expert`, `09 · 1a`.

### Case à cocher · interrupteur · cercle de choix
- Case 24 px, bord 2 px `filet-fort`, rayon 4 ; cochée `accent` plein + coche blanche 16. Interrupteur 52 × 32, rayon 24 ; actif `accent` avec coche dans le curseur 24 : jamais la couleur seule. Cercle 24 bord 2 px, choix unique ; coché `accent` plein + coche. Toute la ligne (56 px) est la cible.
- **Voir** : `02 · 06 Thématiques`, `02 · 11 Profil`, `12 · 1j`, `17 · 2a`.

### Filtres et sélecteur segmenté
- Onglets de filtre sous l'en-tête (16/600 ; actif 700 + filet 3 px). Pilule 40 px de haut dans une zone de 48, rayon 24 : repos bord 2 px `filet`, choisie `titre` plein + texte blanc 700, désactivée bord `gris-pale`. Pilule à choix : chevron bas 20 ; un choix fait : `titre` plein, « Type : Bulletin » ; deux ou plus : « Type : 2 » ; ouvre une feuille à cases + « Afficher n documents ». Pilules décochables (Suggéré par l'IA) : cochée `titre` plein + coche 16, décochée bordée 2 px ; un toucher bascule. Segmenté à choix unique (taille de lecture, thème).
- **Voir** : `03 · 02 Filtre Type`, `04 · 07 Réglages`, `12 · 1f Proposer depuis la conversation`, `12 · 1h`.

### Réactions
- Trois, fixes, sans emoji : D'accord (coche) · À retenir (marque-page) · Question (point d'interrogation). Pilule 36 px, bord 1 px, compte 15/700 ; la mienne `titre` plein ; zone tactile 48. « À retenir » range le message dans la liste « À retenir » du canal.
- **Voir** : `13 · 1a`, `13 · 1g`.

### Barre de saisie
- « Joindre » 48 bordé 2 px · champ 48 min (croît jusqu'à trois lignes), bord 2 px `filet` · « Envoyer » 48 `accent` plein ; filet 2 px `filet-fort` au-dessus ; jamais de barre d'onglets dessous. Assistant : sans « Joindre », compteur « n questions sur 20 aujourd'hui » + jauge 6 px dessous ; à 20 sur 20, champ et bouton désactivés.
- **Voir** : `13 · 1a`, `14 · 1a`, `12 · 1i`.

### Barre de lecture
- En bas, à portée de pouce. Repliée : 32 px + progression 6 px. Dépliée : mêmes mesures que la barre d'onglets (68, pictos 26, libellés 13) — Sommaire · Rechercher · Réglages · Marquer + « Page n sur N ». Toucher au centre bascule ; défiler replie.
- **Voir** : `04 · 01 Lecture`, `04 · 02 Barre dépliée`.

---

## Surfaces

### Feuille basse
- Voile `gris` à 60 % (100 % sur la maquette). Feuille blanche, filet 3 px `filet-fort` en haut, poignée 40 × 4 gris, titre 24/700 ou 20/700 avec sous-titre 15 gris, options de 56 px (phrases complètes, picto 24), « Annuler » secondaire. Sert aux menus (long-appui, « ⋯ »), aux motifs (Signaler, Refuser), aux filtres, aux réglages de lecture, à l'export, à « Proposer au classement ».
- **Voir** : `09 · 1a`, `12 · 1d Menu de la fiche`, `12 · 1f`, `13 · 1e`.

### Boîte de confirmation
- Boîte bord 2 px `filet-fort`, rayon 4, 16 px d'air ; question en titre 20/700, phrase 15, action principale à droite (Revenir · Envoyer). Obligatoire avant tout bouton dangereux (Retirer mon message, Quitter le canal, Bloquer).
- **Voir** : section 5 du système ; `13 · 1e`.

### Message éphémère
- Fond `titre` #233400, texte blanc 17, action en `vif-jaune` #FFD500 ; disparaît après 6 s ; jamais seul porteur d'une information (« Annuler » après acceptation).
- **Voir** : `09 · 1b`, `11 · 1a`.

### États vide et erreur
- Vide : picto 24, titre 20/700, ce qu'il n'y a pas et quand ça revient, une sortie en lien. Erreur : titre 20/700 `danger` avec triangle, ce qui a échoué, ce qui reste vrai (« Les sessions affichées datent de 11:35 »), deux sorties (Réessayer · Programme officiel).
- **Voir** : `03 · 03 Bibliothèque vide`, `07 · 1c`, `07 · 1d`.

### Verrou de module réservé
- Cadenas 40 `titre` centré, titre 20/700, phrase 17, liste bordée 1 px de ce qui est fermé (cadenas 16 gris, 48 px par ligne), phrase 15 gris de ce qui reste ouvert ; « Saisir mon code d'invitation » principal, « Continuer en visiteur » discret. Titre et sous-titre de l'écran restent visibles.
- **Voir** : `02 · 13 Réservé`, `12 · 1k Sans code`, `05 · 03c Question réservée`.

### Bandeau « Remplacé par… » · avertissement de source ancienne
- Pleine largeur, fond `attention-fond` #FFF7DA, picto Remplacé 20 `attention`, « Remplacé par » 15/700 `attention`, titre du document à jour en lien 17/600, date 15 gris, chevron ; tout le bandeau est la cible. Assistant : même bandeau, phrase datée, au-dessus de la carte de source.
- **Voir** : `03 · 05b Fiche remplacée`, `14 · 1f`.

### Ligne d'information bordée
- Boîte bord 1 px `filet`, picto Information 20 gris, texte 15 gris, 48 px min ; avec ou sans sortie en lien (« Écrire à l'IFDD »). Remplace la saisie sous une annonce ; rappelle une règle (annuaire, canal modéré).
- **Voir** : `13 · 1i`, `12 · 1c`, `12 · 1j`.

### Encart « Signalé par le réseau »
- Boîte bord 2 px `reseau` #732F85, sans fond ; en-tête 15/700 violet avec le losange, sans signature ; texte 17 ; marque d'état en seconde ligne s'il y en a. Sous l'en-tête de la fiche, au-dessus de la donnée officielle qui reste entière : il complète, ne remplace pas.
- **Voir** : `08` (fiche avec signalement).

### Citation de source · carte de source
- Filet gauche 3 px `filet-fort`, étiquette 15/700 gris (« Source », « Source officielle, lue à 11:35 »), passage 17 en italique ou entre guillemets, référence 15 gris (« Guide des négociations — CdP31, p. 59 »). Carte (assistant) : boîte 1 px autour ; note de correction repliée s'il y en a ; ligne cible 48 px 17/600 `accent` (« Ouvrir à la page 59 », « Lire de 12:30 à 14:10 ») + chevron.
- **Voir** : `05 · 02 Entrée de FAQ`, `14 · 1b`.

### Note de correction d'un expert
- Couleur et picto de l'état Dépassé (`danger`, triangle). Filet 3 px à gauche du passage, en marge. Repliée : une ligne 48 px. Dépliée : boîte bordée 1 px, signée (« Dr Koffi Mensah, expert IFDD — 10 novembre 2026 »). Le texte du guide n'est jamais modifié. Sur une vidéo : intervalle corrigé `sombre-rouge` #F08A90 sur la barre.
- **Voir** : `04 · 11a Note repliée`, `04 · 11b Note dépliée`, `15 · 3b`.

### Bulle de message
- Reçue `fond-2` #ECFFD3 · envoyée `titre` #233400 texte blanc ; texte 17, 10 × 12 px d'air ; nom 15/700 `titre` + marque de rôle ; heure 15 gris dessous (« Envoyé · 11:39 » avec picto Envoyer 20 `information`). Mention « @Prénom Nom » 600 `accent` ; message qui me mentionne : filet gauche 3 px `attention-aplat`. Pied de fil 15/600 `accent` : picto Répondre, « n réponses · dernier auteur, heure ». Retiré : cadre 1 px gris, 15/600 avec cercle barré ; auteur et heure restent. Hors connexion : « Sera envoyé au retour du réseau » (horloge `attention`).
- **Voir** : `13 · 1a`, `13 · 1c`, `13 · 1f`, `12 · 1e`.

### Tour d'assistant
- Question dans la bulle envoyée + « Envoyé · heure ». Réponse pleine largeur : « Assistant · heure » 15/700 gris, arc de chargement, texte 17, marque éventuelle (« Chiffre à confirmer par un expert », bouclier `attention`), bloc SOURCES et retours Oui / Non, « Dépassé ou faux ». Ni avatar ni prénom.
- **Voir** : `14 · 1a`, `14 · 1c`.

### Carte de notification (écran verrouillé, schéma)
- Fond `sombre-fond` #101704, heure 32/700 `sombre-texte`, carte blanche rayon 4 ; icône « GN » 40 ; « Guide Négo · 09:48 » 15 gris ; titre 17/600 commence par l'état ; texte finit par l'agenda d'origine.
- **Voir** : `09 · 4a`.

### Clavier · feuille de partage du téléphone (schémas)
- Jamais dessinés en détail. Clavier : fond `gris-pale`, touches blanches 40, lettres 15/600, touche d'action `accent` avec la loupe ; sert à mesurer ce qui reste visible. Feuille de partage : fond `gris-pale`, feuille blanche filet 3 px, nom du fichier 15/700, cibles 48 avec libellé 13, Guide Négo au monogramme « GN ».
- **Voir** : `06 · 02 Recherche à la frappe`, `12 · 1g Partage du téléphone`.

### Vignette vidéo
- Fond `sombre-fond`, 16 : 9, lecture 40 au centre, heure courante 20/700 `sombre-texte`, durée 15 `sombre-texte-2`, barre 6 px sur `gris` — position `sombre-vert` #B5D66A, intervalle cité `sombre-vert-filet` #6E9A2A, intervalle corrigé `sombre-rouge` #F08A90 (le rouge l'emporte). Hors connexion sans fichier : même cadre, picto Hors connexion `sombre-jaune`, phrase 15/600.
- **Voir** : `15 · 1b`, `15 · 3a`, `15 · 3c`.

### Jauge · barre de progression
- 6 px, `accent` sur `gris-pale`, toujours avec le nombre écrit (« 35 % », « 7 étapes sur 18 », « 7 Mo utilisés · 2,1 Go libres », « 7 questions sur 20 aujourd'hui »).
- **Voir** : `03 · 05c`, `05 · 04 Parcours Ma première COP`, `14 · 1g`.

### Bloc de lieu · carte de restitution · proposition de l'IA · rubrique de rédaction
- Bloc de lieu : bord 1 px, épingle 24 `picto`, nom 17/600, précision 15 gris, lien externe vers le plan ; en tête de la section Pavillon. Carte de restitution : avatar, nom · pays, date ; titre = la session ; rubriques repliées en une ligne (jamais « À remonter au ministère ») ; étiquette de cercle ; « À retenir » seule réaction. Proposition de l'IA : boîte 1 px, étiquette « Suggéré par l'IA », texte, « Revenir au mien » · « Garder ». Rubrique : libellé 15/700, zone de texte 120, compteur « n / 1 200 », deux boutons secondaires demi-largeur (désactivés hors connexion).
- **Voir** : `10 · 1c`, `17 · 1a`, `17 · 3a`.

### Chargement
- Arc : 24 px, trait 2,5, `accent`, 1 s linéaire, `aria-label="Chargement"`. Squelette : blocs `gris-pale` (sombre : `sombre-squelette`), hauteur 16, pulsation 1,6 s (opacité 1 → 0,55). Fixes sous « réduire les animations ».
- **Voir** : `03 · 05c`, `07 · 1a` (dernière ligne), `16 · 5a` (génération).

### Mention de pied (Échanges)
- « Échanges hébergés et modérés par l'IFDD » — 15 gris centré, sans pictogramme, 12 × 16 px d'air, au bas de toute liste des Échanges (canaux, annuaire, réglages d'un canal) ; pas sous une conversation, dont la saisie occupe le bas.
- **Voir** : `12 · 1a`, `12 · 1b`, `12 · 1j`.
