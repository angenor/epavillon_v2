# 05 — Design

> Ce que l'interface de Guide Négo doit être, et pourquoi. La maquette se fait dans Claude Design avec [design/prompts-claude-design.md](design/prompts-claude-design.md) ; ses livrables reviennent dans [design/](design/). Le vocabulaire de l'interface est dans [design/lexique.md](design/lexique.md).

## Intention

**Agréable, belle, et lisible en plein soleil.** Une application qu'on a plaisir à ouvrir dix fois par jour : chaleureuse, sûre d'elle, à la hauteur d'une déléguée qui représente son pays.

**Guide Négo a sa propre identité.** Elle ne reprend rien du design de l'ePavillon — ni son cyan, ni sa mise en page, ni ses composants, ni sa retenue — [ADR-016](adr/016-vert-et-jaune-fonces-police-hors-charte.md). Seules les couleurs viennent de la charte de l'IFDD.

**Direction retenue le 19/09 : « Typographique »** — le blanc dominant, le vert très foncé comme encre, une seule police. La beauté vient de la typographie, du rythme et de la justesse des espaces. L'écran de référence est dans [design/ecrans/00-direction-retenue.html](design/ecrans/00-direction-retenue.html), la décision dans [ADR-018](adr/018-direction-typographique-quatre-onglets.md).

## Les conditions d'usage commandent tout

- Debout dans un couloir, une main prise, entre deux salles ; ou dehors, en plein soleil, entre deux bâtiments.
- Tard le soir en salle : les séances se prolongent la nuit.
- Téléphone Android de milieu de gamme, écran de 360 px de large, réseau saturé ou absent.
- Peu de temps : trouver une salle, un terme anglais ou un document en moins de dix secondes.
- Des lectrices de documents de négociation : la densité ne les rebute pas, le flou si.

## Couleurs

Dominante **vert foncé**, seconde **jaune foncé** : des nuances foncées du vert et du jaune de la [charte graphique de l'IFDD](../CHARTE_GRAPHIQUE.md), contrastes mesurés. Ce sont des repères : la maquette peut ajuster la clarté d'une nuance si le contraste tient, jamais la teinte.

| Nuance | Valeur | Contraste | Usage pressenti |
|---|---|---|---|
| Vert très foncé | `#233400` | 13,4:1 sur blanc | En-têtes, navigation, grands aplats |
| Vert foncé | `#3C5404` | 8,5:1 sur blanc | Action principale, liens |
| Vert moyen | `#557607` | 5,3:1 sur blanc | Pictogrammes, bordures franches |
| Vert de charte | `#8FBF2F` | 2,2:1 sur blanc | Accent **sur vert foncé** ; jamais en texte sur clair |
| Jaune foncé | `#D3B011` | 7,8:1 avec le noir | Aplat de mise en avant, texte noir dessus |
| Jaune très foncé | `#816B06` | 5,2:1 sur blanc | Texte et pictogrammes jaunes sur fond clair |
| Jaune de charte | `#FFD500` | 1,4:1 sur blanc | Accent **sur vert foncé** ; jamais sur blanc |
| Noir | `#231F20` | — | Texte courant |
| Gris, gris pâle | `#565554`, `#D9D8D6` | 7,4:1 pour le gris | Texte secondaire ; fonds et séparations |
| Teintes claires | `#ECFFD3`, `#FFF7DA` | — | Fonds de bloc vert et jaune |

**Les états d'une session**, décidés avec la direction : En cours vert `#557607` · Déplacée cyan `#0C6792` · Annulée rouge `#C7101D` · Non annoncée violet `#732F85` · Terminée gris `#565554` · Prévue noir. Le système de design étend cette correspondance aux autres objets. Une règle ne bouge pas : **un état = un pictogramme + un mot + une couleur**, jamais la couleur seule.

## Typographie

**Ni Helvetica ni NeueMaverick** : les polices de la charte ne conviennent pas à une application mobile. Critères du choix :

- licence ouverte, fichiers embarqués dans l'application — aucune police ne se télécharge en salle ;
- diacritiques français complets, chiffres tabulaires pour les horaires ;
- grande hauteur d'x, formes ouvertes, `I`, `l` et `1` distincts, `O` et `0` aussi ;
- graisses de 400 à 700, jamais moins de 400.

**Police retenue : Atkinson Hyperlegible Next**, seule, en 400, 600 et 700. Conçue pour la basse vision — c'est exactement la contrainte du plein soleil —, à licence ouverte. La maquette la charge depuis Google Fonts ; l'application l'embarque.

## Lisible en plein soleil

1. Thème clair par défaut, fonds opaques. Texte courant à **7:1** au moins, texte secondaire à 4,5:1 — aucun gris clair.
2. Corps de 17 px, interligne 1,5. L'heure et la salle d'une session : 20 px et graisse 600 au moins.
3. Deux blocs voisins se distinguent sans effort : aplat, bordure franche ou espace — jamais une ombre légère seule.
4. Jamais de texte posé sur une photo, un dégradé ou une surface translucide.
5. Le jaune ne s'écrit jamais sur du clair.
6. Cibles tactiles de **48 px**, action principale en bas de l'écran, à portée de pouce.
7. Thème sombre pour les séances de nuit : pas une inversion, vert et jaune désaturés.
8. Aucune animation n'est nécessaire pour comprendre.

Trois points de l'écran de référence ne tiennent pas encore ces règles, et se règlent au système de design : des filets de liste sous 2:1, des textes à 14 px dont un italique, et des signes d'état qui sont des caractères et non des pictogrammes.

## Navigation

**Décidé le 19/09, complété le 20/09** : une barre basse avec pictogrammes — **Accueil · Négociations · Francophonie · Ressources**, et **Échanges** en quatrième position dès que le module ouvre ; onglets à largeur de libellé. Le lexique n'est pas un onglet : un bouton « Aa », permanent, en haut à droite de chaque écran, l'ouvre d'un geste.

**Trois agendas, jamais confondus** ([ADR-008](adr/008-trois-agendas-jamais-confondus.md)) : « Sessions de négociation », « Réunions de la Francophonie », « Pavillon de la Francophonie ». Le mot « Programme » seul n'apparaît nulle part. Ils ne se mêlent que dans « Ma journée », sur l'accueil, où chaque ligne porte son origine.

Trois éléments reviennent sur tous les écrans : l'état de connexion (« Hors connexion — lu à 14:05 »), l'origine d'une donnée importée (« Source officielle »), et le verrou d'un module réservé, qui invite à saisir son code.

## Inventaire des écrans

| Page de maquette | Écrans | Forme |
|---|---|---|
| Socle | Installation, compte, code d'invitation, attente d'approbation, thématiques, accueil « Ma journée », recherche globale, notifications, profil, états transverses | Groupe |
| Documents | Bibliothèque, fiche, mes documents | Groupe |
| Lecteur | Lecture d'un document, sommaire, recherche | Dédiée |
| Savoir | FAQ, entrée de FAQ, question à un expert, parcours « Ma première COP » | Groupe |
| Lexique | Recherche, entrée | Dédiée |
| Sessions de négociation | Liste du jour, filtres, état de l'import | Dédiée |
| Détail d'une session | Détail, signalements posés par-dessus | Dédiée |
| Signaler, mon agenda | Signalement, suivi, agenda personnel | Groupe |
| Francophonie | Réunions de la Francophonie, Pavillon | Groupe, deux sections distinctes |
| Validation | Files de l'administrateur et de l'expert, sur téléphone | Groupe |
| Échanges | Canaux, annuaire, questions aux experts, proposer un document | Groupe — après le MVP |
| Conversation | Canal, conversation privée, annonce | Dédiée — après le MVP |
| Assistant | Réponse citée, « Je ne sais pas », résumé en français | Dédiée — après le MVP |
| Formations | Modules, fiche, lecteur vidéo avec transcription | Groupe — après le MVP |
| Quiz | Liste, question, correction, quiz personnel | Groupe — après le MVP |
| Restitutions | Rédaction, cercle, fil partagé, synthèse | Groupe — à confirmer |
| Back-office | Gestion sur poste | Groupe, projet séparé |

La maquette couvre **tous** les écrans, y compris ceux d'après le MVP : un système de design se juge sur l'ensemble.

**Le back-office n'est pas l'application.** Ses écrans s'ajoutent au back-office existant de l'ePavillon, où l'équipe gère déjà tout le reste, et en gardent l'apparence. Ils se maquettent à part, pour que rien ne déteigne sur Guide Négo.

## La maquette livrée

Dans [design/ecrans/](design/ecrans/) : dix-huit pages HTML autonomes (00 à 17), leur `index.html`, le système `01-systeme.html` et le `journal-de-maquette.html` — à lire dans l'ordre du [README](design/ecrans/README.md). Dans [design/passation/](design/passation/) : `tokens.json`, `theme.css`, `mesures.css`, `pictogrammes.svg`, `composants.md`, `mouvement.md`, la notice de la police, et [ecarts.md](design/passation/ecarts.md). Dans le code, ce système vit dans le dossier de Guide Négo et ne touche à aucun jeton du site.

Deux décisions prises en maquette : **le jaune signale ce qui vous concerne à l'instant** — session en cours, non-lu, mention — et rien d'autre (une marque de rôle est une étiquette bordée) ; **la barre a cinq onglets** à largeur de libellé dès que les Échanges sont ouverts, quatre avant.

## Les écarts, tranchés

Numéros de [ecarts.md](design/passation/ecarts.md). Le code suit ces décisions, pas les pages qui les contredisent.

| Écart | Décision |
|---|---|
| 1 | `#2E3F0E` entre au système, pour le squelette sombre seulement ; jamais en filet |
| 2, 11 | Badges de maquette et rayon 6 px : hors application |
| 3 | Un seul vert d'état en sombre : `#B5D66A` |
| 4, 27 | Les rôles sans valeur sombre se définissent à l'étape 0a, contrastes mesurés ; aucun écran ne sort sans son thème sombre |
| 5 | Pictogramme Document : vert moyen dans une liste, vert très foncé dans un menu ou un bouton |
| 6, 15 | Pas de 16 px : onglets de filtre et titres de ligne à 17 ; hauteur d'onglet de filtre 48 |
| 7 | Page 00 : planche d'archive, rien à en reprendre sous 15 px |
| 8, 9 | « Aa » et « GN » sont des glyphes de bouton, hors échelle de texte |
| 10 | 13 px : libellés d'onglets, marque de rôle, compteur, jour de la bande — rien d'autre |
| 12 | Barre à largeur de libellé partout ; les pages à quatre onglets égaux sont antérieures |
| 13, 14 | Précisions d'écriture : 48 px pour une ligne de liste, 56 px pour une ligne à réglage, à cocher ou à cercle |
| 16, 17, 18 | `pause`, `chevUp` et `minus` entrent dans la famille ; le compte est celui de `pictogrammes.svg` |
| 19 | Le bouclier coché dit « validé par un expert », sous ses trois libellés : confirmé |
| 20 | Croix rouge = annulé ; triangle rouge = erreur, lecture impossible, ou passage dépassé. *Élargi le 22/09, étape 1* : la note de correction d'un expert porte le triangle rouge — un passage dépassé ou faux est une erreur du texte ; l'écart l'opposait à la croix d'« annulé », pas à la note |
| 21, 22, 24, 25 | Données et phrases : une seule phrase de verrou, celle de la page 12 ; « Synchronisé à » porte l'heure de dernière lecture, distincte de l'instant |
| 26 | Icône provisoire : `frontend/public/logos/svg/epavillon-symbole-inverse.svg`, à la place du monogramme « GN » |
| 28, 29 | À faire à l'étape 0a : ordre de focus et annonces d'état, vérification à 390 px |
| 30 | Le bloc `:root` de `theme.css` — charte et nuances sombres — passe sous `[data-app="guide-nego"]` : aucun sélecteur de Guide Négo hors de sa borne (constitution, XIII) |
| 31 | `--gn-picto` désigne la couleur (`theme.css`) ; la taille de `mesures.css` devient `--gn-picto-taille` |
| 32 | « lu à » et « Synchronisé à » disent l'heure du téléphone, **sans fuseau** : elles se jugent contre l'horloge affichée au-dessus. Hors du jour : « hier à 23:10 », puis « le 11 nov. à 23:10 ». La ligne « Dernière synchronisation » du profil (0c) suit la règle et perd « heure d'Antalya ». Seules les heures d'événement portent leur fuseau |
| 33 | **Le bouton dangereux prend un rouge assombri.** Le rouge de charte `#C7101D` porte du texte blanc à 6,0:1, sous les 7:1 que le plein soleil exige : l'**aplat** du bouton passe à `#B30E1A` — même teinte, clarté abaissée jusqu'au seuil exactement (7,0:1). Le texte, les pictogrammes et les marques d'état rouges gardent `#C7101D`, qui tient son seuil de 4,5:1. Deux rôles : `--gn-danger` et `--gn-danger-aplat`. Arbitré le 21/09 |
| 34 | **Trois écrans que la maquette n'a pas prévus : le retour d'un courriel.** `02-socle.html` mène du compte au code sans passer par la confirmation d'adresse — or le courriel de vérification, comme celui du mot de passe oublié, ramène aujourd'hui sur un écran du **site**, à ses couleurs et sans chemin de retour. S'ajoutent donc `verification-adresse`, `nouveau-mot-de-passe` et, sur l'écran de compte, l'attente de confirmation avec « J'ai confirmé mon adresse » et « Renvoyer le courriel ». Ils se composent avec les composants livrés en 0a, sans nouveau dessin. **Sur iPhone, l'application installée ne partage pas le stockage du navigateur** : la page confirme et renvoie — « Adresse confirmée — retournez dans Guide Négo ». Sur Android, elle ouvre le lien et enchaîne. **Précisé en construisant (21/09) : « enchaîner » mène à la CONNEXION, pas à l'étape suivante du parcours.** L'inscription n'ouvre aucune session — une adresse non vérifiée ne se connecte pas —, donc au retour du courriel il n'y a encore rien à relire : `/auth/me` ne rend rien, et aucune route ne peut dire « cette adresse est-elle confirmée ? » sans révéler qu'un compte existe. La relecture automatique au retour au premier plan vaut pour les écrans où une session existe déjà — le code d'invitation, « Mon accès » —, et l'adresse est reportée dans le champ de connexion pour que personne ne la ressaisisse juste après l'avoir confirmée. Arbitré le 21/09, étape 0b |
| 35 | **Le pays se choisit dans une liste, pas dans un champ de texte.** La maquette dessine « Pays » comme un champ de saisie portant « Sénégal », sans dire d'où vient la valeur — or les pays sont une donnée du référentiel, 249 entrées, et une saisie libre ne peut pas viser juste. L'écran emploie donc la liste native du téléphone, **dans le créneau de `GnChamp`** : libellé, aide et message d'erreur restent ceux du composant, et rien n'est redessiné. Le clavier qui s'ouvre est celui que la personne connaît sur son propre appareil. Arbitré le 21/09, étape 0b |
| 36 | **L'indicateur d'étapes n'est pas sur la planche.** `02-socle.html` le dessine en tête des écrans de compte et de code — « Étape 2 sur 3 » —, mais `design/passation/composants.md` ne le décrit pas et la planche de 0a ne le montre pas. Il est livré comme `GnEtapes` et rejoint la planche avec les autres surfaces. Arbitré le 21/09, étape 0b |
| 37 | **Le code d'invitation fait huit caractères, tirets compris.** La maquette écrit `NEGO-24` en exemple — sept — sous une phrase qui dit « vérifiez les huit caractères ». Huit fait foi : l'exemple devient `NEGO-024`, et le générateur de l'API tire sept caractères plus un tiret. Arbitré le 21/09, étape 0b |
| 38 | **« Demander l'accès à l'IFDD » n'apparaît qu'avec US4.** L'écran du code la dessine en sortie, mais la demande n'existait pas encore : un bouton menant à une adresse inexistante est pire que son absence, et chaque issue garde par ailleurs au moins une suite. Livrée le 22/09 avec l'écran de demande, elle ne paraît que **quand le mode d'admission l'accepte** — en « code seul », personne ne traiterait la demande et la réponse promise ne viendrait jamais. Arbitré le 21/09, construit le 22/09, étape 0b |
| 39 | **Le verrou sans compte porte une sortie de plus.** L'écran « 13 Réservé » en dessine deux et suppose un compte ; sans compte, « Saisir mon code d'invitation » mènerait à un écran qui redemande d'abord de se créer un compte (FR-031). Le verrou enchaîne donc lui-même : « Créer mon compte », « J'ai déjà un compte — Me connecter », puis « Continuer en visiteur ». **Deux sorties dès qu'un compte existe**, comme la maquette. Et la sortie principale suit le mode d'admission : le code, la demande, ou le retour à une demande en attente — un code proposé en « approbation seule » enverrait chercher un code qui n'ouvre rien (FR-022). Arbitré le 22/09, étape 0b |
| 40 | **Les trois interrupteurs de consentement de « 12 À propos » ne sont pas livrés.** La maquette dessine « Mesures d'usage anonymes », « Notifications » et « Annuaire du réseau » — or **aucun des trois n'a d'effet à l'étape 0c** : rien ne mesure l'usage, les notifications viennent à 3b, l'annuaire après le MVP. Un interrupteur qui ne commande rien fait croire à un réglage et trompe (constitution, XII). L'écran porte donc l'édition, la source, la confidentialité et **les textes**, et chaque accord paraîtra avec l'étape qui lui donne un effet. Ce qui est livré ici, c'est **la source unique des textes** — un fichier `fr`/`en` par texte, avec sa version, servi par une route publique de l'API pour le site comme pour l'application — et la consignation d'un accord sous **la version servie**, en remplacement du réglage `PRIVACY_POLICY_VERSION`. Arbitré le 22/09, étape 0c | — *Mis à jour le 26/09, étape 3b* : **« Notifications » est livré** et gouverne **le courriel** des changements de sessions (FR-030) ; l'application prévient toujours. Allumé par défaut — aucune ligne vaut accord —, chaque bascule écrit une ligne dans `identity.consents` avec la version du texte servi. Son sous-titre dit cet effet (écart 58). « Mesures d'usage anonymes » et « Annuaire du réseau » **restent non livrés**, pour la même raison qu'au premier jour
| 41 | **Le téléchargement ne demande pas de compte.** L'écran « 02 Ouverture » (02-socle) range « Favoris, téléchargements, quiz » sous « Avec un compte » ; or un document public se télécharge sans compte — une déléguée sans compte lirait sinon le guide avec du réseau, jamais en salle, contre ADR-003 — et un téléchargement ne crée rien sur le serveur. Les téléchargements quittent ce groupe, qui garde « Favoris, quiz » ; le favori, qui suit le compte, y reste. Arbitré le 22/09, étape 1 |
| 42 | **« Marquer » n'est pas livré dans la barre du lecteur.** « 04 Lecteur », écran 02, le dessine à côté de « Sommaire », « Rechercher » et « Réglages », mais aucune page ne dit ce qu'il produit ni où l'on retrouve ses marques. Un bouton sans effet trompe (constitution, XII). Il paraîtra quand son résultat sera dessiné. Arbitré le 22/09, étape 1 |
| 43 | **Le lecteur s'ouvre sur la page du PDF ; le texte recomposé devient « Texte agrandi ».** « 04 Lecteur » dessine une page de texte recomposé, taille réglable et termes anglais touchables ; aucune page ne dessine le PDF. Le commanditaire a tranché le 24/09 : on lit le document tel que l'IFDD l'a mis en page — colonnes, tableaux, figures —, la page tenant d'abord la largeur de l'écran, agrandie d'un pincement ; l'interface suit le thème, la page garde ses couleurs. La page recomposée de la maquette devient le second mode, « Texte agrandi », pour qui lit mal une page A4 réduite à la largeur d'un téléphone. **Le choix « Pages · Texte » se voit** : il prend dans la barre dépliée (écran 02) l'emplacement laissé libre par « Marquer », qui garde ainsi ses quatre emplacements, et reste aussi dans « Réglages » (écran 07) avec la taille et le thème. Au premier document ouvert sur un écran étroit, une ligne d'information dit une seule fois que « Texte agrandi » existe. Arbitré le 24/09, étape 1b |
| 44 | **Le pictogramme de « Réglages » ne ressemble pas à « Aa ».** `gn-text-size` dessine un grand A et un petit A : sur l'écran du lecteur, il côtoie le « Aa » de l'en-tête, qui ouvre le lexique partout ([ADR-018](adr/018-direction-typographique-quatre-onglets.md)). Deux « Aa » sur un même écran ne peuvent pas mener à deux endroits. Le bouton s'appelle « Réglages », comme dans la maquette, et prend un pictogramme de réglage qui entre dans la famille. Arbitré le 24/09, étape 1b |
| 45 | **Les titres des sessions officielles sont traduits par l'IA et publiés sans relecture humaine.** Exception au principe XII, décidée par le commanditaire le 25/09 : une COP compte des centaines de titres par jour, et une relecture arriverait après la session. Elle est bornée : **les titres des sessions officielles seulement** ; le titre anglais d'origine reste affiché, précédé de « EN », et **c'est lui qui fait foi** ; la traduction porte « Traduction automatique » sur chaque ligne et chaque fiche ; une seule traduction par titre anglais, gardée avec son modèle et sa date, refaite seulement si le titre change ; sans traduction, l'anglais s'affiche seul, sans la mention. Inscrit aussi dans `COMMENT ON TABLE negotiation.title_translations`. Arbitré le 25/09, étape 3a |
| 46 | **Le rappel « 15 minutes avant » ne sonne pas.** « 08 Détail de session » dessine l'interrupteur sans dire comment le rappel arrive ; l'étape 3a n'a ni notification poussée ni service qui réveille le téléphone (le centre de notifications vient à 3b). Le rappel paraît donc **en tête de Guide Négo, application ouverte**, une fois par session, hors connexion aussi. Pour ne pas laisser croire à une alarme, une ligne sous l'interrupteur le dit : « Le rappel paraît en tête de Guide Négo quand l'application est ouverte, sans sonnerie ni notification. » Le bandeau du rappel n'est pas maquetté : il reprend la matière du bandeau jaune (`GnBandeauRappel`) — l'heure avec son fuseau, le titre, la salle, « Voir la session ». Arbitré le 25/09, étape 3a |
| 47 | **La liste des sessions dit le fuseau une fois, au-dessus des heures.** « 07 Sessions » l'écrit dans « Source officielle — lu à…, heure d'Antalya » ; or « lu à » se donne sans fuseau (écart 32), et chaque ligne répétant « heure d'Antalya » mangerait une ligne sur quatre à 360 px. Le fuseau se lit dans l'en-tête du jour — « Vendredi 25 septembre — heure d'Antalya » —, et chaque ligne le porte dans son nom accessible ; la fiche le redit sous l'heure, avec le décalage. Arbitré le 25/09, étape 3a |
| 48 | **Le fil « Sessions de négociation · type » passe sous le titre de la fiche.** « 08 Détail de session » le pose au-dessus, en surtitre ; `GnEntete` n'a pas encore de surtitre (il arrive avec l'étape 2). Il remontera à la fusion. Arbitré le 25/09, étape 3a |
| 49 | **La fiche d'une session n'a pas de section « Documents liés ».** La maquette la dessine ; la source officielle ne relie aucun document à une session, et une section toujours vide promettrait ce qu'aucune donnée ne tient. Elle viendra quand un document pourra être rattaché. Arbitré le 25/09, étape 3a |
| 50 | **« Validé par l'IFDD », partout.** Les maquettes 02, 07 et 09 écrivent « validé par un expert », la 08 « validé par l'IFDD à … ». Ce n'est pas un expert qui tranche un signalement de session mais une personne qui tient la permission de valider pour l'IFDD : l'encart de la fiche, la ligne de liste et de « Mon agenda », l'avis et le courriel disent « validé par l'IFDD », jamais le nom de l'autrice (SC-008). La réunion non annoncée garde la phrase courte de « 07 Sessions », « Non annoncée — signalée par le réseau, validée à … », et sa fiche dit que l'IFDD l'a vérifiée. Arbitré le 26/09, étape 3b |
| 51 | **« Validé. Affiché dans une minute au plus. »** « 11 Validation », écran 1, écrit « Validé. Affiché à toutes et tous. » ; or la publication est **différée** — un travail la pose une trentaine de secondes après la décision, pour que « Annuler » (six secondes à l'écran, trente côté serveur) ne laisse partir ni avis ni courriel (SC-004). La phrase de la maquette serait fausse pendant ce délai. Arbitré le 26/09, étape 3b |
| 52 | **La précision se saisit dans la même feuille que le motif.** « 09 Signaler » enchaîne le choix du motif et la saisie de la précision comme deux vues. La feuille garde le même cadre et remplace les motifs par la précision (salle, heure, texte) et « Envoyer » : trois gestes en tout (SC-002), et « Annuler » ramène à la fiche sans rien envoyer. Arbitré le 26/09, étape 3b |
| 53 | **« Mes signalements » s'ouvre depuis le profil.** La maquette dessine l'écran (09 · 2a) sans dire d'où l'on y arrive, hors du message éphémère « Voir ». « 11 Profil » gagne une ligne « Mes signalements » dans « Mon suivi », pour qu'on y revienne sans avoir signalé la minute d'avant. Arbitré le 26/09, étape 3b |
| 54 | **Le repère « Signalé » paraît dans les listes.** « 07 Sessions » ne dessine la mention du réseau que sur la réunion non annoncée. Une session officielle dont un changement signalé est validé porte, dans la liste et dans « Mon agenda », le losange violet et « Signalé par le réseau, validé par l'IFDD à … » : sans lui, on ne saurait qu'en ouvrant la fiche qu'une salle ou une heure n'est plus celle de la source. Le losange accompagne toujours le texte, jamais la couleur seule. Arbitré le 26/09, étape 3b |
| 55 | **« Administration » et « L'autrice ou l'auteur verra le motif ».** « 11 Validation » écrit « Administratrice » au-dessus du titre et, après un refus, nomme la personne. Le rôle ne suppose pas un genre : le surtitre dit « Administration ». La feuille « Ne pas retenir » garde le nom (« Clarisse Nguema verra le motif »), qui est sous les yeux ; le message qui suit la décision, bref et sans nom, dit « Non retenu. L'autrice ou l'auteur verra le motif. » Arbitré le 26/09, étape 3b |
| 56 | **Les boutons de « Ne pas retenir » sont empilés.** La maquette (11 · 1f) pose « Revenir » et « Ne pas retenir » côte à côte. À 360 px, avec la taille de texte agrandie, deux libellés de 48 px de haut ne tiennent pas sur une ligne sans se couper : l'action principale passe au-dessus, pleine largeur, « Revenir » dessous, comme dans les autres feuilles de Guide Négo. Arbitré le 26/09, étape 3b |
| 57 | **Pas de rappel pour une réunion non annoncée.** Sa fiche n'offre pas « Me rappeler 15 minutes avant » : son heure vient d'un signalement, qu'un administrateur peut retirer à tout moment, et un rappel sonnant pour une réunion retirée tromperait. Elle s'ajoute à « Mon agenda », où elle garde sa mention du réseau. Arbitré le 26/09, étape 3b |
| 58 | **Le sous-titre de l'interrupteur « Notifications » dit ce qu'il commande.** « 12 À propos » écrit « Réglées par thématique dans votre profil » ; or l'interrupteur ne règle pas les thématiques — il décide du **courriel** (FR-030). Il dit : « Les changements de vos sessions vous arrivent aussi par courriel. L'application vous prévient toujours. » Les thématiques se règlent au profil, sous « Notifications par thématique ». Arbitré le 26/09, étape 3b |
| 59 | **Une ligne du centre de notifications commence par l'état.** « 02 · 10 » écrit « Groupe de contact … — déplacé à 15:00, salle 9 », l'état en fin de phrase. FR-028 veut l'état en tête, là où l'œil le trouve en parcourant la liste : « Déplacée — Groupe de contact …, nouvelle heure 15:00 (heure d'Antalya), selon la source officielle. » La seconde ligne garde la forme de la maquette, « Sessions de négociation · 09:48 » — l'origine et l'heure, sans redire l'état. Arbitré le 26/09, étape 3b |
| 60 | **L'entrée de la file s'appelle « Validation ».** Ressources ouvre « Validation », avec le nombre à traiter, puis « Signalements » : c'est le nom de la page 11 de la maquette, qui porte aussi les autres files de validation à venir (réponses d'experts, restitutions). Arbitré le 26/09, étape 3b |

### Les rôles sombres définis à l'étape 0a (écarts 4 et 27)

Mesurés par `frontend/scripts/guide-nego-contrastes.mjs`, qui échoue la construction sous le seuil.

| Rôle | Valeur sombre | Mesure |
|---|---|---|
| Pressé | `#182208` (bloc) | Texte `#EEF2E4` dessus : 14,5:1 |
| Focus | `#B5D66A` | Anneau sur fond : 11,2:1 ; sur bloc : 10,1:1 |
| Désactivé | Fond `#182208`, texte `#A7A6A3` | 6,8:1 |
| Fond d'attention — bandeau « Remplacé par… » | `#2B2607`, **nuance ajoutée** : aucune existante ne gardait le jaune | Texte dessus : 13,3:1 |
| Bulle envoyée | Fond `#B5D66A`, texte `#101704` | 11,2:1 |
| Fond de jauge | `#182208` — presque le fond : en sombre, la jauge porte un contour d'un filet | Partie pleine `#B5D66A` : 10,1:1 sur la piste |
| Voile | `#231F20` à 70 % | — |
| Texte sur aplat de titre | `#101704` sur `#EEF2E4` | 16,1:1 — en sombre, l'aplat `titre` devient clair |
| Texte sur bouton dangereux | `#101704` sur `#F08A90` | 7,6:1 ; en clair, `#FFFFFF` sur `#B30E1A` : 7,0:1 (écart 33) |
