# Feature Specification: Événements (B3)

**Feature Branch**: `003-evenements`

**Created**: 2026-08-20

**Status**: Draft

**Input**: User description: "Module ÉVÉNEMENTS de l'API ePavillon v2 (Rust + Actix Web + SQLx). Séries, éditions, journées, journées spéciales, salles, canal de diffusion, appel unique par édition, grille de critères, comité. Publication de la programmation avec contrôle préalable. Le sigle d'une édition reste facultatif en base et devient obligatoire, dans le service, pour une édition tenant un pavillon (écart n° 9). Le périmètre d'administration borne AUSSI ce module. Livrable : `backend/crates/modules/event`, monté par `api` et `worker`, qui existent depuis B1."

---

## Contexte

**Ce module est celui qui permet d'ouvrir l'appel à propositions de la COP31.** Sans édition, pas d'appel ; sans appel, pas de dossier ; sans grille, pas d'évaluation. B4 (propositions) et B5 (sessions) reposent entièrement sur ce que B3 pose.

Le modèle fait autorité et **n'est pas modifié** : `docs/database/060_events.sql` (séries, éditions, calendrier, fils de programmation, lieux, salles, canaux, appels, grille, comité, vue des éditions publiques), `docs/database/030_identity.sql` § 6 (les deux permissions `event.*` et la fonction de périmètre), `docs/database/075_programme_sessions.sql` § 7 (détection des conflits et contrôle avant publication, lus et jamais réécrits), `docs/database/020_reference.sql` (thématiques d'un fil), `docs/database/050_media.sql` (les trois déclinaisons d'image, résolues en lecture).

Deux décisions structurantes du modèle gouvernent tout ce qui suit, et elles sont écrites en tête du fichier SQL : **la série porte l'identité durable, l'édition porte l'occurrence** — c'est ce qui rend « combien d'organisations reviennent d'une COP à l'autre ? » possible ; et **l'appel à propositions est une entité, en cardinalité 0..1**, parce que « quand il n'y a pas de stand, on ne fait pas d'appel à propositions ». La cardinalité est tenue par un index unique, pas par l'application.

Le front existe depuis le 18/08 et consomme des données simulées. Ses contrats — `frontend/app/types/admin-events.ts`, `frontend/app/types/event/{series,edition,venue,call}.ts` — et les chemins déclarés dans `frontend/app/composables/api/admin-events.ts`, `composables/api/planner.ts` et le bloc `events` de `composables/useApi.ts` **sont le contrat de cette API**. Ils ne se renégocient pas. Cinq écrans en dépendent : la page publique d'une édition (A3), l'accueil et sa frise d'historique (A15), le tableau de bord (A6), le planificateur (A9) et les trois pages de gestion des événements (A10).

Le socle existe depuis B1 et les frontières depuis B2 : `kernel` (contexte de requête, erreurs à code stable, unique porte d'écriture, garde d'autorisation testant permission **et** portée, file de travaux, jetons, contrat d'envoi de courriel), `contracts`, `api`, `worker`. **Ce module ne les réinvente pas et ne dépend d'aucun autre crate de module.** La règle de frontière posée en B2 s'applique telle quelle : *un module lit hors de son schéma quand la question porte sur ses propres entités, il n'y écrit jamais, et il n'appelle jamais un autre module.*

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Une édition existe, avec un numéro de dossier qu'on peut épeler (Priority: P1)

L'équipe de l'IFDD crée l'édition d'une COP : sa série, son millésime, ses dates, son fuseau, sa ville, et le fait qu'elle tienne ou non un pavillon. Elle la modifie ensuite au fil des annonces. Le calendrier des journées suit les dates sans qu'on ait à le composer à la main.

**Et si l'édition tient un pavillon, elle porte un sigle.** Le sigle préfixe le numéro de dossier communiqué aux organisations : sans lui, la base retombe sur les huit premiers caractères de l'adresse d'URL et produit « COP31-TE-00001 », qu'il faut épeler lettre à lettre au téléphone. La contrainte vit dans le service et non dans la base, parce que les webinaires du cycle PACO n'ont pas de sigle et doivent continuer d'exister.

**Why this priority**: rien de tout le jalon ne commence sans une édition. C'est l'entité dont B4 et B5 dépendent, et le premier endroit où l'appel se pose.

**Independent Test**: créer une édition avec pavillon sans sigle et constater le refus **nommant le champ et proposant une valeur** ; la créer avec sigle et constater qu'elle apparaît dans la liste avec son calendrier ; créer une édition sans pavillon sans sigle et constater qu'elle est acceptée.

**Acceptance Scenarios**:

1. **Given** une personne détenant la gestion des événements sur l'ensemble de la plateforme, **When** elle crée une édition avec pavillon **sans** sigle, **Then** la création est refusée, le champ du sigle est désigné, et la réponse porte **une valeur par défaut proposée**, modifiable.
2. **Given** la même personne, **When** elle crée une édition **sans** pavillon et sans sigle, **Then** l'édition est créée : la règle ne vaut que pour les éditions tenant un pavillon.
3. **Given** une édition existante sans pavillon et sans sigle, **When** on la bascule en « pavillon tenu » sans fournir de sigle, **Then** la modification est refusée de la même façon — la règle se vérifie à la **modification** autant qu'à la création.
4. **Given** un sigle de 1 caractère, de 13 caractères, ou contenant un espace ou un accent, **When** on l'envoie, **Then** il est refusé sur son champ : de 2 à 12 caractères, lettres, chiffres et tiret.
5. **Given** une édition dont l'adresse d'URL est déjà prise, **Then** le refus le dit sur le champ de l'adresse et non sur l'ensemble du formulaire.
6. **Given** une édition en présentiel ou hybride sans pays ni ville, **Then** le refus dit que les deux sont exigés dès que l'édition n'est pas entièrement en ligne.
7. **Given** une latitude sans longitude, **Then** le refus dit qu'un point se donne en entier ou pas du tout.
8. **Given** une période élargie, **When** l'édition est enregistrée, **Then** les journées manquantes du calendrier sont créées, la réponse annonce combien, et **aucune journée existante n'est supprimée**.

---

### User Story 2 — Le périmètre d'administration borne aussi les événements (Priority: P1)

Un responsable détaché sur un seul webinaire ouvre le back-office. Il voit son édition, ses journées, ses salles, son canal, son appel — et rien d'autre. Une personne sans aucun droit d'administration reçoit un refus, pas une page vide.

**Why this priority**: c'est la règle métier n° 8, et ce module est le premier où le périmètre porte sur l'entité qui le définit. Un défaut ici ouvre toutes les éditions de la plateforme à qui n'en administre qu'une.

**Independent Test**: se connecter comme administrateur détaché sur une édition, lister, ouvrir l'édition confiée, puis forger l'identifiant d'une autre dans l'URL sur chacune des routes ; recommencer avec un compte sans aucun droit.

**Acceptance Scenarios**:

1. **Given** un compte administrant l'ensemble de la plateforme, **When** il liste les éditions, **Then** toutes remontent et la réponse le dit.
2. **Given** un compte détaché sur une seule édition, **When** il liste, **Then** seule cette édition remonte.
3. **Given** un compte **sans aucun** droit d'administration, **When** il liste, **Then** il reçoit un **refus d'accès explicite**, jamais une liste vide.
4. **Given** un compte détaché sur une édition, **When** il demande le détail, les journées, les fils, les lieux, les salles, les canaux, l'appel ou le comité d'une **autre** édition en forgeant l'identifiant, **Then** chaque route refuse, et le refus ne se distingue pas de celui d'un identifiant inexistant.
5. **Given** le même compte, **When** il tente d'écrire sur une édition hors périmètre — journée, fil, lieu, salle, canal, appel, comité —, **Then** l'écriture est refusée sur chacune de ces routes.
6. **Given** un compte détaché sur une édition, **When** il tente de **créer** une édition, **Then** la création est refusée : une édition qui n'existe pas encore n'a aucun périmètre où vérifier un droit, la création exige la portée globale.

---

### User Story 3 — L'appel unique s'ouvre avec sa grille, et jamais sans (Priority: P1)

L'équipe ouvre l'appel à propositions de l'édition : sa fenêtre, ses règles de recevabilité, la plage d'accueil du pavillon, et sa grille de critères pondérés. La grille par défaut est proposée depuis la base, puis ajustée. Une prolongation se pose sans effacer l'échéance annoncée aux organisations.

**Why this priority**: c'est la raison d'être du jalon. Un appel sans grille ne peut recevoir aucune évaluation, et un appel en double n'existe pas.

**Independent Test**: ouvrir un appel avec sa grille sur une édition qui n'en a pas, vérifier qu'un second est refusé, prolonger, et constater que l'échéance effective bouge sans que l'échéance initiale soit perdue.

**Acceptance Scenarios**:

1. **Given** une édition sans appel, **When** on enregistre un appel avec sa grille, **Then** l'appel et ses critères sont créés **d'un seul geste** ; un échec sur la grille ne laisse aucun appel.
2. **Given** une édition portant déjà un appel non annulé, **When** on tente d'en créer un second, **Then** le refus le dit clairement, et l'écran n'a rien à deviner.
3. **Given** une édition dont l'appel a été **annulé**, **When** on en crée un nouveau, **Then** il est accepté : l'annulé reste à l'historique.
4. **Given** une grille vide, **When** on enregistre, **Then** le refus dit qu'aucun dossier ne pourrait être évalué.
5. **Given** deux critères portant le même code, **Then** le refus désigne **le rang de la ligne fautive** dans la grille.
6. **Given** une clôture antérieure à l'ouverture, une prolongation antérieure à la clôture, une durée par défaut hors des bornes, ou une fermeture de pavillon antérieure à son ouverture, **Then** chacun de ces refus porte son propre code et son propre champ.
7. **Given** un appel dont des notes ont déjà été posées, **When** on modifie un barème, **Then** l'enregistrement aboutit et la réponse **prévient** que des moyennes déjà calculées vont se déplacer.
8. **Given** une demande de grille par défaut, **When** elle arrive, **Then** les six critères et leurs poids viennent **de la base** et ne sont recopiés nulle part.

---

### User Story 4 — Le public voit une édition, ses échéances et son visuel (Priority: P1)

Un visiteur ouvre la page d'une édition ou la frise d'historique de l'accueil. Il y lit la série, le pays, les dates dans le fuseau de l'édition, l'état temporel, les trois déclinaisons d'image et l'état de l'appel avec son échéance effective. C'est là qu'il dépose un dossier.

**Why this priority**: c'est la porte d'entrée des organisations, et le seul écran public de ce module. Deux écarts consignés (n° 25 et n° 26) s'y referment.

**Independent Test**: demander les éditions publiques et le détail d'une édition annoncée, sans session ; vérifier que le brouillon et l'annulée n'y sont pas, et que la réponse embarque ses images et son appel **sans appel supplémentaire**.

**Acceptance Scenarios**:

1. **Given** des éditions en brouillon, annoncées, en cours, terminées, annulées et suspendues, **When** on demande les éditions publiques, **Then** le brouillon et l'annulée sont **absentes**, et toutes les autres présentes — **une édition annoncée dont le programme n'est pas publié en fait partie**.
2. **Given** une édition publique, **When** on demande sa page par son adresse d'URL, **Then** la réponse porte sa série, son pays, ses **trois** déclinaisons d'image résolues, son état temporel et son appel avec son échéance effective — en **une seule** réponse.
3. **Given** une édition en brouillon ou annulée, **When** on demande sa page publique, **Then** elle est introuvable, comme une édition qui n'existerait pas.
4. **Given** une édition dont aucune image n'a été téléversée, **When** on demande sa page, **Then** les trois déclinaisons valent « rien » et l'édition reste publique.
5. **Given** une édition hors série, **When** on demande les éditions publiques, **Then** elle y figure : une édition sans série ne disparaît pas de l'historique.
6. **Given** un appel prolongé, **When** on lit l'édition, **Then** c'est l'échéance **effective** qui est rendue, et l'état « ouvert » tient compte de la fenêtre et non du seul statut.

---

### User Story 5 — Le stand a ses salles, et le direct son canal (Priority: P2)

L'équipe déclare le pavillon, ses salles et leurs équipements, puis le canal de diffusion par défaut de l'édition. Une salle virtuelle est marquée comme telle. Un seul canal peut être le canal par défaut.

**Why this priority**: sans salle, un conflit de créneau ne peut pas être **nommé** — c'était le défaut de la v1. Sans canal par défaut, une séance diffusée n'occupe rien et échappe à la règle du direct unique. Les deux servent B5 ; ils ne bloquent pas l'ouverture de l'appel.

**Independent Test**: créer un lieu et deux salles dont une virtuelle, poser un canal par défaut, en poser un second et vérifier que le premier cesse de l'être, puis retirer un canal qui a servi.

**Acceptance Scenarios**:

1. **Given** un lieu et ses salles, **When** on retire une salle, **Then** les séances qui s'y tenaient **survivent** et perdent leur salle, et la réponse dit combien.
2. **Given** deux salles du même lieu portant le même code, **Then** la seconde est refusée sur son champ.
3. **Given** un canal déjà par défaut, **When** on en désigne un second par défaut, **Then** le premier cesse de l'être **dans la même écriture**, sans que personne ait à le décocher.
4. **Given** un canal sur lequel des séances ont été diffusées, **When** on le retire, **Then** il est **désactivé et non supprimé**, la réponse le dit, et c'est un succès et non un refus.
5. **Given** un canal général de la plateforme, non rattaché à une édition, **When** on tente de le modifier depuis une édition, **Then** l'écriture est refusée.
6. **Given** un lieu retiré, **When** l'écriture aboutit, **Then** ses salles disparaissent avec lui et les séances qu'elles portaient sont **détachées, jamais supprimées**, la réponse chiffrant le détachement.

---

### User Story 6 — Les journées du calendrier et les journées spéciales ne se confondent pas (Priority: P2)

L'équipe habille les jours du calendrier — titre, page dédiée, couleur, mise en avant — et compose à part les journées spéciales, qui ne sont pas des jours : une matinée, deux fils le même jour, un séminaire à cheval sur trois jours.

**Why this priority**: c'est la distinction que la v1 n'a pas su exprimer et qu'elle a payée par des journées codées en dur dans le routeur. Elle est déjà tenue par le modèle ; l'API ne doit pas la défaire.

**Independent Test**: demander le plan de génération du calendrier avant de l'exécuter, créer un fil, l'habiller de thématiques, ouvrir sa page publique, puis le supprimer et constater le nombre de rattachements perdus.

**Acceptance Scenarios**:

1. **Given** une édition dont la période a été resserrée, **When** on demande le plan de génération, **Then** il annonce les jours à créer, les jours **hors période** avec le nombre de séances qu'ils portent, et ceux que rien ne change — **sans rien écrire**.
2. **Given** ce plan, **When** on lance la génération **sans** demander le retrait des jours hors période, **Then** rien n'est supprimé.
3. **Given** ce plan, **When** on lance la génération **en** demandant le retrait, **Then** les jours hors période partent, leurs séances sont détachées sans être supprimées, et la réponse chiffre le détachement.
4. **Given** une journée générée, **Then** elle porte sa date et **aucun contenu éditorial** : pas de titre inventé.
5. **Given** une journée déjà habillée, **When** on relance la génération, **Then** son titre, son adresse, sa couleur et sa mise en avant sont **intacts**.
6. **Given** un fil de programmation, **When** on l'enregistre, **Then** son code et son adresse sont uniques **au sein de son édition** et les refus le disent ; ses thématiques sont enregistrées, et sa page publique s'ouvre ou se referme d'un même geste.
7. **Given** un fil composé de séances, **When** on le supprime, **Then** les séances **survivent** et la réponse dit combien de rattachements éditoriaux sont perdus.
8. **Given** un fil dont la fin précède le début, **Then** le refus le dit — la période d'un fil est indicative, mais elle reste cohérente.

---

### User Story 7 — Le comité se compose sans ouvrir de droits (Priority: P2)

L'équipe désigne qui siège au comité de sélection de l'appel, qui le pilote, et le nombre de dossiers que chacun peut prendre. La composition se dit ici ; le **droit** d'évaluer, lui, reste une attribution de rôle.

**Why this priority**: le comité conditionne l'évaluation (B4), pas la soumission. Mais l'ambiguïté « siéger = pouvoir lire » est un piège que l'écran a déjà signalé.

**Independent Test**: enregistrer une composition d'un seul geste — ajouts, retraits et plafonds —, vérifier que la réponse nomme les membres retirés qui portaient encore des dossiers, et qu'un membre ajouté sans le rôle d'évaluateur est **signalé** et non doté.

**Acceptance Scenarios**:

1. **Given** une composition envoyée d'un seul geste, **When** elle est enregistrée, **Then** les ajouts, les retraits et les plafonds sont appliqués **ensemble** ; un échec ne laisse aucune moitié.
2. **Given** un membre retiré qui portait encore des dossiers, **Then** la réponse **le nomme** avec le nombre de dossiers concernés, et ses évaluations déjà rendues restent au dossier.
3. **Given** un membre ajouté qui ne détient pas la permission d'évaluer sur cette édition, **Then** il est ajouté et la réponse **le signale** : siéger n'accorde rien.
4. **Given** une personne inexistante ou hors du personnel proposable, **Then** l'ajout est refusé.
5. **Given** une composition sans aucun pilote, **Then** elle est acceptée : le pilote est facultatif.
6. **Given** une personne détenant la gestion des événements mais **pas** celle des appels, **When** elle tente d'écrire le comité ou l'appel, **Then** l'écriture est refusée — les deux permissions sont distinctes.

---

### User Story 8 — La programmation ne se publie pas avec un conflit ouvert (Priority: P3)

Avant de rendre le programme public, l'équipe voit ce qui reste à régler : deux activités simultanées sur un stand unique, une salle réservée deux fois, deux directs en même temps, une séance sans créneau, sans lieu, sans intervenant. Les points **bloquants** retiennent la publication ; les avertissements l'accompagnent.

**Why this priority**: c'est le seul contrôle bloquant de toute la chaîne, et il ne sert qu'une fois les séances placées — donc après B5. Il est spécifié ici parce que la date de publication appartient à l'édition.

**Independent Test**: sur une édition portant un conflit bloquant, demander le contrôle préalable, constater la liste, tenter de publier, constater le refus ; lever le conflit, publier, constater la date posée sur l'édition.

**Acceptance Scenarios**:

1. **Given** une édition portant au moins un point bloquant, **When** on demande le contrôle préalable, **Then** la liste arrive **avant** toute tentative de publication, chaque point portant sa gravité, son intitulé, sa séance et son instant.
2. **Given** la même édition, **When** on tente de publier, **Then** rien n'est publié, la réponse le dit et rend la liste de ce qui bloque.
3. **Given** une édition ne portant que des avertissements, **When** on publie, **Then** la publication aboutit et les avertissements accompagnent la réponse.
4. **Given** une publication qui aboutit, **Then** l'édition porte sa date de publication, la réponse annonce le nombre de séances rendues publiques, et un événement de domaine est écrit **une fois**.
5. **Given** une édition sans aucune séance, **When** on publie, **Then** la publication aboutit avec zéro séance : une édition peut annoncer un programme vide, ce n'est pas un conflit.
6. **Given** une personne détenant la gestion des événements mais pas le droit de planifier, **When** elle tente de publier, **Then** l'écriture est refusée.

---

### Edge Cases

- **Une édition tenant un pavillon dont le sigle est retiré à la modification.** La règle se vérifie sur l'état **résultant** de l'écriture, jamais sur l'état antérieur : retirer le sigle d'une édition avec pavillon est le même refus que ne pas en fournir à la création.
- **Deux éditions de la même série, même année, même libellé.** Refusées par l'unicité du modèle ; le message doit désigner la série et l'année, pas l'adresse d'URL.
- **Une édition sans série.** Elle est légitime — un rendez-vous ponctuel —, et ne doit disparaître ni de la liste du back-office, ni de l'historique public.
- **Une période d'édition d'un an**, comme le cycle de webinaires du jeu de données : la génération du calendrier proposerait plus de trois cents journées vides. Le plan doit **annoncer le nombre avant d'écrire**, et rien ne doit s'écrire sans geste explicite.
- **Une période resserrée sur une édition dont les journées portent des séances.** Rien ne se supprime sans demande explicite, et le nombre de séances détachées s'annonce **avant** l'écriture.
- **Un fuseau que la base de fuseaux de PostgreSQL refuse.** Le refus doit désigner le champ du fuseau, pas retomber en erreur interne.
- **Deux canaux par défaut posés simultanément.** L'index unique du modèle en refusera un ; le service ne doit pas laisser l'écran découvrir une erreur de base, mais retirer le drapeau du précédent dans la même transaction.
- **Un canal général de la plateforme (sans édition) et un canal d'édition portant le même code.** L'unicité du modèle traite les deux valeurs « aucune édition » comme identiques : deux canaux généraux ne peuvent pas partager un code, alors qu'un canal général et un canal d'édition le peuvent.
- **Un appel annulé puis recréé.** L'unicité écarte les annulés : la seconde création doit aboutir, et la première rester lisible.
- **Un critère supprimé de la grille alors que des notes le référencent.** Les notes référencent le critère : la suppression doit être refusée ou neutralisée, jamais silencieusement destructrice.
- **Une note maximale de grille à zéro.** La normalisation de l'affichage (« 17,5 / 20 ») divise par cette valeur : une grille dont le total pondéré vaut zéro est impossible par construction, mais la lecture doit le supporter sans erreur.
- **Le même membre ajouté deux fois dans une composition de comité.** La clé primaire l'interdit ; le service doit dédoublonner la charge utile plutôt que remonter une erreur de base.
- **Une édition supprimée.** Elle emporte en cascade ses journées, ses fils, ses lieux, ses salles, ses canaux et son appel. Aucun écran ne l'offre ; l'API ne doit pas l'offrir non plus.
- **Le contrôle avant publication sur une édition sans séance.** La fonction du modèle rend une liste vide : ce n'est pas une erreur, et la publication doit aboutir.
- **Une publication rejouée.** Publier une programmation déjà publiée ne doit pas produire un second événement de domaine ni écraser la date d'origine sans le dire.
- **Les images d'une édition envoyées par le formulaire.** Le rattachement d'un fichier appartient au module Média, qui n'existe pas encore : la lecture les résout, l'écriture ne peut pas encore les poser.

---

## Requirements *(mandatory)*

### Le module et ses frontières

- **FR-001**: Le module DOIT vivre dans un crate propre sous les modules métier, ne dépendre que du noyau et des contrats d'événements, et n'être atteint par aucun autre crate de module. Le graphe DOIT rester vérifiable mécaniquement.
- **FR-002**: Le module DOIT être déclaré dans le registre des modules de la base — il l'est déjà — et n'être monté au démarrage que s'il y est actif.
- **FR-003**: Les routes, les noms de champs et les formes de réponse DOIVENT être **exactement** ceux que le front consomme déjà, pour la gestion des événements, pour les lectures publiques et pour les deux appels de publication du planificateur. Ils ne se renégocient pas.
- **FR-004**: Aucun invariant déjà porté par la base NE DOIT être réimplémenté : le service **traduit** l'erreur de la base en erreur d'API à code stable et message français, à partir de son code et du nom de la contrainte.
- **FR-005**: Le module NE DOIT écrire que dans son propre périmètre de données. Il **lit** hors de celui-ci lorsque la question porte sur ses propres entités — nombre de séances d'une journée, contrôle avant publication d'une édition, images rattachées à une édition, thématiques d'un fil, nom d'un responsable. Il n'appelle jamais un autre module.
- **FR-006**: Aucun fichier du modèle de données NE DOIT être modifié par ce jalon.

### Le périmètre d'administration

- **FR-007**: Toute lecture et toute écriture du back-office DOIVENT être bornées par le périmètre d'administration de l'appelant, obtenu par la fonction du modèle, qui renvoie **toujours** une ligne et jamais de valeur absente.
- **FR-008**: Les **trois** cas DOIVENT rester distincts : administration globale → tout ; éditions listées → filtrage sur ces éditions ; **aucun droit → refus d'accès explicite**, jamais une liste vide.
- **FR-009**: Une route paramétrée par un identifiant d'édition — ou par un identifiant de journée, de fil, de lieu, de salle, de canal, d'appel ou de comité qui s'y rattache — DOIT vérifier le périmètre **avant** de lire, et refuser une édition hors périmètre **y compris quand l'identifiant est forgé**.
- **FR-010**: Le refus pour cause de périmètre NE DOIT PAS se distinguer, par la forme de la réponse, du refus pour identifiant inexistant.
- **FR-011**: La **création** d'une édition DOIT exiger la gestion des événements sur la portée **globale** : une édition qui n'existe pas encore n'offre aucune portée où vérifier un droit.
- **FR-012**: Les écritures portant sur une édition existante — l'édition, ses journées, ses fils, ses lieux, ses salles, ses canaux — DOIVENT exiger la gestion des événements sur la portée de **cette édition** ou sur la portée globale.
- **FR-013**: Les écritures portant sur l'appel, sa grille et son comité DOIVENT exiger la gestion des **appels**, sur la portée de l'édition ou la portée globale. Les deux permissions DOIVENT être testées séparément : détenir l'une n'accorde pas l'autre.
- **FR-014**: La réponse de la liste DOIT dire si l'appelant administre la plateforme entière, pour que l'écran distingue un filtrage d'une absence.

### Séries d'événements

- **FR-015**: L'API DOIT exposer la liste des séries avec leur genre, leur nom, leur état d'activité et **le nombre d'éditions déjà rattachées** — ce qui distingue une série vive d'une coquille.
- **FR-016**: La liste des séries DOIT être servie aussi bien à la page publique, sans session, qu'au formulaire du back-office : c'est le genre de la série, et non une liste d'adresses recopiée dans un composant, qui distingue une COP d'un cycle de webinaires.
- **FR-017**: Le genre d'une série NE DOIT PAS être recopié dans un vocabulaire applicatif : c'est une énumération du modèle, et elle fait foi.

### Éditions — lecture du back-office

- **FR-018**: La liste des éditions DOIT rendre, **en une réponse**, les lignes, les séries proposables au filtre et les millésimes présents dans les lignes — les facettes se comptent sur le même jeu de lignes que la liste.
- **FR-019**: Chaque ligne DOIT porter sa **série résolue** — nom et genre —, et non le seul identifiant.
- **FR-020**: Chaque ligne DOIT porter ses décomptes joints : dossiers **déposés** (brouillons exclus), séances de l'édition, séances effectivement placées en salle, journées du calendrier créées, statut de l'appel et échéance effective de l'appel.
- **FR-021**: La ligne NE DOIT PAS porter la description ni le message d'accueil : ce sont deux paragraphes par édition, et la liste s'en passe.
- **FR-022**: Le détail d'une édition DOIT rendre, **en une réponse**, l'édition, sa description, son message d'accueil, sa période en dates civiles dans son fuseau, ses **trois** déclinaisons d'image résolues, ses journées, ses fils, ses lieux et salles, ses canaux, son appel et sa grille, son comité, le personnel assignable comme responsable de fil, les candidats au comité et les thématiques disponibles.
- **FR-023**: Le détail NE DOIT PAS exiger d'appel supplémentaire pour ouvrir un onglet : les six onglets d'une même édition tiennent en une réponse, et l'équipe passe de l'un à l'autre sans arrêt.
- **FR-024**: Une écriture d'onglet DOIT rendre la composition **entière** recalculée, afin qu'aucun décompte des cinq autres onglets ne reste faux.
- **FR-025**: Les options du formulaire — séries, pays, fuseaux, statuts atteignables — DOIVENT être servies **à part** de la liste : elles ne changent pas d'une édition à l'autre.

### Éditions — écriture, sigle et calendrier

- **FR-026**: La création et la modification d'une édition DOIVENT accepter exactement les champs que le formulaire envoie, et rendre l'édition telle qu'elle est devenue.
- **FR-027**: **Écart n° 9.** Une édition dont le pavillon est tenu DOIT porter un sigle. La règle est vérifiée **par le service**, sur l'état résultant de l'écriture, à la création comme à la modification. Le modèle N'EST PAS modifié : le sigle reste facultatif en base, parce que les webinaires du cycle PACO n'en ont pas et que la reprise des données de la v1 en dépend.
- **FR-028**: Le sigle DOIT comporter de **2 à 12 caractères**, lettres, chiffres et tiret uniquement. Un sigle hors de ces bornes est refusé **sur son champ**.
- **FR-029**: Le refus pour sigle manquant DOIT porter une **valeur par défaut proposée**, dérivée du libellé de l'édition, que l'écran peut préremplir et que l'équipe peut modifier. Un refus qui ne propose rien fait chercher une convention que personne n'a écrite.
- **FR-030**: Une édition **sans** pavillon DOIT être acceptée sans sigle. Aucun sigle NE DOIT être inventé d'office pour elle : le numéro de dossier ne la concerne pas.
- **FR-031**: Chaque contrainte nommée du modèle DOIT être traduite en un code stable distinct et rattachée au **champ** fautif : période inversée, lieu physique exigé hors ligne, adresse d'URL déjà prise, série et millésime déjà pris, millésime hors bornes, coordonnées incomplètes, champ obligatoire manquant.
- **FR-032**: Les instants de début et de fin DOIVENT être reçus et conservés comme des instants complets. Le service NE DOIT PAS décider d'un fuseau à la place de l'appelant.
- **FR-033**: Un enregistrement d'édition DOIT créer les journées manquantes de la période et **n'en supprimer aucune**. La réponse annonce le nombre créé.
- **FR-034**: L'API DOIT exposer un **plan de génération du calendrier** en lecture seule : dates à créer, journées hors période avec le nombre de séances qu'elles portent, et nombre de journées inchangées. Rien ne s'écrit à cette lecture.
- **FR-035**: La génération du calendrier DOIT rester un **geste explicite**. Le retrait des journées hors période DOIT être demandé explicitement ; à défaut, aucune journée n'est retirée.
- **FR-036**: Le retrait d'une journée DOIT laisser survivre les séances qu'elle portait, qui perdent leur rattachement au jour. La réponse DOIT chiffrer ce détachement.
- **FR-037**: Une journée générée DOIT porter sa date, son rang et **rien d'autre** : aucun titre inventé, qui s'afficherait tel quel sur la page publique.
- **FR-038**: La régénération NE DOIT PAS écraser le contenu éditorial d'une journée déjà habillée — titre, adresse de page, description, mise en avant, couleur.
- **FR-039**: L'écriture éditoriale d'une journée DOIT être une route distincte de la génération, et refuser une adresse de page déjà prise **au sein de l'édition**.

### Fils de programmation — les journées spéciales

- **FR-040**: L'API DOIT permettre de créer, modifier et supprimer un fil de programmation, avec son genre, son code, son adresse, ses textes, sa période **indicative**, sa couleur, son responsable, son rang et l'ouverture de sa page publique.
- **FR-041**: Le code et l'adresse d'un fil DOIVENT être uniques **au sein de l'édition**, et chaque refus porter son propre code et son propre champ.
- **FR-042**: Les thématiques d'un fil DOIVENT être écrites dans le référentiel de rattachement prévu par le modèle, et rendues à l'affichage avec leur **libellé traduit et leur couleur**. Elles NE DOIVENT jamais être recopiées dans un fichier de traduction ni figées dans le code.
- **FR-043**: La période d'un fil est **indicative** et NE DOIT contraindre aucun rattachement de séance. Seule sa cohérence interne — fin après début — est vérifiée par la base.
- **FR-044**: L'API NE DOIT PAS composer un fil : le rattachement d'une séance à un fil est une décision éditoriale prise au planificateur et écrite dans le schéma de la programmation. Le nombre de séances rattachées est rendu **en lecture seule**.
- **FR-045**: La suppression d'un fil DOIT laisser survivre les séances rattachées et **chiffrer** le nombre de rattachements éditoriaux perdus.
- **FR-046**: L'ouverture et la fermeture de la page publique d'un fil DOIVENT être portées par le même enregistrement que le reste du fil : deux gestes distincts laisseraient exister un fil publié sans contenu.

### Lieux, salles et canaux de diffusion

- **FR-047**: L'API DOIT permettre de créer, modifier et supprimer un lieu et ses salles, avec leur genre, leur adresse, leur plan, leur code, leur capacité, leurs équipements et leur rang.
- **FR-048**: Le caractère **virtuel** d'une salle DOIT être écrit tel quel, sans interprétation : une salle virtuelle accepte des séances simultanées et la détection des conflits ne signale aucune double réservation dessus. Le service NE DOIT PAS le déduire du mode de participation.
- **FR-049**: Le code d'une salle DOIT être unique au sein de son lieu, et le refus porter son champ.
- **FR-050**: La suppression d'un lieu ou d'une salle DOIT laisser survivre les séances qui s'y tenaient et **chiffrer** leur détachement.
- **FR-051**: L'API DOIT permettre de créer, modifier, désactiver et supprimer un canal de diffusion d'édition, avec son code, son nom, son fournisseur, son compte diffuseur, sa langue et ses deux drapeaux.
- **FR-052**: Désigner un canal comme **canal par défaut** DOIT retirer ce drapeau au canal qui le portait, **dans la même transaction** : l'index unique du modèle refuserait deux canaux par défaut, et l'écran n'a pas à décocher le précédent.
- **FR-053**: Le retrait d'un canal ayant servi à une diffusion DOIT le **désactiver** et non le supprimer. La réponse DOIT dire que la ligne est restée, inactive — c'est un succès, pas un refus.
- **FR-054**: Un canal **général de la plateforme**, non rattaché à une édition, NE DOIT PAS être modifiable depuis une édition, et le refus DOIT le dire.
- **FR-055**: Le service NE DOIT PAS affecter de canal à une séance : le modèle le fait par déclenchement dès qu'une séance devient diffusée.

### Appel à propositions et grille de critères

- **FR-056**: L'API DOIT permettre de créer et de modifier l'appel d'une édition **avec sa grille de critères, en un seul geste et une seule transaction**. Un échec sur la grille NE DOIT laisser aucun appel.
- **FR-057**: L'unicité 0..1 par édition DOIT être traduite en un refus **nommé** : cette édition porte déjà un appel non annulé. Un appel **annulé** n'empêche pas d'en créer un nouveau.
- **FR-058**: Une grille **vide** DOIT être refusée : aucun dossier ne pourrait être évalué.
- **FR-059**: Deux critères portant le même code DOIVENT être refusés, et le refus DOIT désigner **le rang de la ligne fautive** dans la grille.
- **FR-060**: Chaque contrainte nommée de l'appel DOIT être traduite en un code stable distinct rattaché à son champ : fenêtre inversée, prolongation antérieure à la clôture, bornes d'intervenants incohérentes, bornes de durée incohérentes ou durée par défaut hors bornes, plage d'accueil du pavillon inversée, code d'appel déjà pris sur l'édition.
- **FR-061**: L'API DOIT exposer la **grille par défaut** du modèle, lue et jamais recopiée. Les six critères, leurs libellés bilingues, leurs poids et le critère éliminatoire viennent de la base.
- **FR-062**: L'enregistrement DOIT **prévenir** lorsqu'un barème modifié affecte des notes déjà posées : les notes ne sont pas perdues, mais les moyennes se recalculent, et un classement qui bouge sans explication est une conversation difficile avec le comité.
- **FR-063**: Un critère référencé par des notes NE DOIT PAS être détruit silencieusement. Le service DOIT refuser sa suppression en nommant le critère et le nombre de notes qu'il porte.
- **FR-064**: L'appel rendu DOIT porter, outre ses colonnes, son **échéance effective**, son état **ouvert** — statut **et** fenêtre —, sa **note maximale atteignable** et le nombre de dossiers déposés. Les trois premiers viennent des fonctions du modèle et ne sont pas recalculés.
- **FR-065**: La plage d'accueil du pavillon est exprimée en heure **locale de l'édition**. Le service NE DOIT PAS la convertir : c'est une règle de la campagne, appliquée par B4 au moment de la soumission.
- **FR-066**: Une prolongation DOIT être conservée **à part** de l'échéance initiale : la trace de ce qui a été annoncé aux organisations ne se perd pas.
- **FR-067**: Le mode d'évaluation en aveugle DOIT être écrit tel que l'appel le porte, sans valeur par défaut imposée par le service : un seul mot dans cette colonne tranche l'arbitrage encore ouvert, sans toucher au code.

### Comité de sélection

- **FR-068**: La composition du comité DOIT s'enregistrer **d'un seul geste** — ajouts, retraits et plafonds de charge ensemble, dans une seule transaction.
- **FR-069**: La réponse DOIT **nommer** les membres retirés qui portaient encore des dossiers, avec le nombre de dossiers concernés. Un retrait silencieux laisse des dossiers sans lecteur à trois jours de la décision.
- **FR-070**: Le retrait d'un membre NE DOIT annuler aucune évaluation déjà rendue.
- **FR-071**: Chaque membre rendu DOIT porter son nom, son adresse, son organisation, son rôle de pilote, son plafond **indicatif**, ses dossiers confiés, ses évaluations rendues et **le fait qu'il détienne ou non la permission d'évaluer sur cette édition**.
- **FR-072**: Siéger au comité N'ACCORDE aucun droit. Le service NE DOIT PAS attribuer de rôle en ajoutant un membre : l'autorisation reste portée par les attributions de rôle, sur la portée de l'édition.
- **FR-073**: Une charge utile contenant deux fois la même personne DOIT être dédoublonnée par le service, jamais remontée comme une erreur de base.

### Publication de la programmation

- **FR-074**: L'API DOIT exposer le **contrôle avant publication** d'une édition en lecture seule, en appelant la fonction du modèle et sans en réécrire une ligne. Chaque point rendu porte sa gravité, son intitulé, son détail, sa séance et son **instant** — jamais un intervalle déjà mis en forme.
- **FR-075**: Le contrôle DOIT être consultable **avant** toute tentative de publication : on montre ce qui bloque, on ne le découvre pas en essayant.
- **FR-076**: La publication DOIT être **refusée** dès qu'un point de gravité bloquante subsiste. Rien n'est publié, et la réponse rend la liste de ce qui reste à régler.
- **FR-077**: Les **avertissements** NE DOIVENT PAS retenir la publication : un intervenant attendu à deux endroits est un problème que l'équipe juge, pas une impossibilité matérielle.
- **FR-078**: Une publication qui aboutit DOIT poser la **date de publication de la programmation sur l'édition**, rendre les séances planifiées visibles au public, et annoncer le nombre de séances rendues publiques.
- **FR-079**: La visibilité des séances relève du schéma de la programmation. Ce module NE DOIT PAS y écrire : l'effet est annoncé par un **événement de domaine** émis dans la même transaction que la date posée sur l'édition, et consommé par le module Programmation, avec garde contre le rejeu.
- **FR-080**: Publier une programmation **déjà publiée** NE DOIT PAS écraser silencieusement la date d'origine ni produire un second événement de domaine.
- **FR-081**: Une édition **sans aucune séance** DOIT pouvoir être publiée : la liste de contrôle est vide, et zéro séance publiée n'est pas une erreur.
- **FR-082**: La publication DOIT exiger la permission de **planifier la programmation** sur la portée de l'édition ou la portée globale — c'est elle que le modèle attribue au rôle chargé de « publier la programmation », et non la gestion des événements.
- **FR-083**: Les chevauchements de créneaux NE DOIVENT jamais être bloqués à l'écriture. Le seul contrôle bloquant de tout le module est celui de la publication.

### Lectures publiques

- **FR-084**: L'API DOIT exposer les **éditions publiques** — ni brouillon, ni annulée — sans exiger de session, en s'appuyant sur la vue prête à l'emploi du modèle. **Écart n° 26** : la règle vit ici, et non recopiée dans chaque écran.
- **FR-085**: Une édition **annoncée** dont le programme n'est pas publié DOIT figurer dans cette liste : sa page existe, elle annonce ses échéances, et c'est là qu'on dépose un dossier.
- **FR-086**: Une édition **sans série** DOIT figurer dans cette liste : une jointure stricte ferait disparaître les rendez-vous ponctuels de tout historique.
- **FR-087**: La page publique d'une édition DOIT être servie par son adresse d'URL et porter, **en une réponse**, sa série, son pays, ses trois déclinaisons d'image résolues, son état temporel, son appel avec son état et son échéance effective, ses thématiques avec leur libellé et leur couleur. **Écart n° 25** : la lecture d'image séparée disparaît.
- **FR-088**: Une édition en brouillon ou annulée NE DOIT PAS être servie publiquement, et son absence NE DOIT PAS se distinguer d'une édition inexistante.
- **FR-089**: Les trois déclinaisons d'image DOIVENT être rendues **toutes les trois**, sans repli décidé par l'API : l'écran seul sait de quelle largeur il dispose.
- **FR-090**: L'API DOIT servir les journées, les fils, les lieux, les salles, les canaux et l'appel d'une édition en lecture publique, aux chemins que le front consomme déjà.
- **FR-091**: Le volume du programme d'une édition — nombre de séances publiées, bornes réelles du programme — DOIT être lu dans la vue du schéma de la programmation prévue pour cela, et joint à la lecture publique. La vue des éditions ne peut pas le porter, l'ordre de chargement du modèle allant dans l'autre sens.

### Événements de domaine, travaux différés, erreurs et documentation

- **FR-092**: Tout changement d'état notable DOIT être annoncé par un événement de domaine écrit dans la **même transaction** que le changement, par la fonction du modèle prévue pour cela, avec un type respectant la forme imposée. Le service NE DOIT jamais insérer à la main dans la file de sortie.
- **FR-093**: Aucun déclencheur du modèle n'émet d'événement de domaine pour ce module — contrairement à ceux des propositions et des séances. Le service DOIT donc les émettre lui-même, et le piège inverse — la double émission — n'existe pas ici : c'est vérifié, pas supposé.
- **FR-094**: Toute écriture DOIT passer par l'unique porte d'écriture du noyau, qui positionne l'acteur et l'identifiant de requête en début de transaction. Sans elle, l'historique d'une édition et d'un appel — les deux seules tables du module qui soient auditées — devient anonyme.
- **FR-095**: Toute erreur DOIT porter un **code stable** et un **message français**. Une erreur de validation DOIT désigner le **champ** fautif ; une erreur portant sur une ligne de grille DOIT désigner son **rang**.
- **FR-096**: Aucune erreur NE DOIT divulguer l'existence d'une édition hors du périmètre de l'appelant.
- **FR-097**: La documentation OpenAPI DOIT être **engendrée** auprès des gestionnaires de routes et depuis le catalogue de codes du noyau, jamais écrite à la main : un code ajouté apparaît au prochain démarrage, un code oublié n'existe pas.
- **FR-098**: Les travaux de fond du module — clôture d'un appel dont l'échéance effective est passée, rappel d'échéance — DOIVENT passer par la file de travaux du modèle, avec une clé d'unicité dès qu'un doublon serait visible par une organisation.
- **FR-099**: Aucun fichier livré NE DOIT dépasser 1000 lignes.

### Key Entities *(include if feature involves data)*

Toutes existent déjà dans `docs/database/`. Aucune n'est créée, aucune n'est modifiée.

- **Série** (`event.event_series`) — le rendez-vous récurrent, son genre, son identité durable et son organisateur. C'est elle qui rend les comparaisons pluriannuelles possibles.
- **Édition** (`event.events`) — l'occurrence : millésime, libellé dans la série, statut, mode de participation, fuseau de référence, période, lieu et point relevé, **pavillon tenu**, **sigle facultatif**, message d'accueil, et **date de publication de la programmation**. Deux unicités : l'adresse d'URL sur toute la plateforme, et le triplet série–millésime–libellé.
- **Journée du calendrier** (`event.event_days`) — une ligne par jour, avec son habillage éditorial facultatif et l'adresse de sa page dédiée. **Aucun déclencheur ne la dérive des dates de l'édition** : la génération est un comportement d'application.
- **Fil de programmation** (`event.programme_tracks`) — la journée spéciale ou le fil transversal, composé à la main par l'IFDD. Sa période est **indicative**. Ses thématiques passent par le référentiel de rattachement d'entités.
- **Lieu et salle** (`event.venues`, `event.rooms`) — ce qui donne un **sujet nommable** à un conflit de créneau. Le caractère virtuel d'une salle décide si la détection y signale une double réservation.
- **Canal de diffusion** (`event.broadcast_channels`) — ressource **réservable** au même titre qu'une salle, sur laquelle tient la règle « un seul direct à la fois ». Un seul canal par défaut par édition, et les canaux généraux de la plateforme partagent la même unicité de code.
- **Appel à propositions** (`event.calls_for_proposals`) — la campagne : fenêtre, prolongation conservée à part, règles de recevabilité, bornes de durée, plage d'accueil du pavillon, formats admis, nombre d'évaluations exigé, évaluation en aveugle. **Un seul par édition**, les annulés exclus.
- **Critère d'évaluation** (`event.review_criteria`) — la note pondérée, avec son barème, son poids et son caractère **éliminatoire**. La grille appartient à l'appel, ce qui permet de la faire évoluer d'une année sur l'autre sans réécrire l'historique.
- **Membre du comité** (`event.call_reviewers`) — qui siège, qui pilote, quel plafond. **Dit la composition, jamais le droit d'accès.**
- **Périmètre d'administration** (`identity.administered_events`) — une ligne, jamais de valeur absente, trois cas distincts.
- **Permissions** (`identity.permissions`) — gestion des événements et gestion des appels, toutes deux du module `event` ; planification de la programmation, du module `programme`, qui garde la publication.
- **Contrôle avant publication** (`programme.publication_readiness`) et **détection des conflits** (`programme.detect_conflicts`) — lus, jamais réécrits.
- **Vue des éditions publiques** (`event.v_public_editions`) et **statistiques de programmation** (`programme.v_edition_stats`) — les deux moitiés de la lecture publique, jointes sur l'identifiant d'édition.
- **Événement de domaine** (`platform.outbox_events`) et **travail différé** (`platform.jobs`) — annonces et travaux de fond du module.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Une édition tenant un pavillon ne peut **jamais** être enregistrée sans sigle, ni à la création ni à la modification, et le refus propose une valeur préremplissable — vérifié sur les quatre chemins d'écriture.
- **SC-002**: Une édition **sans** pavillon s'enregistre sans sigle, et le cycle de webinaires du jeu de données reste créable de bout en bout : **aucun cas d'usage existant n'est cassé** par l'écart n° 9.
- **SC-003**: Un dossier déposé sur une édition à sigle porte un numéro **lisible au téléphone** ; le cas « COP31-TE-00001 » reproduit en base le 16/08 ne se reproduit plus sur une édition à pavillon.
- **SC-004**: Les **six** contraintes nommées d'une édition et les **six** d'un appel produisent chacune un code stable distinct rattaché au bon champ — douze refus, douze champs, aucun message technique.
- **SC-005**: Un administrateur détaché sur une édition n'atteint **aucune** donnée d'une autre édition sur **aucune** des routes livrées, y compris en forgeant l'identifiant ; une personne sans droit reçoit **un refus**, jamais une liste vide.
- **SC-006**: La création d'une édition est refusée à un administrateur détaché sur une édition et acceptée à un administrateur global — les deux cas vérifiés.
- **SC-007**: Détenir la gestion des événements sans celle des appels permet d'écrire journées, fils, lieux, salles et canaux, et **refuse** l'appel, la grille et le comité — les deux moitiés vérifiées.
- **SC-008**: Une édition portant déjà un appel non annulé refuse le second ; la même édition, appel annulé, accepte le suivant.
- **SC-009**: L'appel et sa grille s'enregistrent **ensemble** : un échec sur la grille ne laisse **aucun** appel en base, vérifié par relecture après échec.
- **SC-010**: La grille par défaut servie correspond **exactement** — codes, libellés, barèmes, poids, critère éliminatoire — à celle que la base sème, sans une valeur recopiée dans le code.
- **SC-011**: Le détail d'une édition répond **en une seule requête cliente** pour ses six onglets, et une écriture d'onglet rend des décomptes justes sur les cinq autres.
- **SC-012**: Le plan de génération du calendrier n'écrit **rien** : la base est identique avant et après, journée par journée.
- **SC-013**: Une génération sans demande de retrait ne supprime **aucune** journée ; avec retrait, le nombre de séances détachées annoncé correspond **exactement** au nombre de séances effectivement détachées.
- **SC-014**: Une régénération sur une édition dont les journées sont habillées laisse **intacts** titres, adresses, couleurs et mises en avant — comparés champ à champ.
- **SC-015**: Désigner un second canal par défaut laisse **exactement un** canal par défaut sur l'édition, et l'index du modèle n'est jamais violé, y compris sur deux désignations concurrentes.
- **SC-016**: Le retrait d'un canal ayant servi le laisse **présent et inactif**, et la réponse le dit comme un succès.
- **SC-017**: La suppression d'un lieu, d'une salle, d'une journée ou d'un fil ne supprime **aucune** séance, et chaque réponse chiffre le détachement au nombre exact.
- **SC-018**: Une publication est refusée tant qu'un point bloquant subsiste, et **aucune** séance n'est rendue publique dans ce cas ; le même contrôle, consulté avant, rend la même liste.
- **SC-019**: Une publication qui aboutit produit **exactement un** événement de domaine, et une seconde publication n'en produit pas un deuxième ni n'écrase la date d'origine en silence.
- **SC-020**: Les chemins et les noms de champs servis correspondent **exactement** à ce que le front consomme : le basculement des données simulées vers l'API ne demande aucune modification d'écran.
- **SC-021**: La page publique d'une édition et la liste des éditions publiques répondent **sans session**, et une édition en brouillon ou annulée y est introuvable.
- **SC-022**: La page publique d'une édition porte ses trois déclinaisons d'image **sans appel supplémentaire** : l'écart n° 25 est refermé, et la lecture d'image séparée disparaît du contrat.
- **SC-023**: La documentation OpenAPI engendrée couvre **100 %** des routes livrées et **chaque** code d'erreur stable ajouté par ce module.
- **SC-024**: La porte de qualité passe : schéma rechargé de zéro, rapport de frontières de modules vide, **aucune arête** entre deux crates de module, mise en forme et analyse statique sans avertissement, tests d'intégration au vert sur base réelle.
- **SC-025**: Aucun fichier de `backend/` ne dépasse **1000 lignes**.

---

## Assumptions

Décisions prises faute de précision dans la demande, et à confirmer au besoin.

- **Le sigle est traité selon l'option A des points bloqués** (question n° 9, en attente depuis le 16/08) : obligatoire **dans le service** pour une édition tenant un pavillon, facultatif en base. C'est ce que le prompt demande, et c'est la seule option qui n'interdise pas un cas d'usage existant. Si l'arbitrage tranchait autrement, seul le service changerait.
- **Le format du sigle — 2 à 12 caractères, lettres, chiffres, tiret — est une règle de saisie du service**, non une contrainte du modèle. La valeur par défaut proposée se dérive du libellé français de l'édition, en majuscules, sans accent ni espace, tronquée à douze caractères.
- **La génération du calendrier reste applicative** (écart n° 1 d'A10). Aucune fonction n'est ajoutée au modèle : le prompt interdit d'y toucher sans justification, et la décision de retirer une journée — qui détache des séances — appartient à l'équipe, pas à un déclencheur. Le geste explicite, avec son plan annoncé, est ce que l'écran a déjà tranché.
- **Aucune borne dure n'est posée sur le nombre de journées générées** (écart n° 2 d'A10). Le plan annonce le nombre avant d'écrire, et l'écran avertit au-delà de quarante jours. Le choix entre « ne générer que pour les séries de genre COP » et « une édition de cycle de webinaires s'en passe » **reste à arbitrer avec le commanditaire** ; d'ici là, aucune règle implicite n'est codée.
- **Les images d'une édition sont lues, jamais écrites** (écart n° 4 d'A10). Le rattachement d'un fichier appartient au module Média (B6) : la charge utile du formulaire porte trois identifiants d'objet, que ce jalon **accepte sans les poser**, et la réponse résout les images déjà rattachées. À compléter en B6.
- **La publication de la programmation est spécifiée ici parce que la date appartient à l'édition, mais elle est livrée en dernier** (US8, P3) : elle ne sert qu'une fois les séances placées, donc après B5. Le module pose la date et annonce ; la visibilité des séances est l'effet d'un événement de domaine consommé par le module Programmation. La route que le front appelle aujourd'hui vit sous le planificateur et sera complétée par B5.
- **La publication est gardée par la permission de planifier la programmation**, du module `programme`, et non par la gestion des événements : c'est ce que le modèle attribue au rôle chargé de publier le programme. Le garde d'autorisation vivant dans le noyau depuis B1, tester cette permission ne crée **aucune** dépendance entre deux crates de module.
- **Le rôle de programmateur ne détient aujourd'hui aucune permission du module `event`** : un chargé de programmation ne peut donc ni composer les journées spéciales ni toucher aux lieux, contrairement à ce qu'annonçait l'écart n° 5 d'A10. Rien n'est corrigé dans le semis par ce jalon — ce serait modifier le modèle ; l'écart est consigné, à trancher.
- **Aucune route de suppression d'édition n'est livrée.** Aucun écran ne l'offre, et la cascade emporterait journées, fils, lieux, salles, canaux et appel. Le retrait d'une édition passe par son statut.
- **Aucune route de dé-publication de la programmation n'est livrée** : le contrat du front n'en porte aucune.
- **Les séries sont lues et non écrites par ce jalon.** Aucun écran n'offre de créer une série ; la liste est servie au filtre, au formulaire et à la page publique. La création d'une série viendra avec l'écran qui la demandera.
- **Seules l'édition et l'appel sont audités** par le modèle : l'historique champ par champ n'existe donc pas pour les journées, les fils, les lieux, les salles et les canaux. Aucun écran ne le demande ; c'est consigné pour que personne ne le suppose.
- **La clôture automatique d'un appel dont l'échéance effective est passée** est livrée comme travail différé récurrent, sur le modèle de la purge des jetons de B1. Sans elle, un appel resterait « ouvert » après son échéance et n'attendrait qu'un geste manuel.
- **Dépendance d'environnement** : les requêtes étant vérifiées à la compilation, la base doit être démarrée et le schéma chargé pour construire.

---

## Vérifications faites en écrivant cette spécification

- **`060_events.sql` n'a besoin d'aucune modification.** Relecture intégrale : les huit sections portent les dix-huit champs d'une édition, la cardinalité 0..1 de l'appel, la grille pondérée, le comité, et la vue des éditions publiques. Les deux compléments du 18/08 (coordonnées du lieu) et du 19/08 (trois déclinaisons d'image) y sont déjà.
- **Aucun déclencheur de `060_events.sql` n'émet d'événement de domaine.** Vérifié : le fichier ne contient que deux déclencheurs d'audit, sur les éditions et sur les appels. Le piège n° 1 rencontré en B1 et en B2 — une fonction de base qui émet déjà l'événement que le service s'apprête à émettre — **n'existe pas dans ce module**.
- **Aucune fonction de `060_events.sql` n'écrit.** Les quatre fonctions du fichier sont en lecture (`effective_deadline`, `is_call_open`, `max_weighted_score`) ou en semis (`seed_default_criteria`), et aucune n'est en `SECURITY DEFINER`. Rien n'a donc à être appelé depuis la porte d'écriture pour des raisons de contexte, contrairement à la fusion d'organisations.
- **`event.event_days` n'a aucun déclencheur de dérivation.** Rien en base ne crée une journée quand une édition change de période, et rien n'en supprime quand elle se resserre. La génération est bien un comportement d'application — l'écart n° 1 d'A10 est confirmé, pas contourné.
- **Le préfixe du numéro de dossier se lit dans le déclencheur d'affectation du schéma des propositions** : il prend le sigle en majuscules, et à défaut les **huit premiers caractères** de l'adresse d'URL, et à défaut une valeur de repli. Le cas « slug `cop31-test` → dossier COP31-TE-00001 » est exact et vient de là.
- **`ux_calls_one_per_event` exclut les appels annulés**, ce qui permet de repartir après une annulation sans supprimer l'historique. Le refus « appel déjà existant » ne doit donc pas se déclencher sur un appel annulé.
- **L'unicité des canaux traite les deux valeurs « aucune édition » comme identiques** (`NULLS NOT DISTINCT`) : deux canaux généraux de la plateforme ne peuvent pas partager un code, alors qu'un canal général et un canal d'édition le peuvent. L'index du canal par défaut, lui, regroupe les canaux généraux sous un identifiant de substitution et ne porte que sur les canaux **actifs**.
- **La vue des éditions publiques exclut brouillons et annulées, et rien d'autre.** Une édition **annoncée** dont le programme n'est pas publié y figure — c'est écrit en toutes lettres dans le fichier, et c'est exactement ce que l'écart n° 26 demandait de porter côté API.
- **La vue des éditions publiques ne porte pas le nombre d'activités**, et ne le peut pas : le schéma des événements se charge **avant** celui de la programmation. Le décompte vit dans la vue de statistiques de la programmation, jointe sur l'identifiant d'édition.
- **La vue de la programmation publique filtre sur la date de publication de chaque séance**, et non sur celle de l'édition. Publier une programmation exige donc **deux** écritures dans deux schémas : d'où la frontière posée par FR-079.
- **Le rôle de programmateur ne détient aucune permission `event.*`** : vérifié dans le semis des permissions de rôle. Seuls l'administrateur et le super-administrateur détiennent la gestion des événements et celle des appels. L'écart n° 5 d'A10 supposait le contraire.
- **La description du rôle de programmateur dit « planifie les créneaux et publie la programmation »**, et ce rôle détient la permission de planification. C'est elle, et non la gestion des événements, qui garde la publication.
- **Les deux permissions du module appartiennent bien au module `event`** dans le catalogue des permissions, et l'administrateur est attribuable **globalement ou sur un seul événement** — c'est la réponse du modèle au cas du webinaire confié à son responsable.
- **La fonction de périmètre filtre sur la permission de lecture de toutes les propositions**, et non sur une permission du module des événements. Le périmètre d'un administrateur d'édition est donc le même dans les six modules ; ce jalon n'a rien à y ajouter.
- **Les thématiques d'un fil passent par le référentiel de rattachement d'entités**, annoncé en toutes lettres par le commentaire du modèle. Aucune table de liaison à maintenir, et aucun libellé à recopier dans un fichier de traduction.
- **Le contrôle avant publication rend un instant et non un intervalle mis en forme** depuis la correction du 18/08 : l'interface situe l'instant dans le fuseau de l'édition et la langue du lecteur. L'API doit le laisser passer tel quel.
- **La détection des conflits ne vise que les séances installées en salle physique** depuis la correction du 18/08 : une séance sans salle n'occupe rien, et c'est l'état normal d'une activité retenue mais pas encore placée.
- **Aucun code Rust de ce module n'existe** : `backend/crates/modules/` contient `identity` et `org`. Cette spécification ajoute un crate, elle n'en complète aucun.
- **Le contrat du front est complet et cohérent** : quinze écritures et quatre lectures dans la fabrique de gestion des événements, treize lectures publiques dans la couche d'accès, deux appels de publication dans la fabrique du planificateur. Aucun nom de champ n'a été renégocié en écrivant cette spécification.
