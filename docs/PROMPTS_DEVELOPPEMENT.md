# Prompts de développement — ePavillon v2

Un prompt = une session Claude Code = un écran ou un module.

**Ces prompts sont conçus pour être repris, pas seulement exécutés.** On change de session quand le contexte sature ; la session suivante ne se souvient de rien. Trois mécanismes compensent cette amnésie :

1. **`CLAUDE.md`** est chargé automatiquement au démarrage de chaque session : conventions, règles métier, interdits.
2. **`docs/MODELE_INDEX.md`** dit quels fichiers SQL lire pour la tâche du jour — le modèle va chercher lui-même, on ne recopie jamais le schéma dans un prompt.
3. **`docs/PROGRESSION.md`** porte l'état d'avancement. Chaque session le lit en arrivant et le met à jour en partant.

Conséquence pratique : **aucun prompt n'est « à exécuter une seule fois »**. Tous commencent par constater l'existant et complètent ce qui manque. Relancer A0.2 après une interruption ne détruit rien — il reprend là où l'on s'était arrêté.

---

## Le préambule

À coller en tête de **chaque** prompt de ce fichier, sans exception.

```
Lis d'abord CLAUDE.md et docs/PROGRESSION.md.

Repère dans docs/MODELE_INDEX.md les fichiers SQL qui concernent cette tâche,
et lis-les intégralement. Ils sont la source de vérité du modèle : chaque table
et chaque colonne non évidente y est commentée en français. Ne devine aucun nom
de champ, ne recopie pas un schéma de mémoire.

Constate ce qui existe déjà dans le dépôt avant de créer quoi que ce soit :
complète, ne réécris pas. Si une partie du travail demandé est déjà faite,
dis-le et passe à la suite.

À la fin, mets à jour docs/PROGRESSION.md : ligne de journal, état du prompt,
écarts constatés entre le modèle et l'interface, décisions prises en chemin.
```

---

# PHASE A — Le front sur données simulées

## A0.1 — Socle du projet

```
[PRÉAMBULE]

Crée le socle du frontend dans frontend/.

- Nuxt 4, TypeScript strict, TailwindCSS v4, Pinia, @nuxtjs/i18n (fr par défaut, en).
- app/assets/css/design-tokens.css : les jetons issus du guide de style produit
  avec Claude Design. Si le guide n'a pas encore été produit, dérive une palette
  provisoire depuis docs/CHARTE_GRAPHIQUE.md et signale-le clairement en tête du
  fichier — elle sera remplacée.
- Thème clair et sombre : palette claire sur `:root`, redéfinie sous
  `@media (prefers-color-scheme: dark)` gardé par `:root:not([data-theme="light"])`
  ET sous `:root[data-theme="dark"]`.
- Deux layouts : `public` (barre de navigation, sélecteur de langue, bascule de
  thème, pied de page) et `admin` (navigation latérale, fil d'Ariane, sélecteur
  d'événement).
- Traductions **découpées par domaine**, jamais un fichier unique :

      i18n/locales/fr/common.json      actions, états, formats, messages génériques
      i18n/locales/fr/validation.json  messages de validation de formulaire
      i18n/locales/fr/auth.json
      i18n/locales/fr/organization.json
      i18n/locales/fr/event.json
      i18n/locales/fr/proposal.json
      i18n/locales/fr/programme.json
      i18n/locales/fr/admin.json
      i18n/locales/en/…               les mêmes fichiers

  Déclare-les dans `nuxt.config.ts` avec `lazy: true` et la propriété `files`
  (tableau) de chaque locale.

  RÈGLE DE NOMMAGE : **le premier segment d'une clé est le nom de son fichier**.
  `proposal.form.speakers.title` vit dans `proposal.json`, et nulle part
  ailleurs. Ainsi on sait ouvrir un seul fichier pour trouver ou ajouter une
  clé, au lieu de charger toutes les traductions du projet.

  Quand un fichier dépasse environ 200 lignes, le scinder en sous-dossier
  (`proposal/form.json`, `proposal/review.json`) et adapter les clés.

  À ce stade, crée les fichiers avec la seule structure de `common.json` et
  `validation.json` remplie ; les autres restent des objets vides que chaque
  écran complétera.
- Utilitaires dans app/utils/ : résolution d'un texte multilingue avec repli sur
  le français, formatage d'une date AVEC son fuseau, formatage d'une plage horaire.
- Configuration de la connexion à l'API par variable d'environnement, non
  utilisée pour l'instant.

Ne crée ni composant d'interface, ni page, ni donnée simulée : c'est l'objet des
prompts suivants.
```

## A0.2 — Types dérivés du modèle

```
[PRÉAMBULE]

Crée les types TypeScript dérivés LITTÉRALEMENT du schéma SQL — mêmes noms de
champs, même nullabilité.

**UN FICHIER PAR MODULE, jamais un fichier unique.** Le modèle compte 149
tables : un `domain.ts` monolithique deviendrait impossible à charger en
contexte, et chaque écran n'a besoin que d'une poignée de types.

    frontend/types/
    ├── index.ts        ne fait que ré-exporter — aucune définition
    ├── shared.ts       I18nText, types utilitaires, alias d'identifiants
    ├── reference.ts    pays, locales, taxonomies
    ├── identity.ts     personnes, comptes, rôles, permissions
    ├── org.ts          organisations, dénominations, adhésions
    ├── event.ts        séries, éditions, journées, salles, appels, critères
    ├── programme.ts    propositions, évaluations, sessions, inscriptions
    └── views.ts        types des vues SQL consommées telles quelles

Le nom du fichier reprend celui du schéma PostgreSQL : on sait immédiatement
lequel ouvrir depuis docs/MODELE_INDEX.md.

Couvre le périmètre du jalon en cours (voir CLAUDE.md). Les autres modules
attendront leur propre fichier.

- `I18nText = { fr: string } & Record<string, string>` dans `shared.ts`
- Une union de chaînes par ENUM PostgreSQL rencontré. Va les lire dans les
  fichiers SQL ; ne les reconstitue pas de mémoire.
- Une interface par table, avec en commentaire au-dessus le nom de la table
  d'origine et le fichier SQL dont elle vient.
- Les vues `programme.v_public_schedule` et `programme.v_proposal_dashboard`
  méritent leur propre type dans `views.ts` : elles sont consommées telles
  quelles par l'interface.

Travaille fichier SQL par fichier SQL, en notant ta progression dans
docs/PROGRESSION.md au fur et à mesure. Si le contexte sature avant la fin, la
session suivante reprendra au module suivant — c'est précisément ce que ce
découpage rend possible.

Termine en listant les endroits où le modèle et ce que l'interface demanderait
naturellement divergent. Ne corrige rien : consigne-les dans PROGRESSION.md,
ils se trancheront un par un.
```

## A0.3 — Données simulées

```
[PRÉAMBULE]

Crée frontend/mocks/ : des données simulées réalistes et cohérentes entre elles.
Les identifiants doivent se répondre d'un fichier à l'autre — une proposition
pointe vers une organisation qui existe, une session vers une proposition qui
existe.

**UN FICHIER PAR DOMAINE**, nommé comme les fichiers de types, jamais un fichier
unique : 40 propositions et 60 inscriptions écrites à la main représentent
plusieurs milliers de lignes, et un écran n'en consulte qu'une partie.

    frontend/mocks/
    ├── index.ts          ré-exporte tout — aucune donnée
    ├── ids.ts            les identifiants partagés, DÉCLARÉS UNE SEULE FOIS
    │                     et importés par les autres fichiers ; c'est ce qui
    │                     garantit la cohérence entre eux
    ├── reference.ts · identity.ts · org.ts · event.ts
    ├── proposals.ts · sessions.ts · registrations.ts

Contenu attendu :

- 1 événement : COP31 Climat, Belém, 9–20 novembre 2027, hybride, fuseau
  America/Belem, pavillon tenu, avec ses journées de calendrier
- 1 appel à propositions ouvert, 6 critères d'évaluation pondérés dont un
  éliminatoire
- 3 salles dont une virtuelle, 1 canal de diffusion
- 2 journées spéciales : « Journée finance durable », « Journée jeunesse et climat »
- 12 organisations francophones FICTIVES mais crédibles, dont DEUX EN DOUBLON
  MANIFESTE : la même entité saisie une fois par son nom complet, une fois par
  son sigle. Ce doublon sert aux écrans A2 et A11, il est indispensable.
- 25 personnes, avec des rôles variés dont un administrateur limité à un seul
  événement
- 40 propositions couvrant TOUS les statuts, dont 5 co-organisées
- 30 sessions programmées, dont 2 qui se chevauchent volontairement et 2
  diffusées en même temps — ces conflits alimentent l'écran A9
- 60 inscriptions, avec des canaux d'acquisition variés

Écris ces données À LA MAIN, en français, avec de vrais intitulés d'activité
climat (« Financer l'adaptation côtière en Afrique de l'Ouest », pas
« Activité 1 »). Pas de génération aléatoire : elles seront lues cent fois et
serviront de référence visuelle pendant tout le développement.

Crée aussi composables/useApi.ts : la couche d'accès unique, qui lit aujourd'hui
les mocks et basculera vers l'API réelle par variable d'environnement. Aucune
page ne doit jamais importer un mock directement.
```

## A0.4 — Composants d'interface

```
[PRÉAMBULE]

Crée frontend/app/components/ui/ : les composants du guide de style, chacun avec
TOUS ses états (repos, survol, focus clavier, actif, désactivé, chargement).

Button, Input, Textarea, Select, Checkbox, Radio, Switch, DatePicker, Badge,
Chip, Alert, Card, Table, Modal, Drawer, Tabs, Stepper, Pagination, EmptyState,
SkeletonLoader, ErrorState, ForbiddenState.

Deux composants métier, les plus utilisés de la plateforme, à soigner
particulièrement :
- SessionCard : créneau avec fuseau, titre, organisation avec pays, pastilles
  thématiques, format, salle, jauge, état temporel (à venir / en cours / en
  direct / terminé / reporté / annulé), pastille de journée spéciale.
  Rappel : le repère « en direct » ne peut apparaître que sur UNE carte à la fois.
- StatusTimeline : frise d'avancement d'un dossier (brouillon → déposé → en
  évaluation → décision), chaque étape portant sa date.

Puis app/pages/style-guide.vue qui les montre tous, avec leurs états, sur des
données réalistes — 12 lignes de tableau, 6 cartes, pas un exemplaire de chaque.
Cette page reste dans le projet : c'est la référence pendant tout le
développement et le test de non-régression visuelle.
```

---

## A1 à A14 — Les écrans

Chaque prompt reçoit le préambule, plus ce rappel :

> *Respecte les types de `types/`, les composants de `components/ui/`, les jetons de `design-tokens.css` et la page `style-guide.vue`. Passe par `composables/useApi.ts`. Thème clair et sombre. Responsive à partir de 375 px. Les quatre états : chargement, vide, erreur, accès refusé.*
>
> *Traductions : n'ouvre que le fichier de domaine concerné (`i18n/locales/fr/<domaine>.json` et son équivalent anglais), plus `common.json` si nécessaire. Le premier segment de chaque clé est le nom de son fichier. Ne crée jamais de clé dans un fichier qui ne correspond pas à son préfixe, et n'ouvre pas les autres fichiers de traduction.*

---

**A1 — Authentification**

```
Pages d'authentification : connexion, création de compte, vérification
d'adresse, mot de passe oublié, réinitialisation.

Sobre, centré, sans image de fond. La création de compte demande le strict
minimum : prénom, nom, adresse, pays, mot de passe. Le rattachement à une
organisation vient APRÈS — c'est une étape à part entière (A2).

Détails qui comptent :
- Indicateur de robustesse du mot de passe, sans règles absurdes.
- Après inscription : « Un lien de vérification a été envoyé à … », avec renvoi
  possible au bout de 60 secondes.
- Erreurs d'authentification volontairement peu bavardes : ne jamais révéler si
  une adresse existe.
- Prévois l'emplacement de la connexion par fournisseur institutionnel et du
  second facteur, sans les implémenter.
```

**A2 — Rattachement à une organisation**

```
Écran de rattachement à une organisation, présenté après la création de compte
et accessible depuis le profil.

C'EST L'ÉCRAN LE PLUS IMPORTANT DU JALON pour la qualité des données. Le défaut
le plus coûteux de l'ancienne plateforme : deux personnes créaient deux fois la
même organisation, l'une en cherchant par nom complet, l'autre par sigle, et
rien ne permettait ensuite de les réunir.

Comportement attendu :
- Un champ unique. Dès la deuxième lettre, il interroge le référentiel et
  propose des correspondances — en tapant le NOM COMPLET comme en tapant le
  SIGLE SEUL. Les mocks contiennent volontairement une organisation saisie deux
  fois : utilise-la pour la démonstration.
- Chaque résultat montre nom, sigle, pays, nombre de membres déjà inscrits,
  sceau de vérification. Assez pour reconnaître « c'est bien la mienne ».
- Le bouton « Créer une nouvelle organisation » existe mais reste SECONDAIRE,
  et n'apparaît qu'après une recherche infructueuse.
- Si l'utilisateur insiste alors qu'une correspondance forte existe, un écran
  intermédiaire montre côte à côte ce qu'il s'apprête à créer et ce qui existe
  déjà, et propose de rejoindre. Ne le bloque pas : rends l'erreur visible.
- Si le domaine de son adresse correspond à une organisation vérifiée, propose
  le rattachement automatique.
- Formulaire de création : nom légal, sigle, type, pays, ville, site, description.

Montre les trois moments : recherche vide, résultats avec correspondance forte,
avertissement avant création.
```

**A3 — Page publique de l'événement**

```
Page publique d'une édition (COP31 Climat, Belém).

Répond à quatre questions dans l'ordre : de quoi s'agit-il, quelles sont les
échéances, que puis-je faire, où est-ce que ça se passe.

- En-tête : titre, dates, lieu, mode de participation, visuel de l'édition.
- Encart d'appel à propositions TRÈS visible tant qu'il est ouvert : échéance,
  compte à rebours, bouton de soumission, lien vers les critères. Trois états :
  à venir, ouvert, clos.
- Frise des jalons : ouverture de l'appel, clôture, annonce des résultats,
  tenue de l'événement.
- Les journées spéciales, avec leur couleur et leur description.
- Section « Programmation » : présente, indiquant que le programme sera publié
  après sélection.
- Les critères d'évaluation, publiés : une organisation doit savoir sur quoi
  elle sera jugée.
```

**A4 — Formulaire de soumission**

```
Formulaire de soumission d'une proposition d'activité — le formulaire le plus
long et le plus déterminant de la plateforme. Une organisation qui abandonne ici
ne participe pas à la COP. Il est rempli par des agents qui ne sont pas des
utilisateurs experts, souvent en plusieurs fois.

Étapes :
1. Organisation porteuse et CO-ORGANISATEURS — plusieurs organisations peuvent
   co-organiser une activité, chacune avec son rôle (co-organisateur, partenaire,
   soutien). La recherche est celle de A2.
2. Présentation : titre, résumé, objectifs, présentation détaillée, résultats
   attendus, public visé
3. Thématiques (multi-sélection), catégorie, format, langues
4. Intervenants : civilité, prénom, nom, adresse, fonction, organisation, rôle,
   photo, notice
5. Créneau souhaité et durée — avec une mention explicite : « Indiquez vos
   préférences ; l'IFDD arbitrera le calendrier définitif. » Les organisations
   proposent sans se soucier des collisions, c'est voulu.
6. Documents : présentation, note de cadrage, annexes
7. Relecture et envoi

Exigences :
- Progression visible en permanence, étapes atteignables librement.
- Enregistrement automatique en brouillon, horodatage du dernier enregistrement
  affiché sans ambiguïté.
- Compteur de caractères restants sur les champs longs.
- Erreurs regroupées en tête ET signalées sur le champ concerné.
- L'étape 7 récapitule tout, avec retour direct vers chaque étape.
- Encart permanent : échéance de l'appel et temps restant.
- Après envoi : confirmation portant le NUMÉRO DE DOSSIER (format COP31-00147),
  copiable, avec la suite des opérations.

Montre l'étape 1 avec un co-organisateur ajouté, et l'étape 4.
```

**A5 — Espace organisation**

```
Espace personnel d'une organisation : suivi de ses dossiers.

Répond en un coup d'œil à : où en est chacun de mes dossiers, et qu'est-ce qui
attend une action de ma part ?

- Frise d'avancement par dossier, chaque étape portant sa date.
- L'état « corrections demandées » saute aux yeux, avec le nombre de points à
  traiter.
- Les commentaires du comité partagés avec le soumissionnaire s'affichent en fil
  de discussion, avec réponse et marquage « résolu ».
- Un dossier accepté montre ses sessions programmées, leur créneau, le nombre
  d'inscrits.
- Onglet « Historique » : toutes les modifications du dossier, champ par champ,
  avec auteur et date. L'ancienne plateforme ne le permettait pas.
- Section « Membres de l'organisation », avec invitation par adresse.
- État vide soigné : aucune proposition, avec l'appel en cours mis en avant.
```

**A6 — Tableau de bord du back-office**

```
Page d'accueil du back-office, pour l'équipe de l'IFDD dans les semaines qui
précèdent une COP.

Trois zones, dans cet ordre :
1. CE QUI DEMANDE UNE ACTION — propositions sans évaluation à l'approche de
   l'échéance, revues en retard, doublons d'organisations à trancher, conflits
   de créneaux non résolus, incidents actifs. Chaque ligne mène à l'écran
   concerné. Bloc le plus haut et le plus visible.
2. LES CHIFFRES — entonnoir des propositions, courbe des soumissions par jour
   avec l'échéance marquée, courbe des inscriptions, répartition par pays et par
   thématique.
3. SANTÉ OPÉRATIONNELLE — file d'événements en attente, travaux en échec,
   courriels en rebond, synchronisations visio en erreur. Trois niveaux de
   gravité.

Contrainte : le premier bloc doit rester lisible quand il est vide. Un
back-office où tout va bien ne doit pas ressembler à un écran cassé.

Sélecteur d'événement en tête de page. Important : un administrateur peut
n'avoir accès qu'à UN SEUL événement — le sélecteur n'affiche alors que le sien,
et rien dans la page ne laisse deviner l'existence des autres.

Graphiques sobres : pas de 3D, pas de dégradé, une couleur par série, légende
directe sur la courbe.
```

**A7 — Liste des propositions**

```
Liste des propositions reçues, back-office.

Tableau dense — les 40 lignes des mocks, pas 5. Colonnes : numéro de dossier,
titre, organisation porteuse (avec pastille « +2 » si co-organisée), pays,
thématiques, format, statut, avancement des revues (2/3), note moyenne, rang,
date de dépôt.

- Tri sur chaque colonne, par défaut sur la note décroissante.
- Filtres : statut, thématique, format, pays, organisation, révisionniste
  assigné, « non évaluées », « en retard ».
- Sélection multiple et actions groupées : assigner à un révisionniste, changer
  de statut, exporter.
- Ligne cliquable ouvrant la fiche (A8).
- Recherche plein texte, export CSV.
- Indicateur discret des dossiers non encore consultés par l'utilisateur courant.
```

**A8 — Fiche d'évaluation d'une proposition**

```
Fiche d'évaluation d'une proposition, back-office. C'est ici qu'un membre du
comité décide. Tout doit se faire sans quitter la page.

Deux colonnes.

GAUCHE (lecture, largeur dominante) : le dossier complet — organisation porteuse
et co-organisateurs avec leur historique de participation, présentation,
objectifs, thématiques, intervenants, créneau souhaité, documents consultables,
onglet Historique des modifications.

DROITE (panneau d'évaluation, collant au défilement) :
- Grille par critères pondérés, lue depuis les mocks : chaque critère porte son
  poids, son maximum et son caractère éliminatoire. Un critère éliminatoire noté
  0 produit un avertissement net.
- Total pondéré et conversion sur 20 recalculés en direct.
- Recommandation : retenir / retenir avec modifications / neutre / rejeter.
- Points forts, points faibles.
- Bouton de déport, en déclarant un lien avec l'organisation.

EN-TÊTE : numéro de dossier, avancement du comité, note moyenne, rang, et les
actions de décision (retenir / demander des corrections / rejeter) — réservées à
ceux qui en ont le droit, donc visuellement séparées de la notation.

SOUS LE PANNEAU : les commentaires, avec une distinction TRÈS claire entre
« interne au comité » et « partagé avec le soumissionnaire ». Se tromper de
visibilité est le risque principal de cet écran : rends l'erreur difficile —
fond différent, libellé explicite, confirmation au premier envoi partagé.

Montre l'écran en mode évaluation en aveugle : tant que je n'ai pas soumis ma
note, je ne vois pas celles des autres.
```

**A9 — Planificateur de créneaux**

```
Planificateur de la programmation, back-office. L'équipe place les activités
retenues dans les salles, sur les jours de l'édition.

Panneau latéral gauche : les activités retenues non encore placées, filtrables
et triables par note. Zone principale : calendrier salle × heure, glisser-déposer.
Utilise vue-cal ou un équivalent.

RÈGLE CENTRALE, à respecter absolument : les chevauchements NE SONT PAS BLOQUÉS.
Les organisations ont proposé leurs créneaux sans se coordonner ; c'est l'équipe
qui réorganise. Un dépôt sur un créneau occupé fonctionne, et le conflit devient
visible. On n'empêche pas, on montre.

Conflits sur deux niveaux :
- BLOQUANT (rouge) : deux activités de la MÊME ÉDITION en même temps — l'IFDD
  tient un seul stand ; deux diffusions en direct simultanées — une seule équipe
  technique ; une salle physique réservée deux fois.
  Attention : deux activités de DEUX ÉVÉNEMENTS DIFFÉRENTS peuvent se tenir en
  parallèle sans conflit, sauf pour la diffusion.
- AVERTISSEMENT (orange) : un intervenant attendu à deux endroits, une
  organisation programmée deux fois.
Bandeau permanent recensant les conflits, avec compteur et lien vers chaque cas.
Blocs en conflit marqués dans le calendrier.

Autres comportements :
- Redimensionnement d'un bloc pour ajuster la durée.
- Rattachement d'une activité placée à une journée spéciale par un sélecteur.
  Ce rattachement est MANUEL et indépendant de la date : toutes les activités du
  jour n'en font pas partie.
- Marquer une session comme diffusée, avec son canal.
- Compteur « 12 activités restant à placer ».
- Bouton « Publier la programmation », ouvrant un récapitulatif de ce qui doit
  être réglé avant publication. C'est le SEUL endroit où un contrôle bloquant a
  du sens.

Vue mobile : le glisser-déposer n'a pas de sens, prévois une affectation par
sélection en deux temps.
```

**A10 — Gestion des événements**

```
Écrans de gestion des événements, back-office.

1. Liste des éditions : titre, série, année, dates, lieu, statut, nombre de
   propositions, programmation publiée ou non.
2. Création et édition d'une édition : série de rattachement, libellé, année,
   titre, description, dates, fuseau, pays, ville, adresse, mode de
   participation, pavillon tenu ou non, visuels.
3. Onglets de l'édition :
   - Journées du calendrier, générées depuis les dates
   - JOURNÉES SPÉCIALES : création d'un fil thématique — titre, sous-titre,
     couleur, période indicative, responsable, page publique. La composition se
     fait depuis le planificateur (A9), pas ici.
   - Lieux et salles : physique ou virtuelle, capacité, équipement
   - Canal de diffusion : fournisseur, compte, canal par défaut
   - Appel à propositions : UN SEUL par édition — ouverture, clôture,
     prolongation, plafond par organisation, nombre de revues exigé, évaluation
     en aveugle, grille de critères pondérés avec critères éliminatoires
   - Comité de sélection : composition, plafond de charge par membre
```

**A11 — Organisations et fusion des doublons**

```
Gestion des organisations, back-office, avec l'outil de fusion.

1. LISTE : nom, sigle, pays, type, membres, propositions déposées et acceptées,
   RATIO D'ACCEPTATION DE LEURS ACTIVITÉS, score de confiance, sceau de
   vérification. Triable par score de confiance croissant, pour traiter en
   priorité les fiches douteuses.

2. FILE DES DOUBLONS PRÉSUMÉS : paires détectées, triées par similarité
   décroissante, avec le MOTIF de la suspicion — similarité de nom, domaine de
   courriel partagé, même pays, correspondance de sigle.

3. ÉCRAN DE FUSION, en vue comparée côte à côte :
   - Les deux fiches en regard, champ par champ, écarts mis en évidence
   - Choix de la valeur à conserver pour chaque champ divergent
   - Décompte de ce qui sera transféré : membres, propositions, co-organisations,
     sessions, dénominations
   - Désignation explicite de la fiche qui absorbe l'autre
   - Rappel que la fiche absorbée reste consultable et que ses anciennes adresses
     continueront de fonctionner
   - Motif obligatoire
   - Confirmation exigeant de saisir le nom de l'organisation absorbée
   - Action « ce ne sont pas des doublons », qui retire la paire de la file

4. Fiche d'une organisation : dénominations (nom, sigle, traductions, anciens
   noms), domaines de courriel et leur vérification, membres, activités,
   historique.
```

**A12 — Utilisateurs et rôles**

```
Gestion des utilisateurs et des rôles, back-office.

Liste : nom, adresse, organisation, pays, rôles avec leur portée, dernière
connexion, statut.

Panneau d'attribution de rôle avec PORTÉE — le point central de cet écran.
L'interface doit rendre évidente la différence entre :
  « Administrateur »                 → toute la plateforme
  « Administrateur de la COP31 »     → cette édition uniquement
  « Révisionniste de la COP31 »      → le comité de cette édition
  « Référent de l'organisation X »   → cette organisation

Le cas réel : confier un webinaire à un responsable qui ne doit voir que son
événement. Dans l'ancienne plateforme, cela avait imposé de développer une page
d'administration séparée, dans l'urgence. Ici c'est une attribution de rôle.

Prévois : date de fin facultative, motif, historique des attributions et
révocations, et un écran montrant les permissions effectives d'une personne
(« voici ce que cette personne peut faire, et où »).

Écrans annexes : suspension et blocage d'un compte avec motif, demandes RGPD.
```

**A13 — Messages d'incident**

```
Gestion des messages d'incident, back-office.

Il arrive qu'une activité déborde sur la suivante, qu'un intervenant soit en
retard, qu'une panne technique interrompe une diffusion. Il faut informer les
spectateurs sans délai.

Formulaire de publication :
- Portée : toute la plateforme / un événement / une journée / une session /
  une organisation
- Nature : retard, débordement sur le créneau suivant, panne technique,
  annulation, changement de salle, information
- Gravité : information (bleu), avertissement (orange), erreur (rouge)
- Message en français ET en anglais
- Fenêtre d'affichage : immédiat ou programmé, avec fin automatique
- Aperçu en direct du bandeau tel qu'il apparaîtra au public

Liste des incidents : actifs en premier, dépublication en un clic, historique.

Prévois un raccourci « Signaler un débordement » depuis la fiche d'une session
en cours, qui pré-remplit le formulaire.
```

**A14 — Modules en attente**

```
Page « En cours de maintenance » pour les modules hors périmètre : Publications,
Négociations, Formations, Outils, Messagerie, Annuaire.

Une seule page réutilisable, paramétrée par le nom du module. Sobre et honnête :
indiquer que le module arrive, sans date fantaisiste, avec un renvoi vers ce qui
est disponible. Pas d'illustration de chantier ni de robot.

Le routage doit la servir automatiquement quand le drapeau de fonctionnalité du
module est désactivé, sans toucher aux pages elles-mêmes.
```

---

# PHASE B — L'API avec GitHub Spec Kit

Une fois le front stabilisé sur les mocks, les écarts sont connus et le contrat d'API se déduit de ce qui est réellement affiché.

## B0 — Constitution

```
/speckit.constitution

Projet : ePavillon v2, API Rust + Actix Web pour l'IFDD.
Lis CLAUDE.md : les principes qui suivent en découlent et ne doivent pas le
contredire.

Principes non négociables :
1. docs/database/ est la source de vérité. Aucune table, aucune colonne n'est
   créée sans y être ajoutée d'abord.
2. Un module = un schéma PostgreSQL = un crate. Un crate de module ne dépend
   jamais d'un autre crate de module : uniquement de `kernel` et des contrats
   d'événements.
3. Toute clé étrangère traversant deux schémas métier est nommée `xmod_fk_*`.
   `SELECT * FROM platform.cross_module_fk_report WHERE NOT is_compliant`
   doit rester vide.
4. Les effets de bord inter-modules passent par platform.outbox_events, dans la
   même transaction que le changement d'état. Jamais d'appel direct.
5. L'autorisation se teste par PERMISSION (identity.has_permission), jamais par
   nom de rôle, et toujours avec sa portée. Toute liste du back-office est
   filtrée par identity.administered_events : un administrateur d'événement ne
   doit atteindre aucune donnée d'une autre édition, y compris en forgeant une URL.
6. SQLx avec vérification à la compilation. Pas d'ORM.
7. Toute écriture positionne app.actor_id et app.request_id en début de
   transaction — c'est ce qui alimente l'audit et l'historique.
8. Les invariants portés par la base ne sont pas réimplémentés : le code traduit
   l'erreur PostgreSQL en message français exploitable par l'interface.
9. Les erreurs d'API portent un code stable et un message français.
10. Tests d'intégration sur base réelle (conteneur jetable chargé depuis
    docs/database/), pas de mocks de base de données.
```

## B1 à B6 — Les modules

Un module = un cycle complet : `/speckit.specify` → `/speckit.clarify` → `/speckit.plan` → `/speckit.tasks` → `/speckit.implement`.

Ordre imposé par les dépendances :

| # | Module | Contenu |
|---|--------|---------|
| B1 | **Socle + Identité** | Kernel (contexte de requête, erreurs, i18n, bus d'événements), relais d'outbox, file de travaux. Authentification Argon2id, jetons d'accès courts, jetons de rafraîchissement hachés et révocables, vérification d'adresse, réinitialisation. RBAC scopé et filtrage par périmètre d'administration. |
| B2 | **Organisations** | Recherche multi-signaux, création avec détection de doublon, rattachement par domaine vérifié, adhésions, détection continue, fusion. Point délicat : la recherche doit répondre en moins de 150 ms sur 5 000 organisations, avec anti-rebond côté client. |
| B3 | **Événements** | Séries, éditions, journées, journées spéciales, salles, canal de diffusion, appel unique par édition, grille de critères, comité. Publication de la programmation avec contrôle préalable. |
| B4 | **Propositions** | Soumission avec brouillon, co-organisateurs, intervenants, documents. Machine à états : le code ne réimplémente pas les transitions, il lit `programme.proposal_transitions_allowed`. Évaluation, scores, commentaires à visibilité contrôlée, historique. |
| B5 | **Sessions** | Création depuis une proposition acceptée, planification, détection de conflits, rattachement aux journées spéciales, publication, inscriptions avec formulaire configurable. |
| B6 | **Média + Engagement** | Téléversement vers Garage avec déduplication par empreinte, génération des variantes en tâche de fond, quotas. Notifications, modèles multilingues, rappels sans doublon. |

**Modèle de prompt** :

```
/speckit.specify

Module <NOM> de l'API ePavillon v2 (Rust + Actix Web + SQLx).

Lis CLAUDE.md et docs/PROGRESSION.md. Repère dans docs/MODELE_INDEX.md les
fichiers SQL de ce module et lis-les intégralement : le schéma existe déjà, il
fait autorité. Ne propose aucune modification sans la justifier explicitement.

Le front existe et consomme des données simulées : lis les fichiers de
frontend/types/ et frontend/mocks/ correspondant à CE module uniquement.
Le contrat d'API doit servir CE front, sans
renégociation des noms de champs. Les écarts déjà consignés dans PROGRESSION.md
sont à traiter, pas à contourner.

Fonctionnalités attendues : <contenu du tableau ci-dessus>

Livrable : un crate crates/modules/<nom> exposant domaine, dépôts, service et
routes, plus la documentation OpenAPI générée.
```

## B7 — Raccordement du front

```
[PRÉAMBULE]

Bascule le frontend des données simulées vers l'API réelle.

- Génère le client TypeScript depuis l'OpenAPI (`openapi-typescript`) dans
  frontend/types/api.ts.
- Compare-le aux types de frontend/types/ et LISTE LES ÉCARTS avant de modifier
  quoi que ce soit. Chaque écart est soit un défaut du front, soit un défaut de
  l'API : tranche l'un après l'autre, ne les masque pas par des conversions.
- Fais basculer composables/useApi.ts sur l'API réelle, pilotée par variable
  d'environnement. Les mocks restent en place : ils servent aux tests et au
  développement hors ligne.
- Gestion des erreurs : messages français, reconnexion sur expiration de jeton,
  mode dégradé si l'API est injoignable.
```

---

## Ordre d'exécution

```
  A0.1 socle ─ A0.2 types ─ A0.3 mocks ─ A0.4 composants
   │
   ├── A1 authentification ─── A2 rattachement organisation
   │                                   │
   ├── A3 page événement ───────────────┼─── A4 soumission ─── A5 espace organisation
   │                                   │
   └── A6 tableau de bord ── A7 liste ──┴─ A8 évaluation ── A9 planificateur
                              A10 événements · A11 organisations · A12 rôles
                              A13 incidents · A14 maintenance
   │
   ▼
  B0 constitution
   │
  B1 socle + identité ─ B2 organisations ─ B3 événements ─ B4 propositions
   │                                                          │
   └────────────────── B5 sessions ─ B6 média + engagement ────┘
   │
   ▼
  B7 raccordement
```

Les deux écrans à traiter en premier après le socle, parce qu'ils portent les décisions dont tout le reste hérite : **A2** — la qualité du référentiel d'organisations se joue là — et **A8**, l'écran le plus dense ; s'il tient, les autres tiennent.

## Avant de passer en phase B

- [ ] Toutes les pages du jalon existent et fonctionnent sur les mocks.
- [ ] Les écarts consignés dans `PROGRESSION.md` sont tranchés — dans le SQL si le modèle a tort, dans le front sinon.
- [ ] Les quatre états sont traités partout : chargement, vide, erreur, accès refusé.
- [ ] Le thème sombre tient sur chaque page.
- [ ] Rien n'est en dur : aucune chaîne hors i18n, aucune couleur hors jetons, aucune route de journée spéciale écrite en clair.
- [ ] Le parcours complet est jouable sur les mocks : créer un compte, rejoindre une organisation sans créer de doublon, soumettre une proposition à plusieurs organisations, la noter, la retenir, la programmer, publier le programme, s'y inscrire.
