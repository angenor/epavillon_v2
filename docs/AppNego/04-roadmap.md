# 04 — Feuille de route

> **Un ordre, pas un calendrier.** Chaque étape commence quand la précédente est stable. Chaque étape est une fonctionnalité Spec Kit : son prompt `/speckit-specify` est ci-dessous, prêt à coller. La suite du MVP est décrite dans [06-apres-mvp.md](06-apres-mvp.md) ; l'avancement, dans [progress.md](progress.md).

## Le chemin d'une étape

1. `/speckit-specify` avec le prompt de l'étape — le *quoi*, jamais le *comment*.
2. `/speckit-clarify` si la spécification porte des marqueurs de clarification.
3. `/speckit-plan` avec le **prompt de plan commun** ci-dessous.
4. `/speckit-tasks`, `/speckit-analyze`, puis `/speckit-implement`.
5. `make check-safe` au vert, vérification au navigateur à 360 px contre la page de maquette, puis la ligne de [progress.md](progress.md).

Les spécifications continuent la numérotation de `specs/` : la première sera `008-…`.

## La maquette fait foi

Elle est rangée dans [design/ecrans/](design/ecrans/) : un fichier HTML autonome par page, un [index](design/ecrans/index.html), le [système de design](design/ecrans/01-systeme.html) et le [journal de maquette](design/ecrans/journal-de-maquette.html). Chaque écran porte un attribut `data-screen-label` : c'est par lui qu'un prompt désigne un écran.

- **Le système de design** se lit dans `01-systeme.html` et se reprend depuis [design/passation/](design/passation/) : `tokens.json`, `theme.css` (rôles bornés à `[data-app="guide-nego"]`), `mesures.css`, `pictogrammes.svg`, `composants.md`, `mouvement.md`, et la police, fichiers et licence compris, dans `police/`.
- **Les écarts** relevés entre le système et les pages sont dans [passation/ecarts.md](design/passation/ecarts.md), tranchés dans [05-design.md](05-design.md).
- **Les lignes « En attente » du journal** sont des données inventées : elles viennent de la base, jamais du code.
- **Icône de l'application**, à titre provisoire : `frontend/public/logos/svg/epavillon-symbole-inverse.svg`, à la place du monogramme « GN » de la maquette. Le back-office n'est pas maquetté : il se construit sur celui de l'ePavillon, avec ses composants.
- Les données réelles d'essai : [donnees-lecteur.md](design/donnees-lecteur.md), [donnees-savoir.md](design/donnees-savoir.md), [donnees-lexique.md](design/donnees-lexique.md).

## Constitution : un amendement, pas une réécriture

La constitution 1.0.1 de l'ePavillon reste valable et s'applique à Guide Négo. Elle ignore quatre choses que le projet apporte ; un amendement mineur les y inscrit, **avant la première spécification**.

```
/speckit-constitution
Amende la constitution en 1.1.0, sans toucher aux dix principes existants. Ajoute une
section « Guide Négo » qui renvoie à docs/AppNego/ (brief, domaine, ADR) et pose quatre
principes, chacun vérifiable :
XI. Hors connexion d'abord — tout ce qui se lit se lit sans réseau ; toute donnée lue hors
    connexion affiche l'heure de sa lecture ; une écriture faite sans réseau part à son
    retour (ADR-003).
XII. Confiance — une donnée importée porte son origine et son heure de lecture, et
    l'affichage se coupe plutôt que de montrer du périmé (ADR-009) ; un signalement ne
    modifie jamais la donnée officielle (ADR-010) ; rien de produit par une IA n'est
    publié sans validation humaine (ADR-011, 012, 013).
XIII. Un design propre et borné — l'interface de Guide Négo suit
    docs/AppNego/design/ecrans/01-systeme.html, vit dans son dossier, ne redéfinit aucun
    jeton du site et n'emprunte aucun de ses composants ; le back-office, lui, reste
    celui de l'ePavillon (ADR-016, 018).
XIV. Une seule porte — tout service annexe (IA en Python) est interne, n'a aucune route
    publique et n'écrit que dans son schéma (ADR-004).
Inscris aussi aux contraintes : trois agendas jamais confondus, le mot « Programme » seul
banni de l'interface et de l'API (ADR-008) ; le suivi de Guide Négo vit dans
docs/AppNego/progress.md (ADR-017). Propage aux gabarits.
```

## Prompt de plan commun — `/speckit-plan`

```
Pile et règles : CLAUDE.md et la constitution, sans exception — SQL d'abord dans
docs/database/ puis rechargement sur base jetable, crate de module sans dépendance vers un
autre module, SQLx vérifié, permissions avec portée, contexte d'écriture, outbox, erreurs
françaises à code stable, contrat engendré par `make openapi`, tests d'intégration sur
base réelle. Lis docs/AppNego/01-stack.md, 02-domaine.md, 03-api.md et les ADR cités par
la spécification.
Back : crate `negotiation` (ou celui que la spécification nomme) ; routes sous
`/negotiation`, administration sous `/admin` avec filtrage par périmètre ; listes
« modifié depuis » avec empreinte, pour le hors-connexion.
Client : sous-arbre de pages dédié du Nuxt existant, mise en page propre à Guide Négo,
rendu côté navigateur, PWA. Composants dans le dossier de Guide Négo, bâtis sur
docs/AppNego/design/passation/ (jetons, mesures, pictogrammes, composants.md) et la page de maquette nommée par la
spécification — mêmes mesures, mêmes états, mêmes libellés (docs/AppNego/design/lexique.md).
i18n par écran, quatre états par écran, 360 px sans défilement horizontal, thèmes clair et
sombre. Aucun composant ni jeton du site. Back-office : pages sous `admin/`, avec les
composants `ui/` existants du site.
Attention au préfixe `/v2` (portée du service worker, manifeste, cookies). Aucun fichier
de plus de mille lignes. Ne jamais lancer `make check` ; `make check-safe` seulement.
```

## Le MVP

| # | Étape | Maquette | Critère de sortie |
|---|---|---|---|
| 0 | Socle | `02-socle.html`, `01-systeme.html` | Une négociatrice entre avec son code ; l'administrateur révoque un code et bascule en approbation sans redéployer |
| 1 | Documents | `03-documents.html`, `04-lecteur.html` | Le guide est publié en une journée, lu en salle sans réseau |
| 2 | FAQ et lexique | `05-savoir.html`, `06-lexique.html` | *Contact group* trouvé hors connexion ; une entrée de FAQ porte sa date de vérification |
| 3 | Sessions de négociation | `07`, `08`, `09`, `11-validation.html` (écran 1) | L'import tourne sur les données archivées, se coupe seul, et un signalement validé s'affiche par-dessus |
| 4 | Réunions de la Francophonie | `10-francophonie.html` | Une réunion saisie par l'IFDD apparaît, avec son lien vers le Pavillon |
| 5 | Pavillon de la Francophonie | `10-francophonie.html` | L'API existante est servie telle quelle |

### Étape 0 — Socle

```
/speckit-specify
Le socle de Guide Négo, l'application mobile des négociatrices et négociateurs
francophones (docs/AppNego/00-brief.md). Maquette : docs/AppNego/design/ecrans/02-socle.html,
tous ses écrans, de « 01 Installation » à « 13 Réservé ».
Une personne installe l'application depuis son navigateur et l'ouvre sans compte : elle
voit ce qui est public et ce qui demande un compte. Elle crée un compte ou se connecte
avec celui de l'ePavillon — c'est le même compte (ADR-001) ; la session note qu'elle vient
de l'application, et de quel appareil. Elle saisit un code d'invitation reçu sur WhatsApp :
code juste, inconnu, révoqué, épuisé, trop d'essais. Selon le réglage d'admission — code,
approbation, ou les deux (ADR-006) —, elle entre aussitôt ou attend qu'un administrateur
l'admette. Le code du réseau des négociatrices lui donne en plus l'appartenance au réseau
(ADR-007) ; aucun champ « genre » ne commande un droit. Elle choisit une ou plusieurs
thématiques de négociation, modifiables ensuite. L'accueil « Ma journée » rassemble ce qui
la concerne aujourd'hui ; chaque ligne porte son origine ; tant que les modules suivants
n'existent pas, leurs blocs affichent leur état vide. Recherche globale, centre de
notifications, profil et réglages (thématiques, téléchargements et place occupée, thème
clair, sombre ou système, notifications par thématique, mon accès, déconnexion), à propos,
confidentialité et consentements. Un module réservé, vu sans code, montre son verrou et
invite à saisir le code. Hors connexion, l'application s'ouvre, dit « Hors connexion — lu
à… », et garde ce qui a déjà été lu (ADR-003).
Côté back-office de l'ePavillon : créer et révoquer des codes, voir qui est entré avec
lequel et retirer ces accès, choisir le mode d'admission, traiter les demandes en attente.
L'application entière s'ouvre ou se ferme par un drapeau de module, sans redéploiement.
Inclus dans cette étape : la mise en page et la coquille de l'application (barre d'onglets à
largeur de libellé et bouton « Aa », ADR-018 : quatre onglets tant que les Échanges sont
fermés, cinq ensuite), le système de design repris de docs/AppNego/design/passation/ —
theme.css, mesures.css, pictogrammes.svg, composants.md, police Atkinson Hyperlegible Next
fournie dans police/ (latin et latin étendu, pour « œ ») —, l'icône provisoire de
l'application (frontend/public/logos/svg/epavillon-symbole-inverse.svg, déclinée aux
tailles du manifeste), les écarts tranchés dans 05-design.md, et le service worker. Les onglets des modules à venir montrent leur état vide.
Hors périmètre : documents, FAQ, lexique, agendas, échanges, notifications poussées.
Critère : une négociatrice entre avec le code reçu sur WhatsApp ; l'administrateur révoque
ce code et bascule en approbation sans redéployer.
```

### Étape 1 — Documents

```
/speckit-specify
La bibliothèque de documents de Guide Négo. Maquette : 03-documents.html et 04-lecteur.html
(docs/AppNego/design/ecrans/) ; données d'essai : docs/AppNego/design/donnees-lecteur.md.
Toute personne, même sans compte, cherche et filtre les documents publics par type,
thématique et COP ; un document concerne zéro ou plusieurs thématiques ; il est un fichier
ou un lien externe, jamais les deux. La fiche montre résumé, version, date, éditeur, et le
bandeau « Remplacé par… » qui mène au document à jour. Avec un compte : favori,
téléchargement pour lire sans réseau avec sa progression, « Mes documents » et la place
occupée, tout retirer en un geste. Les documents réservés demandent l'accès négociateur et
s'effacent du téléphone à la déconnexion. Le lecteur : page en lecture, barre repliée et
dépliée, sommaire, recherche dans le document (occurrence courante et autres), progression
et reprise à la dernière page lue, taille du texte, thème ; un terme anglais touché ouvre
une feuille basse — vide tant que le lexique n'existe pas ; une page peut porter la note de
correction d'un expert ; l'état « non téléchargé et pas de réseau » le dit.
Back-office : publier un document en une journée — téléverser ou lier, type, thématiques,
version, « remplace… », réservé ou public, marqueur « utilisable par l'assistant » —,
poser et retirer une note de correction sur une page.
Hors périmètre : indexation par l'IA, quiz, proposition de documents par les membres.
Critère : le guide est publié en une journée et lu en salle sans réseau.
```

### Étape 2 — FAQ et lexique

```
/speckit-specify
La FAQ, le parcours « Ma première COP » et le lexique anglais-français de Guide Négo.
Maquette : 05-savoir.html et 06-lexique.html ; données d'essai : donnees-savoir.md et
donnees-lexique.md (docs/AppNego/design/). Rien n'existe dans le modèle : tout est à créer.
FAQ : rubriques, recherche, questions les plus lues ; une entrée porte sa réponse, sa date
« Vérifié le… », ses sources, des questions liées, « Cette réponse vous a-t-elle aidée ? »
— « Non » ouvre « Qu'est-ce qui manque ? » — et le bouton « Dépassé ou faux » à trois
motifs, qui alimente une file d'experts. Une négociatrice pose une question à un expert ;
elle accepte ou non que sa question, anonymisée, rejoigne la FAQ ; aucun délai n'est promis.
Parcours : étapes à cocher par groupes, progression, lien de lecture facultatif ; les
coches restent sur le téléphone hors connexion et se synchronisent avec un compte.
Lexique : ouvert par le bouton « Aa » de chaque écran ; recherche à la frappe, tolérante
aux fautes, dans l'anglais et le français ; liste alphabétique avec rail, filtres par
famille ; entrée : terme, traduction, définition, exemple entendu en salle, sigle
développé, termes liés, source ; favoris ; « aucun résultat » propose de soumettre le
terme, ouvert à tout compte. FAQ, parcours et lexique se lisent en entier sans réseau.
La feuille du lecteur (étape 1) ouvre désormais l'entrée du lexique.
Rubriques, familles et libellés du métier sont des données, jamais des traductions.
Back-office : rédiger, publier, dater la vérification, mettre « à revoir » ; répondre aux
questions et les promouvoir en FAQ ; traiter les retours et les termes proposés.
Critère : « contact group » trouvé hors connexion ; une entrée porte sa date de vérification.
```

### Étape 3 — Sessions de négociation

```
/speckit-specify
Les sessions de négociation de Guide Négo — les réunions officielles de la CCNUCC, et
elles seules : ni les réunions de la Francophonie, ni le Pavillon (ADR-008). Maquette :
07-sessions.html, 08-detail-de-session.html, 09-signaler-et-mon-agenda.html, et l'écran
« file des signalements » de 11-validation.html.
Liste d'un jour : bande des jours qui ont au moins une session, filtre « Mes thématiques /
Toutes » — les coordinations de groupe portent « Mon groupe » et passent toujours le
filtre —, ligne avec heures de début et de fin, salle, type de réunion, titre anglais
d'origine et titre français marqué « Traduction automatique », accès ouvert ou limité,
thématique, état : Prévue, En cours, Déplacée (ancienne valeur barrée), Annulée, Terminée,
Non annoncée. Toute heure porte son fuseau.
Import (ADR-009) : un travail récurrent lit la source officielle et n'écrit que les
écarts ; chaque session porte son origine et l'heure de dernière lecture ; l'état de
l'import se lit ; passé un seuil réglable de lectures manquées, l'affichage se coupe seul
et renvoie au programme officiel — c'est aussi l'état par défaut, l'import s'allumant d'un
interrupteur du back-office. Le mécanisme s'éprouve sur des données archivées, sans
attendre l'accord du secrétariat.
Détail : lignes « avant → après », point de l'ordre du jour, documents liés, lien vers
l'original, type de réunion relié au lexique, « Ajouter à mon agenda » qui arme « Me
rappeler 15 minutes avant ». Mon agenda signale les chevauchements sans jamais les empêcher,
et n'en signale pas avec une session annulée.
Signalements (ADR-010) : depuis une fiche — Annulée, Déplacée, Salle changée, Autre — ou
depuis la liste, « réunion non annoncée » (quoi, où, quand, thématique) ; trois gestes,
possibles hors connexion ; réservés aux négociatrices. Un administrateur valide en un geste
depuis son téléphone, avec « Annuler » six secondes, ou refuse avec un motif que l'autrice
verra. Un signalement validé s'affiche dans un encart distinct, sans nom d'auteur,
par-dessus la donnée officielle qu'il ne modifie jamais ; il se retire quand la source
l'a rattrapé. « Mes signalements » : Envoyé, Validé, Non retenu.
Notification d'un changement sur une session suivie : dans l'application et par courriel.
Hors périmètre : notifications poussées, export vers le calendrier du téléphone.
Critère : l'import tourne sur les données archivées, se coupe seul quand la source manque,
et un signalement validé s'affiche par-dessus.
```

### Étape 4 — Réunions de la Francophonie

```
/speckit-specify
Les réunions de la Francophonie dans Guide Négo : atelier préparatoire, concertation des
négociatrices et négociateurs, concertation ministérielle. Maquette : 10-francophonie.html,
section « Réunions » (écrans 1a et 1b). Onglet « Francophonie », sélecteur à deux sections
« Réunions · Pavillon » ; le titre d'écran dit toujours le nom complet de la section.
Liste et détail : lieu, heure avec fuseau, lien de visioconférence, accès limité éventuel,
inscription avec ses états — Inscrite, Liste d'attente, Complet, Terminé. Une réunion peut
porter « Se tient aussi au Pavillon » : un lien facultatif vers une activité du Pavillon,
jamais une fusion. Lisible hors connexion, avec l'heure de lecture. Les réunions du jour
entrent dans « Ma journée », chacune avec son origine.
Back-office : saisir, publier, annuler une réunion ; la lier à une activité du Pavillon.
Critère : une réunion saisie par l'IFDD apparaît, avec son lien éventuel vers le Pavillon.
```

### Étape 5 — Pavillon de la Francophonie

```
/speckit-specify
Les activités du Pavillon de la Francophonie dans Guide Négo — le stand OIF/IFDD, déjà
géré par l'ePavillon. Maquette : 10-francophonie.html, section « Pavillon » (écrans 1c et 1d).
Aucune donnée nouvelle : l'application sert telle quelle l'API publique existante du
programme et des inscriptions. Bloc de lieu en tête, activités du jour et à venir,
activités passées avec leur rediffusion et sa durée, détail d'une activité (organisateurs,
intervenants), inscription avec les états Inscrite, Liste d'attente, Complet, Rediffusion.
Lisible hors connexion ; les activités du jour entrent dans « Ma journée » avec leur
origine. Si un manque de l'API apparaît, il se corrige dans le module existant, sans copie.
Critère : l'API existante est servie telle quelle dans l'application ; le scénario qui
clôt le MVP se joue de bout en bout.
```

**Le scénario qui clôt le MVP.** Une négociatrice installe Guide Négo avec le code reçu sur WhatsApp, télécharge le guide, le lit en salle sans réseau, trouve *contact group* dans le lexique, voit les sessions de négociation du jour de ses thématiques avec leur heure de dernière lecture, et signale une annulation que l'administrateur valide depuis son téléphone.

## Après le MVP

### Étape 6a — Coquille de magasin et notifications poussées

```
/speckit-specify
Guide Négo dans les magasins d'applications, préalable aux Échanges (ADR-002, ADR-014).
La même application, emballée pour Android et iOS ; icône de l'application et écran
d'installation, qui manquent à la maquette. Dans cette coquille la session passe par jeton
et non par cookie ; l'API autorise cette origine. La personne enregistre son appareil et
reçoit des notifications poussées selon ses préférences : changement d'une session suivie,
réponse d'un expert, plus tard message et mention. Un appareil se retire à la déconnexion.
Le partage du téléphone peut viser Guide Négo. Critère : une session déplacée fait sonner
le téléphone verrouillé d'une négociatrice qui la suit (09-signaler-et-mon-agenda.html, écran 4).
```

### Étape 6 — Échanges

```
/speckit-specify
Les échanges du réseau, qui remplacent à terme le groupe WhatsApp (ADR-014) ; réservés aux
négociatrices et négociateurs. Maquette : 12-echanges.html et 13-conversation.html.
Cinquième onglet « Échanges », avec son compteur de non-lus. Canaux par thématique et par promotion, annonces de l'IFDD
sans réponse possible, canal réservé du réseau des négociatrices, conversations privées ;
non-lus, fils de réponses, mentions, une pièce jointe, trois réactions fixes — D'accord, À
retenir (qui range le message dans la liste « À retenir » du canal), Question —, retrait
de son propre message, message retiré par la modération, « Signaler ce message » à trois
motifs, envoi différé hors connexion, temps réel. Annuaire : chercher par pays,
thématique, rôle ; n'y figurent que les personnes qui l'ont choisi ; la fiche ne montre
que nom, pays, thématiques et rôle ; bloquer, signaler. Questions aux experts et leur état. « Proposer un document » en deux gestes, depuis
une conversation ou le partage du téléphone : il entre dans l'« arrivée » du corpus
(ADR-011). Mention permanente : « Échanges hébergés et modérés par l'IFDD ».
Back-office : créer et archiver des canaux, modérer, traiter les signalements.
Critère : une négociatrice suit le canal de sa thématique, reçoit une mention sur son
téléphone verrouillé, et retrouve un document partagé la veille.
```

### Étape 7 — Assistant IA

```
/speckit-specify
L'assistant de Guide Négo, réservé aux négociatrices et négociateurs. Maquette :
14-assistant.html et les files 3 et 6 de 11-validation.html. Préalable : corriger dans le
SQL la dimension des vecteurs et nommer le modèle d'origine de chaque vecteur (ADR-005).
Corpus à deux étages (ADR-011) : l'« arrivée » reçoit ce qui est proposé ou capté, avec un
classement suggéré par l'IA ; un expert promeut en « référence », seul étage lu. Sources :
documents, transcriptions horodatées des formations, FAQ. Toute source porte un état et un
« remplacée par » ; un expert pose une note de correction sur une page ou un intervalle de
vidéo ; à chaque nouveau cycle les sources repassent « à vérifier » (ADR-012).
L'assistant répond en français depuis la seule référence, cite toujours — document et
page, vidéo et intervalle, ouvrables au bon endroit —, montre la note de correction d'une
source, avertit d'une source ancienne, dit « Je ne sais pas » et propose alors un expert,
refuse sobrement de conseiller la position d'un pays. « Résumer en français » un document
fourni, marqué « Aide à la lecture — le texte anglais fait foi ». Retours « utile » et
« Dépassé ou faux » vers la file des experts. Historique, quota quotidien par personne,
état hors connexion. Le service d'IA est interne, derrière l'API (ADR-004), par OpenRouter.
Critère de qualité : un jeu de 30 à 50 questions validées par les experts, rejoué à chaque
changement de modèle ou de corpus, et dont le résultat se lit.
```

### Étape 8 — Formations et quiz

```
/speckit-specify
Les formations en vidéo et les quiz de Guide Négo. Maquette : 15-formations.html,
16-quiz.html, et la file 5 de 11-validation.html. Le modèle des formations existe.
Modules filtrés par thématique, durée, progression, supports ; lecteur vidéo avec
transcription horodatée qui défile, recherche dans la transcription, ouverture à la minute
citée par l'assistant, note de correction sur un intervalle, téléchargement « vidéo » ou
« audio seul », transcription lisible hors connexion.
Quiz à choix multiple et vrai ou faux, sur un document, une vidéo ou une thématique. Un
administrateur demande la génération par l'IA ; un expert relit question par question,
chacune avec son passage source, corrige, valide ou rejette ; le quiz se publie quand tout
est validé (ADR-013). Le passage source ne s'affiche qu'à la correction. Résultat, seuil de
réussite réglable, progression dans le temps. Une négociatrice régénère un quiz pour elle
— questions reformulées ou autres passages — : il reste privé, marqué « Non relu », sous
quota, et peut être proposé à la relecture. Un module ne porte jamais « Quiz non relu ».
Critère : un quiz généré depuis le guide est relu, publié, joué hors connexion.
```

### Étape 9 — Restitutions — à confirmer sur le terrain

```
/speckit-specify
À ne lancer qu'après les entretiens de terrain (ADR-015). Maquette : 17-restitutions.html.
Une négociatrice rédige sa restitution du jour, préremplie avec les sessions suivies, en
quatre rubriques ; l'aide de l'IA ne remplace jamais son texte sans son geste ; rédaction
hors connexion ; export. Privée par défaut ; partage à un cercle choisi — ma délégation
(les membres du réseau de mon pays), un de mes groupes de négociation, tout le réseau ;
« À remonter au ministère » ne se partage jamais. Le fil ne montre que ce qui m'est
partagé ; la synthèse automatique ne lit que ce qui est partagé au cercle où elle s'affiche.
```

## L'accord du secrétariat ne bloque rien

L'import des sessions se construit et s'éprouve **sans attendre** l'accord écrit de la CCNUCC. Tant qu'il n'est pas là, ou dès que la source manque, l'application affiche le lien vers le programme officiel : c'est son état par défaut, et l'import se rallume d'un interrupteur.

## Sur le terrain

À la première COP où l'application est installée, le porteur du projet est au stand : cinq entretiens par jour — qu'avez-vous cherché, trouvé, manqué ? Rédigez-vous une restitution, pour qui, la partageriez-vous ?

**Ce qui se mesure dès le premier jour** : installations, personnes actives par jour de COP, documents téléchargés, recherches au lexique, signalements validés, questions restées sans réponse.
