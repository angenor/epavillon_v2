# Prompts de maquette — Claude Design

> Vingt et un prompts, prévus pour **plusieurs sessions**. Ils couvrent **tous les écrans de Guide Négo**, y compris ceux qui ne seront pas prêts à la COP31. Chaque prompt tient seul : il dit quelle page créer, quels écrans y mettre, et ce qui prouve que c'est réussi.

## Mode d'emploi

1. **Un seul projet**, nommé « Guide Négo ». Y joindre dès le départ trois fichiers courts : [00-brief.md](../00-brief.md), [05-design.md](../05-design.md) et [lexique.md](lexique.md).
2. **Le prompt 1 est fait** (19/09) : la direction est choisie, voir plus bas. On reprend au **prompt 2**, après avoir collé le *bloc de contexte* s'il ne l'a pas déjà été.
3. **Toute nouvelle session** (nouvel onglet de discussion) : coller le *bloc de reprise*, puis le prompt du jour. Si les pièces jointes ne sont plus visibles, recoller aussi le bloc de contexte.
4. **Groupe ou page dédiée** : chaque prompt le précise. Un groupe pose plusieurs écrans côte à côte sur une même page ; une page dédiée porte un seul écran, avec tous ses états.
5. Le prompt 2 passe avant tous les autres ; ceux-ci se prennent ensuite dans l'ordre que vous voulez.
6. Après chaque page, passer le **prompt R** (revue) avant la suivante : une erreur de système coûte peu à la page 3, cher à la page 12. À la fin, le **prompt P** (passation).

## Couverture

| # | Page de maquette | Forme | Livraison |
|---|---|---|---|
| 1 | 00 — Directions | Dédiée | ✅ Fait le 19/09 |
| 2 | 01 — Système | Dédiée | — |
| 3 | 02 — Socle | Groupe | MVP |
| 4 | 03 — Documents | Groupe | MVP |
| 5 | 04 — Lecteur | Dédiée | MVP |
| 6 | 05 — Savoir | Groupe | MVP |
| 7 | 06 — Lexique | Dédiée | MVP |
| 8 | 07 — Sessions de négociation | Dédiée | MVP |
| 9 | 08 — Détail d'une session | Dédiée | MVP |
| 10 | 09 — Signaler et mon agenda | Groupe | MVP |
| 11 | 10 — Francophonie | Groupe | MVP |
| 12 | 11 — Validation | Groupe | MVP pour les signalements, le reste après |
| 13 | 12 — Échanges | Groupe | Après le MVP |
| 14 | 13 — Conversation | Dédiée | Après le MVP |
| 15 | 14 — Assistant | Dédiée | Après le MVP |
| 16 | 15 — Formations | Groupe | Après le MVP |
| 17 | 16 — Quiz | Groupe | Après le MVP |
| 18 | 17 — Restitutions | Groupe | Après le MVP, à confirmer sur le terrain |
| 19 | 18 — Back-office | Groupe, sur poste | Au fil des modules |
| R | Revue de cohérence | — | Après chaque page |
| P | Passation vers Claude Code | — | À la fin |

---

## Bloc de contexte — au début du projet

```
CONTEXTE PERMANENT — Guide Négo. Relis-le avant chaque maquette.

PRODUIT. Guide Négo est l'application mobile de l'IFDD (Organisation internationale
de la Francophonie) pour les négociatrices et négociateurs francophones des COP climat.
Elle donne, en français et sans connexion : les documents de négociation (guides,
résumés, notes techniques), une FAQ, un lexique anglais-français, les sessions de
négociation du jour avec leurs changements, les réunions de la Francophonie et les
activités du Pavillon. Ensuite : les échanges du réseau, un assistant IA qui cite ses
sources, les formations en vidéo, des quiz, des restitutions.

QUI S'EN SERT, ET OÙ. Des déléguées de pays francophones, d'Afrique surtout, souvent à
leur première COP. Debout dans un couloir, une main prise, pressées ; dehors en plein
soleil entre deux bâtiments ; ou tard le soir en salle. Téléphone Android de milieu de
gamme, 360 px de large, réseau saturé ou absent. Elles lisent des documents de
négociation : la densité ne les rebute pas, le flou si.

IDENTITÉ PROPRE. Guide Négo a sa propre identité visuelle. Ne reprends RIEN du site
ePavillon : ni son cyan, ni sa mise en page, ni ses composants, ni sa retenue. Seules les
couleurs viennent de la charte de l'IFDD. Je veux une application agréable et belle, qu'on
a plaisir à ouvrir dix fois par jour — et lisible en plein soleil.

DIRECTION RETENUE (19/09) — « Typographique ». Une seule police, Atkinson Hyperlegible
Next (400, 600, 700). Fond blanc dominant, vert très foncé #233400 comme encre des titres
et des filets forts, #3C5404 en accent. Quatre onglets avec pictogrammes — Accueil ·
Négociations · Francophonie · Ressources — et le bouton « Aa » du lexique, permanent, en
haut à droite de chaque écran. États d'une session : En cours vert #557607 · Déplacée
cyan #0C6792 · Annulée rouge #C7101D · Non annoncée violet #732F85 · Terminée gris
#565554 · Prévue noir. L'écran « Sessions de négociation » déjà produit, en clair et en
sombre, fait référence.

TROIS AGENDAS, JAMAIS CONFONDUS NI FUSIONNÉS, chacun sous son nom complet :
1. « Sessions de négociation » — réunions officielles de la CCNUCC (plénières, groupes
   de contact, consultations informelles), importées de la source officielle.
2. « Réunions de la Francophonie » — atelier préparatoire, concertation des
   négociateurs, concertation ministérielle.
3. « Pavillon de la Francophonie » — les activités du stand OIF/IFDD.
Le mot « Programme » seul est INTERDIT dans l'interface.

COULEURS — charte IFDD, valeurs exactes, n'en invente aucune.
Dominante vert foncé : #233400 (en-têtes, navigation), #3C5404 (action principale),
  #557607 (icônes, bordures fortes). Teinte de fond : #ECFFD3.
Seconde jaune foncé : #D3B011 (aplat, texte noir dessus), #816B06 (texte et icônes
  jaunes sur clair). Teinte de fond : #FFF7DA.
Couleurs de charte vives, en accent SUR VERT FONCÉ seulement : vert #8FBF2F, jaune #FFD500.
  Jamais en texte sur fond clair.
Neutres : noir #231F20 (texte), gris #565554 (texte secondaire), gris pâle #D9D8D6, blanc.
Pour les états, le reste de la charte est disponible : rouge #C7101D, violet #732F85,
  cyan foncé #0C6792. La correspondance retenue est dans « DIRECTION RETENUE » ; étends-la
  aux autres états. Un état = une icône + un mot + une couleur, jamais la couleur seule.
Tu peux ajuster la clarté d'une nuance si le contraste est tenu ; tu ne changes pas de teinte.

TYPOGRAPHIE. INTERDIT : Helvetica, NeueMaverick, Arial. Police à licence ouverte,
embarquable hors connexion, diacritiques français complets, chiffres tabulaires, grande
hauteur d'x, I l 1 et O 0 distincts, graisses 400 à 700, jamais moins de 400.

LISIBLE EN PLEIN SOLEIL — règles chiffrées :
- thème clair par défaut, fonds opaques ; texte courant à 7:1 au moins, secondaire à 4,5:1 ;
- corps 17 px, interligne 1,5 ; heure et salle d'une session : 20 px, graisse 600 au moins ;
- deux blocs voisins se distinguent sans effort : aplat, bordure franche ou espace, jamais
  une ombre légère seule ;
- jamais de texte posé sur une photo, un dégradé ou une surface translucide ;
- cibles tactiles de 48 px ; action principale en bas, à portée de pouce ;
- thème sombre pour les séances de nuit : pas une inversion, vert et jaune désaturés.
Dans la direction retenue, la beauté vient de la typographie, du rythme et de la justesse
des espaces — pas d'ornement.

TON. Chaleureux et sûr de lui : l'outil d'une déléguée qui représente son pays. Vouvoiement,
phrases courtes, pas de tournure de réclame. Des pictogrammes, pas d'emoji. Toute heure
porte son fuseau : « 14:30 — 16:00, heure d'Antalya ».
Aucun faux texte : tout contenu vient du jeu de données ci-dessous.

FORMAT. Cadre mobile 360 × 800, vérifié à 390 × 844. Barre d'onglets basse. Chaque écran
est titré, et montre ses états : chargement, vide, erreur, hors connexion, accès réservé.

JEU DE DONNÉES — le même sur toutes les pages, pour que les sessions se raccordent.
Personne : Aïssatou Diallo, Sénégal, première COP ; suit « Adaptation » et « Genre ».
Autres personnes : Mariam Traoré (Mali, coordonnatrice Adaptation), Dr Koffi Mensah
  (expert de l'IFDD), Clarisse Nguema (Gabon, Article 6).
Événement : COP31, Antalya EXPO Center, 9–20 novembre 2026. Jour montré : jeudi 12 novembre.
Instant de référence de toutes les pages : jeudi 12 novembre, 11:40, heure d'Antalya. Tout
  horodatage lui est antérieur ; les états des sessions en découlent.
Thématiques : Adaptation · Atténuation · Finance · Pertes et préjudices · Article 6 ·
  Transparence · Genre · Transition juste · Agriculture · Technologie.
Sessions de négociation du 12 novembre — type · heures · salle · accès · état · thématique :
  Coordination de groupe · 08:00–09:30 · Salle 12 · Accès limité · Terminée · Mon groupe
        Coordination du Groupe africain
  Consultations informelles · 10:00–12:00 · Salle 7 · Ouverte · En cours · Adaptation
        « Informal consultations on the global goal on adaptation » / Consultations
        informelles sur l'objectif mondial d'adaptation
  Groupe de contact · 11:30–13:00 → 15:00–16:30 · Salle 3 → Salle 9 · Ouverte · Déplacée · Genre
        « Contact group on the gender action plan » / Groupe de contact sur le plan
        d'action genre
  Coordination de groupe · 13:00–14:30 · Salle 5 · Accès limité · Prévue · Mon groupe
        Coordination des PMA
  Consultations informelles · 15:00–16:30 · Salle 4 · Ouverte · Annulée · Transition juste
        « Informal consultations on the just transition work programme » — sans titre français
  Aparté · 16:30, fin inconnue · Couloir du bâtiment B · Non annoncée, signalée par le
        réseau, validée à 11:12 · Adaptation
        Aparté sur les indicateurs d'adaptation
  Les coordinations de groupe n'ont pas de thématique : elles portent « Mon groupe » et
  passent toujours le filtre « Mes thématiques ».
  Une septième session, hors de ses thématiques, qu'Aïssatou a ajoutée à son agenda
  depuis « Toutes » : Consultations informelles · 11:00–12:30 · Salle 2 · Ouverte ·
  Prévue · Finance — « Informal consultations on matters relating to the Adaptation
  Fund » / Consultations informelles sur le Fonds pour l'adaptation. Elle chevauche le GGA.
  Source : « Source officielle — lu à 11:35 ».
Réunions de la Francophonie : Atelier préparatoire (dimanche 8 novembre) · Concertation
  francophone des négociatrices et négociateurs (mardi 17 novembre, 18:30) ·
  Concertation ministérielle.
Pavillon de la Francophonie, 12 novembre : 11:00 « Financer l'adaptation en Afrique de
  l'Ouest » · 14:00 « Femmes et négociations climat : dix ans de formation ».
Documents (cinq) : Guide des négociations — CdP31 (92 p., téléchargé, 7 Mo) · Résumé pour
  les décideurs — CdP31 (40 p.) · Note technique — Bilan de la CdP30 (remplacée) ·
  Bulletin des négociations de la Terre, 11 novembre (lien externe, IISD) · « Au nom de
  ma délégation », IISD. Mes téléchargements : 1 document · 7 Mo ; aucune vidéo avant
  les Formations.
Formations : « Module 4 — Comprendre un texte de négociation » (vidéo, 48 min) ·
  « Module 9 — L'adaptation dans l'Accord de Paris » (vidéo, 1 h 12).
Lexique : contact group — groupe de contact · bracketed text — texte entre crochets ·
  informal informals · huddle — aparté · non-paper · stocktake — bilan · NDC — CDN ·
  loss and damage — pertes et préjudices.
FAQ : « Quelle différence entre un groupe de contact et des consultations informelles ? » ·
  « Qui coordonne mon groupe sur l'adaptation ? » · « Comment prendre la parole au nom de
  ma délégation ? » · « Où retirer mon badge ? ».
```

## Bloc de reprise — à coller au début de chaque nouvelle session

```
Nous reprenons la maquette de Guide Négo dans ce projet. Avant de dessiner :
1. Relis les pièces jointes, la page « 01 — Système » et la page « Journal de maquette ».
2. Résume-moi en cinq lignes la direction retenue (« Typographique », Atkinson
   Hyperlegible Next, quatre onglets et bouton « Aa » du lexique), et les trois agendas à
   ne pas confondre.
3. Réutilise les composants et les jetons existants. N'invente ni couleur, ni taille,
   ni composant : s'il en manque un, propose-le d'abord, et ajoute-le au système
   avant de t'en servir.
4. Garde le même jeu de données que sur les pages existantes.
Attends mon feu vert après le résumé.
```

---

## 1 — Trois directions artistiques · page dédiée « 00 — Directions » — ✅ fait le 19/09

Claude Design a posé la question de lui-même, avant tout prompt. **Ne pas relancer ce prompt.**

Direction retenue : **« Typographique » (1b), avec la barre à quatre onglets et pictogrammes (2a)**. L'écran de référence, en clair et en sombre, est gardé dans [ecrans/00-direction-retenue.html](ecrans/00-direction-retenue.html) ; la décision, dans [ADR-018](../adr/018-direction-typographique-quatre-onglets.md).

| Acquis | Valeur |
|---|---|
| Police | Atkinson Hyperlegible Next, seule — 400, 600, 700 ; chiffres tabulaires |
| Couleur | Blanc dominant ; `#233400` en encre des titres et des filets forts ; `#3C5404` en accent |
| Navigation | Accueil · Négociations · Francophonie · Ressources, avec pictogrammes ; bouton « Aa » du lexique, permanent, en haut à droite |
| États d'une session | En cours `#557607` · Déplacée `#0C6792` · Annulée `#C7101D` · Non annoncée `#732F85` · Terminée `#565554` · Prévue noir |
| Thème sombre | Fond `#101704`, texte `#EEF2E4`, accent `#B5D66A` |

## 2 — Système de design · page dédiée « 01 — Système »

```
Je retiens la direction « Typographique » (1b) avec la barre à quatre onglets et
pictogrammes (2a). L'écran « Sessions de négociation » que tu as produit, en clair et en
sombre, fait référence. Construis maintenant le système de design de Guide Négo sur une
page « 01 — Système ». Tout ce qui suivra en sortira.

CE QUI EST ACQUIS — ne le remets pas en cause :
- une seule police, Atkinson Hyperlegible Next (400, 600, 700), chiffres tabulaires ;
- fond blanc dominant, #233400 en encre des titres et des filets forts, #3C5404 en accent ;
- quatre onglets avec pictogrammes — Accueil · Négociations · Francophonie · Ressources —
  et le bouton « Aa » du lexique, permanent, en haut à droite de chaque écran ;
- l'anatomie d'une ligne de session : heure, titre français, titre anglais, salle, accès, état ;
- les couleurs d'état : En cours vert #557607 · Déplacée cyan #0C6792 · Annulée rouge
  #C7101D · Non annoncée violet #732F85 · Terminée gris #565554 · Prévue noir ;
- le thème sombre sur fond #101704.

CE QUE JE TE DEMANDE DE REPRENDRE en construisant le système :
A. Le jaune foncé a presque disparu, alors que c'est la seconde couleur de l'identité.
   Donne-lui UN rôle net et constant — par exemple « ce qui se passe maintenant », la
   nouveauté, ou le surlignage d'un résultat de recherche —, en aplat #D3B011 avec texte
   noir, jamais en texte sur blanc. Montre deux options, je choisis.
B. « En cours » est la ligne la plus importante de l'écran, et elle se fond dans le vert
   général. Fais-la ressortir sans casser la sobriété de la direction.
C. Les filets de liste (#D9D8D6 sur blanc, #2E3F0E sur #101704) sont sous 2:1 : ils
   disparaîtront au soleil. Propose une séparation qui tient — filet plus foncé, espace,
   fond alterné — et montre-la avec la vignette « test plein soleil ».
D. Les signes d'état (✓ ● → ○ ✕ ◆) sont des caractères : remplace-les par de vrais
   pictogrammes au trait de 2 px, de la même famille que ceux des onglets.
E. Tailles : rien sous 15 px hors libellés d'onglets. L'italique du titre anglais à 14 px
   est fragile au soleil : propose mieux.
F. La police vient de Google Fonts dans la maquette ; dans l'application elle sera
   embarquée. Note-le, avec sa licence.

LE SYSTÈME COMPLET :
1. Couleurs — deux niveaux. Les couleurs de la charte gardent leurs valeurs ; des jetons
   nommés par rôle (fond, texte, texte secondaire, titre, accent, filet, filet fort,
   attention, succès, danger, information, et chaque état) les référencent, en thème clair
   ET en thème sombre. Tableau des contrastes mesurés ; signale toute paire sous 7:1 pour
   du texte courant, sous 4,5:1 pour du texte secondaire.
2. Typographie — l'échelle (12 à 32 px, corps à 17), les graisses, les chiffres
   tabulaires, un exemple avec « É À Ç œ « » ».
3. Mesures — grille de 4 px, marges d'écran, rayons, épaisseurs de filet, hauteur des
   cibles (48 px), zones sûres haute et basse.
4. Pictogrammes — une famille au trait de 2 px, et la liste de ceux du produit.
5. Composants, chacun avec ses états (repos, pressé, focus, désactivé, chargement) :
   barre d'onglets, en-tête d'écran avec bouton « Aa », champ de recherche, bouton
   (principal, secondaire, discret, dangereux), filtre, marque d'état, ligne de session,
   ligne de document, ligne d'activité, bandeau de connexion, étiquette de source
   (« Source officielle — lu à 11:35 »), étiquette « Traduction automatique », verrou de
   module réservé, feuille basse, boîte de confirmation, message éphémère, squelette de
   chargement, état vide, état d'erreur, champ de formulaire, sélecteur segmenté, barre
   de progression de téléchargement, bulle de message, citation de source, ligne de
   notification, avatar et marque de rôle.
6. Les états de tous les objets, avec la même règle que ceux d'une session : signalement
   (Envoyé, Validé, Non retenu), document (À jour, Remplacé, Dépassé, Téléchargé), quiz
   (Relu par un expert, Non relu), connexion (Hors connexion, Synchronisé).
7. Mouvement — trois durées, deux courbes, et la règle : aucune animation n'est
   nécessaire pour comprendre ; tout respecte « réduire les animations ».

Crée aussi une page « Journal de maquette » : une ligne par décision prise (direction,
police, navigation, rôle du jaune), que tu compléteras à la fin de chaque étape.
```

## 3 — Socle · groupe « 02 — Socle »

```
Page « 02 — Socle », écrans côte à côte, dans l'ordre du parcours :
1. Installation — comment ajouter Guide Négo à l'écran d'accueil du téléphone.
2. Ouverture sans compte — ce qu'on peut lire tout de suite (documents, FAQ, lexique,
   sessions), et ce qui demande un compte.
3. Création de compte et connexion ; mot de passe oublié.
4. Saisie du code d'invitation — reçu dans le groupe WhatsApp ; états : code juste,
   code inconnu, code révoqué.
5. Demande en attente — quand l'admission passe par un administrateur.
6. Choix des thématiques suivies — une ou plusieurs.
7. Accueil « Ma journée » pour Aïssatou, jeudi 12 novembre : sa prochaine session de
   négociation, les changements du jour (une déplacée, une annulée), ce que ses trois
   agendas proposent aujourd'hui — CHAQUE ligne porte son origine —, ses documents
   récents, un accès direct au lexique.
8. Le même accueil hors connexion : « Hors connexion — lu à 11:35 ».
9. Recherche globale — un seul champ ; résultats groupés par origine, le lexique en
   premier, puis la FAQ, les documents, les sessions de négociation.
10. Centre de notifications — changements de sessions, réponses d'experts, nouveaux
    documents ; lu et non lu.
11. Profil et réglages : mes thématiques, mes téléchargements et la place occupée, thème
    clair ou sombre, notifications par thématique, mon accès, déconnexion.
12. À propos, confidentialité et consentements.
13. Un module réservé vu par une personne sans code : « Réservé aux négociatrices et
    négociateurs — Saisir mon code d'invitation ».

La barre d'onglets est décidée : Accueil · Négociations · Francophonie · Ressources, et le
bouton « Aa » du lexique sur chaque écran. Montre seulement ce qui se passe le jour où un
cinquième onglet, « Échanges », arrive. Note-le au journal.
```

## 4 — Documents · groupe « 03 — Documents »

```
Page « 03 — Documents », trois écrans :
1. Bibliothèque — recherche, filtres par type (guide, résumé, note technique, bulletin),
   par thématique et par COP ; chaque ligne montre type, titre, pages, poids, et ses
   marques : « Téléchargé », « Nouveau », « Remplacé ».
2. Fiche d'un document — résumé, thématiques (zéro à plusieurs), version et date,
   éditeur, boutons « Télécharger pour lire sans réseau », « Favori », « Partager ».
   Variante : la Note technique CdP30, avec le bandeau « Remplacé par… » qui mène au
   document à jour. Variante : téléchargement en cours, puis terminé. Variante : un
   document qui est un lien externe et non un fichier.
3. Mes documents — favoris et téléchargés, place occupée sur le téléphone, tout retirer.
États : bibliothèque vide après filtre, hors connexion (seuls les téléchargés s'ouvrent,
les autres le disent), document réservé vu sans code.
```

## 5 — Lecteur · page dédiée « 04 — Lecteur »

Joindre [donnees-lecteur.md](donnees-lecteur.md) : le sommaire et un passage réels du guide.

```
Page « 04 — Lecteur » : la lecture du Guide des négociations, 92 pages, sur 360 px.
C'est l'écran où l'on passe le plus de temps : confort de lecture longue avant tout.
Montre : la page en lecture, la barre d'outils repliée et dépliée, le sommaire, la
recherche dans le document avec ses résultats, la progression (« page 34 sur 92 »), le
retour à la dernière page lue, le passage en thème sombre, l'agrandissement du texte.
Un terme anglais du document, touché, ouvre son entrée du lexique dans une feuille basse.
Une page portant la note de correction d'un expert la montre en marge.
Montre aussi l'état « document non téléchargé et pas de réseau ».
Données : dernière page lue 59, chapitre « 3.6 Adaptation » ; sommaire et passage réels
dans le fichier joint donnees-lecteur.md — n'invente aucun texte de guide.
```

## 6 — Savoir · groupe « 05 — Savoir »

Joindre [donnees-savoir.md](donnees-savoir.md) : l'entrée de FAQ de référence avec ses vraies sources, les rubriques, et les 18 étapes du parcours.

```
Page « 05 — Savoir », quatre écrans :
1. FAQ — recherche, rubriques (« Ma première COP », « Le processus », « Les groupes de
   négociation », « Sur place »), questions les plus lues.
2. Entrée de FAQ — la réponse, « Vérifié le 12 novembre 2026 », ses sources, « Cette
   réponse vous a-t-elle aidée ? », le bouton « Dépassé ou faux », des questions liées.
3. Poser une question à un expert — réservé ; dit que la réponse pourra rejoindre la FAQ.
4. Parcours « Ma première COP » — une liste à cocher par étapes : avant de partir, le
   premier jour, en salle, le soir. Progression visible, utilisable sans réseau.
Données : celles de donnees-savoir.md — n'invente ni question, ni réponse, ni étape.
```

## 7 — Lexique · page dédiée « 06 — Lexique »

Joindre [donnees-lexique.md](donnees-lexique.md) : dix-huit entrées avec traduction, définition, exemple et source.

```
Page « 06 — Lexique ». L'usage : en salle, quelqu'un dit « bracketed text » ; il faut la
réponse en cinq secondes, d'une main, sans réseau. On y arrive par le bouton « Aa »,
présent sur chaque écran.
Montre : l'ouverture directe sur le champ de recherche, clavier sorti ; la recherche à la
frappe, qui tolère les fautes ; la liste alphabétique ; l'entrée — terme anglais en
italique, traduction, définition en français en deux phrases, un exemple entendu en
salle, le sigle développé, les termes liés, « Favori » ; mes termes favoris ; l'état
« aucun résultat », qui propose de soumettre le terme.
Données : celles de donnees-lexique.md — n'invente ni terme ni définition.
```

## 8 — Sessions du jour · page dédiée « 07 — Sessions de négociation »

```
Page « 07 — Sessions de négociation », l'écran le plus consulté. Jeudi 12 novembre.
Montre : le sélecteur de jour ; le filtre « Mes thématiques / Toutes » ; la liste
chronologique avec les six sessions du jeu de données et leurs six états. Chaque ligne :
heure de début et de fin en grands chiffres tabulaires, salle, titre français marqué
« Traduction automatique » et titre anglais dessous, type de réunion, « Ouverte » ou
« Accès limité », thématique, état. Une session déplacée montre l'ancienne heure barrée
et la nouvelle. La session non annoncée porte « Signalée par le réseau — validée à 11:12 ».
En tête, l'étiquette « Source officielle — lu à 11:35, heure d'Antalya ».
Trois variantes de l'écran entier :
- hors connexion : la dernière lecture, datée ;
- lecture impossible : on le DIT, et on renvoie au programme officiel de la CCNUCC —
  jamais une donnée périmée présentée comme fraîche ;
- aucune session pour mes thématiques aujourd'hui.
Rappel : cet écran ne montre NI les réunions de la Francophonie NI le Pavillon.
```

## 9 — Détail d'une session · page dédiée « 08 — Détail d'une session »

```
Page « 08 — Détail d'une session » : le groupe de contact sur le plan d'action genre,
déplacé à 15:00 en salle 9.
Montre : titre français et anglais ; l'état « Déplacée » et ce qui a changé ; heure avec
fuseau ; salle ; type de réunion, expliqué par un lien vers le lexique ; point de l'ordre
du jour ; « Ouverte » ou « Accès limité » ; documents liés ; source et heure de lecture ;
lien vers l'original ; « Ajouter à mon agenda », « Me rappeler 15 minutes avant »,
« Signaler un changement ».
Un signalement validé se pose PAR-DESSUS la donnée officielle, dans un encart distinct
(« Signalé par le réseau, validé par l'IFDD à 11:12 ») : il ne la remplace jamais.
Variantes : déplacée avec encart du réseau ; annulée par la source officielle ; en cours,
déjà dans mon agenda ; à accès limité (« Mon groupe », sans titre anglais) ; et le cas qui
prouve la règle : annulée par signalement validé alors que la source officielle dit
encore « Prévue ».
Données : point de l'ordre du jour « OSMOE 65 — Genre et changements climatiques »
(numéro à remplacer) ; documents liés : « Décision 7/CP.30 — Plan d'action genre de
Belém » (français) et « Note informelle des co-facilitateurs, 11 novembre » (anglais) ;
encart du réseau, sans nom d'auteur : « L'écran de la salle 3 affiche encore 11:30 :
c'est bien 15:00 en salle 9, bâtiment B. » — validé par l'IFDD à 11:12.
```

## 10 — Signaler, mon agenda · groupe « 09 — Signaler et mon agenda »

```
Page « 09 — Signaler et mon agenda », quatre écrans :
1. Signaler un changement — en trois gestes au plus : Annulée, Déplacée, Salle changée,
   Réunion non annoncée ; précision facultative ; envoi. Dit clairement : « Un
   administrateur vérifie avant d'afficher. »
2. Mes signalements — Envoyé, Validé, Non retenu.
3. Mon agenda — les sessions que je suis ; deux sessions qui se chevauchent sont
   SIGNALÉES, jamais empêchées.
4. La notification d'un changement, sur l'écran verrouillé et dans l'application.
Deux entrées de signalement, pas une : depuis la fiche d'une session — Annulée ·
Déplacée · Salle changée · Autre — et depuis la liste du jour, « Signaler une réunion non
annoncée » (quoi, où, quand, thématique). Trois gestes, aucune boîte de confirmation.
Données — mes signalements : « Salle 3 : l'écran affiche encore 11:30 » Validé 11:15 ·
« Aparté sur les indicateurs d'adaptation » (non annoncée) Validé 11:12 · « Coordination
des PMA annulée » Non retenu, motif « La source officielle la maintient » · « Coordination
du Groupe africain de demain avancée à 7:30 » Envoyé 11:20. Mon agenda : GGA 10:00–12:00 ·
Fonds pour l'adaptation 11:00–12:30 (chevauche le GGA, marque sur les deux lignes) ·
Coordination des PMA 13:00–14:30 · Groupe de contact genre 15:00–16:30 · Transition juste
15:00–16:30 annulée, sans marque de chevauchement · Aparté 16:30. Notification : « Groupe
de contact sur le plan d'action genre — déplacé à 15:00, salle 9 · 09:48 ».
```

## 11 — Francophonie · groupe « 10 — Francophonie »

```
Page « 10 — Francophonie » : DEUX sections nettement distinctes, jamais une liste mêlée.
Section « Réunions de la Francophonie » : la liste (atelier préparatoire, concertation
des négociatrices et négociateurs, concertation ministérielle) ; le détail d'une réunion —
lieu, heure avec fuseau, lien de visioconférence, inscription ; la mention « Se tient
aussi au Pavillon » quand c'est le cas.
Section « Pavillon de la Francophonie » : où est le stand ; les activités du jour et à
venir ; les activités passées avec leur rediffusion ; le détail d'une activité —
organisateurs, intervenants, inscription, état « Inscrite ».
Montre comment on passe de l'une à l'autre sans jamais douter de celle où l'on se trouve.
États d'inscription : Inscrite · Liste d'attente · Complet (propose « Rejoindre la liste
d'attente ») · Rediffusion, avec la durée.
Données — Réunions : Atelier préparatoire, dimanche 8 novembre, 09:00–17:00, hôtel (à
remplacer), Terminé · Concertation des négociatrices et négociateurs, mardi 17 novembre,
18:30–20:00, au Pavillon (« Se tient aussi au Pavillon »), visioconférence, Inscrite ·
Concertation ministérielle, mercredi 18 novembre, 12:30–14:00, salle à confirmer, Accès
limité (ministres et chefs de délégation). Pavillon, zone bleue, hall 4, stand B12 (à
remplacer) — aujourd'hui à 11:40 : 09:30 « Adaptation et sécurité alimentaire au Sahel »
Terminée, rediffusion 52 min · 11:00 « Financer l'adaptation en Afrique de l'Ouest » En
cours, Inscrite · 14:00 « Femmes et négociations climat : dix ans de formation » Prévue,
Inscrite · 16:00 « Les CDN 3.0 des pays francophones : où en est-on ? » Complet, liste
d'attente. Hier, 11 novembre : 10:00 « Finance climat : de Bakou à Belém » rediffusion
58 min · 15:30 « Jeunes négociateurs francophones : retour de la COP30 » rediffusion
47 min. Tous ces titres sont à remplacer par le programme réel de la COP31.
```

## 12 — Validation · groupe « 11 — Validation »

```
Page « 11 — Validation » : ce que font l'administrateur et l'expert depuis leur
téléphone, entre deux tâches au stand. Chaque décision tient en UN geste.
Seul l'écran 1 appartient au MVP ; dessine les autres pour la cohérence du système.
1. File des signalements — qui, quoi, quand, et ce que dit la source officielle à cet
   instant ; « Valider », « Ne pas retenir ».
2. Demandes d'accès — quand l'admission passe par un administrateur : qui, quel pays,
   quel code ; « Admettre », « Refuser ».
3. Arrivée des documents — ce qui a été proposé ou capté, avec le classement suggéré par
   l'IA (type, thématiques) ; « Promouvoir en référence », « Classer sans suite ».
4. File des questions aux experts — répondre ; « Ajouter à la FAQ ».
5. Relecture d'un quiz proposé par l'IA — chaque question avec son passage source ;
   corriger, valider, rejeter.
6. Retours « Dépassé ou faux » — marquer une source « Dépassée », poser une note de
   correction sur une page ou sur un intervalle de vidéo.
Règles : un geste pour accepter, message éphémère avec « Annuler » pendant six secondes ;
refuser demande un motif court (La source officielle la maintient · Déjà pris en compte ·
Pas assez précis), que la personne qui a signalé verra ; un quiz se publie quand toutes
ses questions sont validées. Codes d'invitation : « RESEAU-31 » (réseau des
négociatrices) et « NEGO-31 » (négociateurs).
Données : les quatre signalements de la page 09 vus côté administrateur, le dernier à
traiter, avec « Source officielle, lue à 11:35 : Prévue, salle 5 » en contexte ·
demande d'accès : Clarisse Nguema, Gabon, entrée avec NEGO-31, en attente · document
capté : « Bulletin des négociations de la Terre, 12 novembre », suggestion de l'IA :
Bulletin · Adaptation, Finance · questions : « Qui coordonne mon groupe sur
l'adaptation ? » répondue par le Dr Koffi Mensah à 09:15, et « Comment obtenir la version
française d'un projet de texte ? » à traiter · quiz : le quiz personnel d'Aïssatou « GGA — questions reformulées », dix
questions, passages p. 59–61, proposé à la relecture à 11:20, trois questions montrées ·
retours : la note de correction de la page 04 (p. 59 : les indicateurs ont été
adoptés à Belém, le passage décrit l'enjeu d'avant) et le même intervalle 12:30–14:10
du Module 9, qui dit la même chose.
```

## 13 — Échanges · groupe « 12 — Échanges » — après le MVP

```
Page « 12 — Échanges », cinq écrans. L'onglet « Échanges » entre dans la barre : applique
la décision du journal.
1. Liste des canaux — par thématique, par promotion, annonces de l'IFDD, le canal
   réservé du réseau des négociatrices ; non-lus ; messages privés en tête.
2. Annuaire du réseau — chercher par pays, thématique, rôle (coordonnatrice, experte) ;
   la fiche d'une personne ; « Écrire » ; le réglage « Apparaître dans l'annuaire ».
3. Questions aux experts — mes questions et leur état : En attente, Répondue, Ajoutée à
   la FAQ ; la réponse du Dr Koffi Mensah.
4. Proposer un document — depuis une conversation, ou depuis le partage du téléphone
   (un PDF reçu sur WhatsApp, partagé vers Guide Négo) : deux gestes, puis « Proposé —
   en attente de classement ».
5. Réglages d'un canal — notifications (tout, mentions, rien), membres, quitter.
Une mention sobre et permanente : « Échanges hébergés et modérés par l'IFDD ».
```

## 14 — Conversation · page dédiée « 13 — Conversation » — après le MVP

```
Page « 13 — Conversation » : le canal « Adaptation » un jour de COP.
Montre : messages, réponses en fil, mentions, pièce jointe, document proposé au
classement, réactions, message retiré par la modération, « Signaler ce message », saisie,
envoi hors connexion (« Sera envoyé au retour du réseau »), séparateur de non-lus.
Variante : une conversation privée entre Aïssatou et Mariam Traoré.
Variante : une annonce de l'IFDD, à laquelle on ne répond pas.
Compare honnêtement à WhatsApp : ce que l'on retrouve, ce que l'on gagne (fils par
thématique, documents classés, experts), ce que l'on perd.
Règles : le jaune signale ce qui vous concerne à l'instant — la session en cours, le
non-lu, la mention — et rien d'autre. Trois réactions fixes, sans emoji : D'accord ·
À retenir · Question ; « À retenir » range le message dans une liste « À retenir » du
canal. Pilule de réaction : 36 px visibles, 48 px de zone tactile. Sur son propre message,
le menu offre « Retirer mon message ». Les messages restent factuels et logistiques —
ce qui s'est dit en salle ouverte, où, quand — jamais la stratégie d'un groupe : le
canal réunit des délégations de groupes opposés.
Données — canal « Adaptation », 12 novembre, tout avant 11:40 :
  09:52 Mariam Traoré (coordonnatrice) : « Le groupe de contact genre passe à 15:00,
        salle 9. L'écran de la salle 3 n'est pas à jour. » — 3 réponses, dernière 09:58
  10:20 Aïssatou Diallo, depuis la salle 7 : « GGA : les co-facilitateurs distribuent une
        nouvelle itération. Le financement reste entre crochets. » — À retenir ×4
  10:41 Clarisse Nguema : « Quelqu'un a le texte ? » — Question ×2
  10:43 Aïssatou : pièce jointe « Nouvelle itération GGA — 12 nov., 10:05 » — Proposé
        au classement — en attente d'un expert
  11:05 Dr Koffi Mensah (Expert), en réponse à @Clarisse Nguema : « Bracketed text :
        rien n'est acquis tant que les crochets restent. Entrée du lexique jointe. » —
        D'accord ×6
  11:12 Mariam Traoré : « Aparté sur les indicateurs à 16:30, couloir du bâtiment B. »
  11:30 message retiré par la modération
  — 4 nouveaux messages —
  11:34 Bulletin des négociations de la Terre, 12 novembre — proposé au classement,
        classé : Adaptation, Finance
  11:38 dernier message, non lu
Privé Aïssatou ↔ Mariam : « Je peux vous rejoindre à la coordination de demain ? » — « Oui,
7:30, salle 12. Venez avec le texte entre crochets. »
Annonce IFDD : « Concertation des négociatrices et négociateurs, mardi 17 novembre à
18:30, au Pavillon. Inscription dans Réunions de la Francophonie. »
```

## 15 — Assistant · page dédiée « 14 — Assistant » — après le MVP

```
Page « 14 — Assistant ». L'assistant ne répond que depuis des sources validées et cite
toujours. Montre :
- une question d'Aïssatou et sa réponse, avec ses « Sources » : un document et sa page,
  une vidéo de formation et son intervalle (« 12:30 — 14:10 ») ;
- une source touchée, qui s'ouvre à la bonne page ou à la bonne minute ;
- une réponse appuyée sur une source ancienne, avec son avertissement daté ;
- une source portant une note de correction d'un expert ;
- « Je ne sais pas » — et la proposition de poser la question à un expert ;
- le bouton « Dépassé ou faux » et ce qui suit ;
- le refus sobre de conseiller la position d'un pays ;
- « Résumer en français » un document reçu en anglais, marqué « Aide à la lecture — le
  texte anglais fait foi » ;
- l'historique de mes conversations ; la limite de questions du jour ; l'état hors connexion.
Règles : la conversation se tient à 11:40, l'instant de référence. Résumer un document
que je lui donne n'est pas répondre depuis le corpus : l'étiquette « Aide à la lecture »
le dit. Une note de correction ne rend pas la source « Dépassée » : la carte montre la
source valide et sa note repliée, comme le Lecteur.
Données : 1a « Combien d'indicateurs du GGA ont été adoptés à la CdP30 ? » → « La CMA a
adopté à Belém une liste de 59 indicateurs, retenus parmi les 100 proposés par les
experts » — chiffre à confirmer par un expert avant publication ; sources : Guide CdP31
p. 59 avec sa note de correction · Module 9, 12:30 — 14:10, même note. 1d « Qu'est-ce
qu'une plénière informelle de bilan ? » → réponse depuis « Au nom de ma délégation »,
IISD, p. 39, avec le bandeau « Source de 2024, deuxième édition. Rien de plus récent ne
traite ce point dans le corpus. » 1e « À quelle heure ouvre la salle 7 demain ? » → « Je
ne sais pas », « Poser la question à un expert ». 1g « Quelle position le Sénégal
devrait-il défendre sur le GGA ? » → refus sobre, renvoi au lexique et à la coordonnatrice.
1h résumé de « Nouvelle itération GGA, 12 novembre (EN) ». Quota : 7 questions sur 20 ;
à 20, « Revient à minuit, heure d'Antalya ».
```

## 16 — Formations · groupe « 15 — Formations » — après le MVP

```
Page « 15 — Formations », trois écrans :
1. Les modules de formation — seize modules, filtrés par thématique ; durée, progression,
   quiz associé.
2. Fiche d'un module — la vidéo, sa durée, ses supports à télécharger, son quiz.
3. Lecteur vidéo — la transcription horodatée qui défile avec la vidéo ; la recherche
   dans la transcription ; l'ouverture directe à la minute citée par l'assistant
   (« 12:30 ») ; une note de correction d'un expert posée sur l'intervalle 12:30–14:10 du
   Module 9 (« Depuis la CdP30, les indicateurs du GGA sont adoptés — voir le guide CdP31,
   p. 59 et sa note ») ; l'état hors connexion,
   où la transcription reste lisible sans la vidéo.
Règles : un module porte « Quiz relu par un expert », « Quiz réussi 8/10 » ou « Pas encore
de quiz » — jamais « Quiz non relu », qui n'existe que pour un quiz personnel, privé.
Téléchargement d'une vidéo : « Vidéo · 180 Mo » ou « Audio seul · 35 Mo ».
Données — seize modules, titres à remplacer par ceux du programme réel :
  Bases : 1 La CCNUCC, le Protocole de Kyoto et l'Accord de Paris · 2 Le processus de
  négociation : organes, sessions, décisions · 3 Les groupes de négociation et la place
  de la Francophonie · 4 Comprendre un texte de négociation (vu à 35 %).
  Thématiques : 5 Atténuation et CDN · 6 Finance climat · 7 Article 6 et marchés carbone ·
  8 Transparence · 9 L'adaptation dans l'Accord de Paris (48 min, en cours, quiz relu) ·
  10 Pertes et préjudices · 11 Transition juste · 12 Genre et changements climatiques.
  Pratique : 13 Terminologie et anglais de négociation · 14 Prendre la parole et rédiger
  une proposition · 15 Simulation de négociation · 16 Restituer et communiquer.
  Modules 1 à 3 terminés, quiz réussis ; quiz relu sur 1 à 4 et 9 ; « Pas encore de quiz »
  ailleurs ; durées de 32 à 55 min. Les modules Bases et Pratique n'ont pas de thématique
  et n'apparaissent que sous « Tous ».
```

## 17 — Quiz · groupe « 16 — Quiz » — après le MVP

```
Page « 16 — Quiz », six écrans :
1. Les quiz — par document, par vidéo, par thématique ; « Relu par un expert ».
2. Une question à choix multiple, et une question vrai ou faux.
3. La correction — la bonne réponse, et le passage source qui la justifie, ouvrable.
4. Le résultat, et ma progression dans le temps.
5. « Régénérer un quiz pour moi » — reformuler les questions, ou porter sur d'autres
   passages ; le quiz obtenu reste privé et porte « Non relu » ; génération en cours.
6. « Proposer à la relecture » — et l'état « En relecture ».
Règles : le passage d'origine ne s'affiche qu'à la correction, jamais dans l'énoncé — un
quiz se joue de mémoire, la source sert à vérifier. « Réussi » à 7 sur 10 ; « À refaire »
en gris, jamais en rouge.
Données — quiz relus : Guide CdP31, 10 questions, 8/10 le 12 novembre (6/10 le 10, 7/10
le 11 : c'est l'écran de résultat et sa progression) · « Au nom de ma délégation »,
8 questions, jamais fait · Modules 1 à 3, réussis 8, 9 et 10/10 · Modules 4 et 9, jamais
faits · Adaptation, 12 questions, 9/12 le 10 novembre · Finance, Genre, jamais faits.
Mes quiz : « GGA — questions reformulées », 10 questions, Non relu · Privé, « En
relecture — envoyé à 11:20 ». QCM : « Combien d'indicateurs du GGA la CMA a-t-elle
adoptés à Belém ? » 50 · 59 · 100 · 200 → 59 (chiffre à confirmer par un expert), source
Guide CdP31 p. 59 et sa note. Vrai ou faux : « Une consultation informelle peut aboutir
à un texte convenu (document L). » → Vrai, source Guide CdP31, annexe A.3, p. 74.
```

## 18 — Restitutions · groupe « 17 — Restitutions » — après le MVP, exploratoire

```
Page « 17 — Restitutions », à confirmer sur le terrain. Quatre écrans :
1. Ma restitution du jour — préremplie avec les sessions de négociation suivies ;
   rubriques : ce qui s'est dit, points de blocage, prochaines étapes, à remonter au
   ministère ; aide à la rédaction (« Reformuler », « Résumer mes notes ») ; export en
   PDF ou par courriel.
2. Choix du cercle — « Privé » par défaut ; ma délégation, mon groupe de négociation,
   tout le réseau ; un rappel sobre : la position d'un pays ne se partage pas ici.
3. Le fil des restitutions partagées, par thématique.
4. « Ce que le réseau a vu hier sur l'adaptation » — une synthèse marquée « Synthèse
   automatique », avec les restitutions dont elle vient.
Règles : une rubrique accepte 1 200 caractères ; l'aide de l'IA ne remplace jamais mon
texte sans « Garder », et attend le réseau. Cercles : Privé (défaut) · Ma délégation
(« Sénégal — 6 membres du réseau », pas la délégation entière) · Mon groupe de
négociation, à choisir parmi ceux de mon pays (Groupe africain · PMA · G77 et Chine) ·
Tout le réseau. « À remonter au ministère » ne se partage jamais. Une synthèse ne lit que
les restitutions partagées au cercle où elle s'affiche ; le fil ne montre que ce qui
m'est partagé.
Données (12 novembre, 11:40) : ma restitution du jour, Privé, en cours — session cochée :
consultations informelles GGA, 10:00–12:00, salle 7 ; à venir, décochées : Fonds pour
l'adaptation, Coordination des PMA, Groupe de contact genre 15:00 salle 9, aparté 16:30.
« Ce qui s'est dit » : « Nouvelle itération distribuée à 10:05, en anglais ; deux options
entre crochets sur les moyens de mise en œuvre. » « Points de blocage » 1 · « Prochaines
étapes » 1 · « À remonter au ministère » vide. Aucune autre restitution du 12 n'existe
encore à 11:40 : le fil montre celles d'hier soir — Mariam Traoré (Mali), 11 nov. 18:40,
Tout le réseau, GGA · Nadège Kouassi (Côte d'Ivoire), 11 nov. 19:10, Tout le réseau, Fonds
pour l'adaptation · Clarisse Nguema (Gabon), 11 nov. 20:30, Mon groupe, Article 6 ·
Marie-Josée Pierre (Haïti), 11 nov. 21:05, Tout le réseau, GGA. Synthèse « Ce que le
réseau a vu hier sur l'adaptation — mercredi 11 novembre », produite à 07:00 depuis les
trois restitutions partagées à tout le réseau ; celle de Clarisse n'y entre pas.
```

## 19 — Back-office · groupe « 18 — Back-office », sur poste — au fil des modules

```
ATTENTION : ces écrans ne sont PAS l'application. Ils s'ajoutent au back-office existant
de l'ePavillon, où l'équipe gère déjà tout le reste, et en gardent donc l'apparence
(joindre guide-de-style-epavillon.html). À faire dans un projet séparé, pour que rien de
ce style ne déteigne sur Guide Négo. Écran de 1440 px.
1. Documents — publier en une journée : téléverser, type, thématiques, version,
   « Remplace… », réservé ou public, « Utilisable par l'assistant ».
2. FAQ et lexique — édition, « Vérifié le », rubriques.
3. Accès — codes d'invitation (créer, révoquer, qui est entré avec lequel), mode
   d'admission (code, approbation, les deux), demandes en attente.
4. Import des sessions de négociation — dernière lecture réussie, interrupteur
   « Afficher l'import », journal des lectures, écarts constatés.
5. Signalements — file, historique, qui a validé quoi.
6. Corpus de l'assistant — « Arrivée » et « Référence », état de chaque source, notes de
   correction, retours « Dépassé ou faux ».
7. Quiz — génération depuis un document ou une vidéo, relecture, publication.
8. Canaux et modération.
9. Usage — installations, personnes actives par jour, téléchargements, recherches au
   lexique, signalements, questions restées sans réponse.
```

## R — Revue de cohérence — après chaque page

```
Relis la page que tu viens de produire comme un relecteur sévère, puis corrige :
1. Plein soleil : liste chaque texte sous 7:1 (courant) ou 4,5:1 (secondaire), chaque
   graisse sous 400, chaque texte jaune sur clair, chaque cible sous 48 px.
2. Vocabulaire : le mot « Programme » seul apparaît-il ? Les trois agendas portent-ils
   leur nom complet ? Compare au lexique joint.
3. États : chaque écran a-t-il chargement, vide, erreur, hors connexion, accès réservé ?
   Un état passe-t-il par la couleur seule ?
4. Système : as-tu créé une couleur, une taille ou un composant hors de la page
   « 01 — Système » ? Ramène-le au système ou ajoute-le d'abord.
5. 360 px : rien ne déborde, rien ne défile à l'horizontale, l'action principale est en bas.
6. Dates : chaque heure porte-t-elle son fuseau ?
Donne-moi la liste des corrections faites, et complète le « Journal de maquette ».
```

## P — Passation — à la fin

```
La maquette est validée. Prépare la passation vers Claude Code :
1. `tokens.json` — couleurs de marque et jetons sémantiques (clair et sombre),
   typographie, mesures, rayons, durées.
2. `theme.css` — les mêmes jetons en variables CSS, sémantiques bornés à la mise en page
   de Guide Négo, sans redéfinir une seule couleur de marque.
3. `mesures.css` — grille, espacements, cibles, zones sûres.
4. `composants.md` — chaque composant : rôle, variantes, états, règles d'usage.
5. `mouvement.md` — durées, courbes, ce qui s'anime et ce qui ne s'anime jamais.
6. Un fichier HTML autonome par page, numéroté comme les pages, et un `index.html`.
7. La liste des polices, avec licence et fichiers à embarquer.
8. Les écarts assumés par rapport au brief, chacun avec sa raison.
```
