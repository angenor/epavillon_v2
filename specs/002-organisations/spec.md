# Feature Specification: Organisations (B2)

**Feature Branch**: `002-organisations`

**Created**: 2026-08-20

**Status**: Draft

**Input**: User description: "Module ORGANISATIONS de l'API ePavillon v2 (Rust + Actix Web + SQLx). Recherche multi-signaux, création avec détection de doublon, rattachement par domaine vérifié, adhésions, détection continue, fusion. Point délicat : la recherche doit répondre en moins de 150 ms sur 5 000 organisations, avec anti-rebond côté client. Traiter les écarts n° 23, 24 et 33 relevés en écrivant les écrans A2 et A5, et les quatre obligations relevées en écrivant A11. Livrable : `backend/crates/modules/org`, monté par `api` et `worker`, qui existent depuis B1."

---

## Contexte

**Ce module est la réponse au défaut n° 1 de la version 1.** Citation du cadrage, reprise en tête de `docs/database/040_organizations.sql` : « 2 personnes ont créé deux fois la même organisation et on voit les 2 sur la plateforme sans possibilité de fusionner. Nous avons mis un mécanisme de recherche avant création, mais certains cherchaient par nom complet tandis que d'autres par sigle. » Tout ce qui suit se juge à cette phrase.

Le modèle est complet et fait autorité — `docs/database/040_organizations.sql` (référentiel, dénominations, domaines, adhésions, détection, fusion), `docs/database/130_analytics.sql` § 5 (`mv_organization_scorecard`), `docs/database/030_identity.sql` § 6 (les trois permissions `org.*`), et les **huit fichiers SQL qui alimentent le registre `org.organization_references`** (040 § 6, 050 § 8, 060 § 7, 070, 075, 080, 090, 125). Le SQL pose quatre verrous, du plus préventif au plus curatif : dénominations multiples, recherche multi-signaux, unicité structurelle, fusion réversible. **Aucune modification du SQL n'est proposée ici.**

Le front existe depuis le 17/08 et consomme des données simulées. Ses contrats — `frontend/app/types/org.ts`, `organization-join.ts`, `organization-workspace.ts`, `admin-organizations.ts` — et les chemins déclarés dans `frontend/app/composables/useApi.ts` et `composables/api/admin-organizations.ts` **sont le contrat de cette API**. Ils ne se renégocient pas. Quatre écrans en dépendent : le rattachement (A2), le choix des co-organisateurs du formulaire de dépôt (A4), l'espace organisation (A5), et les quatre pages du back-office (A11).

Le socle existe depuis B1 : `kernel` (contexte de requête, erreurs à code stable, unique porte d'écriture, garde d'autorisation, file de travaux, contrat d'envoi de courriel), `contracts`, `api`, `worker`. **Ce module ne les réinvente pas et ne dépend d'aucun autre crate de module.**

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Trouver son organisation quoi qu'on tape (Priority: P1)

Une personne cherche son organisation avant de s'y rattacher. Elle tape ce qui lui vient : le sigle, le nom complet, les huit premières lettres, une traduction, l'ancien nom. **Dans tous les cas, la même fiche remonte.**

Et elle ne remonte que ce qui répond à la question posée : chercher « Agence spatiale du Sahel » ne doit pas ramener l'organisation qui partage le domaine de son adresse et rien d'autre. Le back-office, lui, veut exactement l'inverse — « tout ce qui pourrait être la même entité ». **Deux besoins, deux lectures, une seule fonction de base.**

**Why this priority**: c'est le geste que la v1 a raté, et le seul endroit où le doublon se prévient. Tout le reste du module le répare après coup.

**Independent Test**: sur un jeu contenant une organisation à sigle, à traduction et à ancien nom, exercer les cinq façons de la désigner et vérifier qu'elles ramènent la même fiche ; puis vérifier qu'une recherche sans rapport avec le nom ne ramène pas la fiche du domaine de l'appelant.

**Acceptance Scenarios**:

1. **Given** une organisation portant un nom légal, un sigle et une traduction, **When** on cherche par le sigle, par le début du nom complet, par deux lettres, ou par la traduction, **Then** la même fiche remonte à chaque fois, **une seule fois**, avec la dénomination qui a déclenché la correspondance.
2. **Given** une personne dont l'adresse porte le domaine d'une organisation, **When** elle cherche le nom d'une organisation sans rapport, **Then** l'organisation du domaine **n'apparaît pas** dans les résultats de la recherche d'utilisateur.
3. **Given** la même personne, **When** elle cherche un nom qui ressemble à celui de l'organisation de son domaine, **Then** cette fiche remonte **et son score est rehaussé** par la correspondance de domaine, ce que la réponse énonce.
4. **Given** une fiche absorbée par une fusion, **When** on cherche son ancien nom, **Then** c'est la **fiche vivante** qui remonte, et jamais la fiche absorbée.
5. **Given** une recherche sans aucun résultat, **When** la réponse arrive, **Then** elle est vide — et non une liste de fiches vaguement proches.
6. **Given** 5 000 organisations en base, **When** une recherche est lancée, **Then** la réponse arrive en moins de 150 ms.

---

### User Story 2 — Être rattaché sans intervention, ou attendre un référent (Priority: P1)

Une personne dont l'adresse professionnelle porte un domaine vérifié rejoint son organisation **immédiatement**, sans que personne n'ait à l'approuver. Toute autre demande naît en attente et un référent tranche.

La différence ne tient pas à la volonté de la personne mais à ce que la base déclare : un domaine **vérifié** et marqué **rattachement automatique**. Une messagerie grand public ne prouve rien et ne déclenche rien.

**Why this priority**: c'est la moitié du parcours d'inscription, et la seule promesse de l'écran que l'API peut tenir seule.

**Independent Test**: se connecter sous une adresse portant un domaine vérifié en rattachement automatique, demander à rejoindre, constater l'adhésion active ; recommencer sous une adresse quelconque, constater l'attente.

**Acceptance Scenarios**:

1. **Given** une adresse portant un domaine vérifié et marqué rattachement automatique, **When** la personne demande à rejoindre cette organisation, **Then** l'adhésion est **active** immédiatement, et l'organisation devient son rattachement principal si elle n'en avait pas.
2. **Given** une adresse portant un domaine **non** vérifié, ou vérifié mais sans rattachement automatique, **When** la personne demande à rejoindre, **Then** l'adhésion est **en attente**, et la réponse le dit sans détour.
3. **Given** une adresse de messagerie grand public, **When** la personne ouvre l'écran de rattachement, **Then** aucune organisation ne lui est proposée d'office.
4. **Given** deux fiches déclarant le même domaine, dont une seule est vérifiée, **When** on interroge le domaine de l'adresse, **Then** c'est la **fiche vérifiée** qui est proposée.
5. **Given** une personne déjà membre ou déjà en attente, **When** elle redemande, **Then** la réponse dit « déjà membre » avec l'état de son adhésion, et **aucune seconde ligne n'est créée**.
6. **Given** une personne dont l'adhésion à cette organisation a été **révoquée**, **When** elle redemande à rejoindre, **Then** la demande est acceptée et **reprend la ligne existante** au lieu d'en créer une seconde.

---

### User Story 3 — Créer une fiche sans en fabriquer une deuxième (Priority: P1)

Une personne ne trouve pas son organisation et la crée. Pendant qu'elle saisit le nom, les fiches proches lui sont montrées. Si elle maintient sa création, la fiche est créée — **rien n'est bloqué** — mais l'API sait qu'on lui a montré, et la fiche part en revue.

Le seul refus possible est le doublon **exact**, celui que la base refuse elle-même : même nom normalisé, même pays. Et ce refus n'est pas un mur : il nomme la fiche existante, de quoi la rejoindre.

**Why this priority**: c'est le second moitié du geste de l'US1. Sans elle, une recherche infructueuse est une impasse.

**Independent Test**: créer une fiche en ayant vu des fiches proches, vérifier qu'elle naît en attente de rapprochement et que la personne en devient référente ; puis créer une fiche portant un nom déjà pris dans le même pays et vérifier le refus nommé.

**Acceptance Scenarios**:

1. **Given** un nom, un sigle, un type, un pays, **When** la fiche est créée, **Then** son statut est **`candidate`** et jamais `active`, son adresse d'URL est composée par l'API, et la personne en devient **référente** avec une adhésion active.
2. **Given** des fiches proches montrées avant la création, **When** la personne maintient sa création, **Then** la fiche est créée et **la liste des fiches montrées est conservée** pour la revue.
3. **Given** une organisation active portant déjà ce nom normalisé dans ce pays, **When** on tente la création, **Then** la réponse est « nom déjà pris » **et porte la fiche en cause**, avec de quoi la reconnaître et la rejoindre.
4. **Given** un sigle saisi, **When** la fiche est créée, **Then** le nom légal et le sigle deviennent tous deux des dénominations cherchables, sans écriture supplémentaire de l'appelant.
5. **Given** une personne qui n'a encore aucune organisation, **When** elle crée une fiche, **Then** cette organisation devient son rattachement principal.
6. **Given** un nom composé uniquement de signes que la normalisation efface, **When** on tente la création, **Then** l'adresse d'URL est tout de même composée et la création aboutit.

---

### User Story 4 — Deux files d'attente qui ne se confondent jamais (Priority: P2)

Un référent voit ce qu'il doit **trancher** : les demandes reçues. Une personne voit ce qu'elle doit **accepter** : les invitations qu'on lui a envoyées. Le même mot — « en attente » — recouvre deux attentes inverses, et les confondre fait entrer dans une organisation quelqu'un qui n'a jamais rien accepté.

Un refus ne supprime rien : l'adhésion est révoquée. La v1 effaçait la ligne, et plus personne ne pouvait distinguer une demande refusée d'une demande jamais faite.

**Why this priority**: l'espace organisation (A5) attend ces deux files, et c'est ce qui permet à une organisation de se constituer sans passer par l'IFDD.

**Independent Test**: inviter une adresse inconnue, constater la personne créée sans compte et l'invitation en vol ; tenter de l'« approuver » comme référent et constater le refus ; l'accepter par le lien reçu et constater l'adhésion active.

**Acceptance Scenarios**:

1. **Given** une adresse inconnue de la plateforme, **When** un référent l'invite, **Then** une personne est créée **sans compte et sans nom**, une adhésion en attente est ouverte **portant la direction de l'invitation**, et un lien à usage unique part par courriel.
2. **Given** une invitation en vol, **When** un référent tente de l'approuver, **Then** c'est **refusé** : une invitation attend la personne, pas l'organisation.
3. **Given** une demande spontanée en attente, **When** un référent l'approuve, **Then** l'adhésion devient active et porte l'auteur et la date de l'approbation.
4. **Given** la même demande, **When** un référent la refuse, **Then** l'adhésion est **révoquée** et sa ligne conservée avec sa date de révocation.
5. **Given** une personne déjà invitée dans cette organisation, **When** on l'invite une seconde fois, **Then** la réponse dit « déjà invitée, relançable » — jamais l'erreur de contrainte de la base.
6. **Given** une invitation acceptée par son lien, **When** l'adhésion devient active, **Then** le lien est consommé et ne vaut plus rien.
7. **Given** une personne qui n'est pas référente de l'organisation, **When** elle tente d'inviter ou de décider, **Then** l'accès est refusé.

---

### User Story 5 — Le back-office ne voit que ce qu'on lui a confié (Priority: P2)

Une coordonnatrice détachée sur une seule édition ouvre la liste des organisations. Elle y voit **les organisations qui ont déposé ou tenu une activité dans son édition**, et l'écran lui dit que sa liste a été restreinte. Une personne sans aucun droit d'administration reçoit un **refus**, jamais une liste vide.

Une organisation n'appartient à aucune édition : la règle métier n° 8 se prend donc par l'autre bout — la permission et sa portée d'un côté, l'activité déposée de l'autre.

**Why this priority**: quatre pages du back-office en dépendent, et c'est le premier module après l'identité à devoir tenir la règle sur une entité **transverse**.

**Independent Test**: appeler la liste sous trois comptes — global, détaché sur une édition, sans droit — et vérifier les trois réponses ; puis forger l'identifiant d'une organisation hors périmètre.

**Acceptance Scenarios**:

1. **Given** une personne au périmètre global, **When** elle demande la liste, **Then** elle reçoit toutes les fiches, et la liste n'est pas annoncée comme restreinte.
2. **Given** une personne détachée sur l'édition A, **When** elle demande la liste, **Then** elle ne reçoit que les organisations ayant déposé ou tenu une activité dans l'édition A, **et la réponse dit que la liste est restreinte**.
3. **Given** cette même personne, **When** elle demande la fiche d'une organisation hors de son périmètre, **Then** elle reçoit un **refus d'accès**, indiscernable de la réponse rendue pour une fiche inexistante.
4. **Given** une personne sans aucun droit d'administration, **When** elle demande la liste, **Then** elle reçoit un **refus d'accès**, et non une liste vide.
5. **Given** une liste restreinte, **When** les facettes sont rendues, **Then** elles sont comptées **sur le même jeu de lignes** que la liste.
6. **Given** une fiche absorbée par une fusion, **When** on ouvre sa page, **Then** elle s'ouvre normalement et porte le renvoi vers la fiche vivante — une fiche absorbée reste consultable.

---

### User Story 6 — Poser le sceau, vérifier un domaine, confirmer une dénomination (Priority: P2)

L'IFDD reconnaît une organisation : elle pose le sceau. Elle vérifie à la main qu'un domaine appartient bien à une fiche, et décide si les nouveaux inscrits de ce domaine y sont rattachés d'office. Elle confirme une dénomination saisie à l'import pour qu'elle cesse d'être seulement cherchable et devienne affichable.

**Le sceau n'est pas le statut.** Une fiche peut être active sans être vérifiée : elle dépose, son sceau ne s'affiche pas.

**Why this priority**: c'est ce qui fait passer une fiche de `candidate` à fiche de référence, et ce qui alimente le rattachement automatique de l'US2.

**Independent Test**: poser le sceau sur une fiche `candidate` et constater qu'elle est admise du même geste ; vérifier un domaine déjà vérifié ailleurs et constater le refus **nommant** la fiche qui le détient.

**Acceptance Scenarios**:

1. **Given** une fiche `candidate`, **When** on pose le sceau, **Then** la fiche est vérifiée **et passe `active`** — on ne certifie pas une organisation qu'on n'a pas admise.
2. **Given** une fiche vérifiée, **When** on retire le sceau, **Then** la vérification tombe **sans** que le statut change.
3. **Given** un domaine déjà vérifié pour une autre fiche, **When** on tente de le vérifier ici, **Then** le refus **nomme la fiche qui le détient** — sans ce nom, le refus est incompréhensible.
4. **Given** un domaine non vérifié, **When** on tente d'activer le rattachement automatique, **Then** c'est refusé : la base exige la vérification d'abord.
5. **Given** une écriture aboutie, **When** la réponse arrive, **Then** elle rend **la fiche entière recomposée** — vérifier un domaine change le score de confiance, qui change le rang dans la liste.
6. **Given** une dénomination posée par la base (nom légal, sigle), **When** on tente de la retirer, **Then** c'est refusé : elle suit la fiche.

---

### User Story 7 — La file des doublons se remplit toute seule (Priority: P2)

Personne ne va chercher les doublons à la main. Un balayage régulier compare les fiches vivantes et consigne les paires suspectes, la plus ressemblante d'abord, avec les motifs qui l'ont fait remonter — et ces motifs ne se valent pas : un domaine partagé est une preuve matérielle, une similarité de nom une présomption, un même pays presque rien.

Une paire écartée ne revient pas. Une paire reportée revient. Et le score de confiance d'une fiche suit ce qu'on vient de lui faire, sans attendre le lendemain.

**Why this priority**: sans ce balayage, la file du back-office reste vide et la fusion n'a rien à fusionner.

**Independent Test**: semer deux fiches proches, déclencher le balayage, constater la paire ; l'écarter, rejouer le balayage, constater qu'elle ne revient pas.

**Acceptance Scenarios**:

1. **Given** deux fiches vivantes suffisamment proches, **When** le balayage passe, **Then** une paire est consignée **une seule fois**, avec son score et ses motifs.
2. **Given** une paire déjà arbitrée « ce ne sont pas des doublons », **When** le balayage repasse, **Then** la paire **n'est pas ressuscitée**.
3. **Given** une paire non arbitrée dont l'une des fiches a changé, **When** le balayage repasse, **Then** son score et ses motifs sont **mis à jour**, sans effacer la paire.
4. **Given** une paire reportée, **When** on la remet en circulation, **Then** elle réapparaît dans la file active.
5. **Given** un domaine que l'on vient de vérifier, **When** la fiche est relue, **Then** son score de confiance **reflète déjà** la vérification.
6. **Given** cent adhésions approuvées coup sur coup sur la même fiche, **When** les recalculs de score se déclenchent, **Then** ils se **coalescent** et ne produisent pas cent recalculs.

---

### User Story 8 — Fusionner sans rien perdre, et sans rien casser (Priority: P2)

Deux fiches sont la même organisation. L'opérateur choisit laquelle absorbe l'autre, arbitre les champs qui divergent, lit **ce qui va se déplacer** table par table, saisit le nom de la fiche absorbée pour confirmer, et fusionne. La fiche absorbée survit : ses anciennes adresses continuent de mener quelque part.

C'est ce que la v1 ne savait pas faire, et c'est l'opération la plus dangereuse du module : rien ne l'annule d'un clic.

**Why this priority**: c'est le quatrième verrou du modèle, et la seule réponse aux doublons déjà créés — dont ceux que la reprise de la v1 amènera.

**Independent Test**: fusionner deux fiches et vérifier, chiffre par chiffre, que le décompte annoncé avant correspond à celui rendu après ; vérifier que la fiche absorbée reste consultable et pointe vers la vivante.

**Acceptance Scenarios**:

1. **Given** une paire et un sens de fusion, **When** on demande l'aperçu, **Then** il rend les champs comparés, **le décompte lu dans le registre des références** (transférées / supprimées car déjà présentes / supprimées), les dénominations et domaines apportés, et les avertissements.
2. **Given** le sens inversé, **When** on redemande l'aperçu, **Then** le décompte est **recalculé** — il n'est pas symétrique.
3. **Given** une fusion confirmée, **When** elle aboutit, **Then** les rattachements sont déplacés, la fiche absorbée passe `merged` avec son pointeur, une trace est écrite avec son motif et son décompte réel, et la paire de la file est marquée « fusionnée ».
4. **Given** des arbitrages de champs, **When** la fusion aboutit, **Then** la fiche survivante porte les valeurs retenues, **dans la même transaction** que la fusion : si l'une échoue, rien n'est écrit.
5. **Given** un nom de confirmation qui ne correspond pas à la fiche absorbée, **When** on tente la fusion, **Then** elle est refusée — et la comparaison ignore la casse et les accents.
6. **Given** une fiche cible elle-même déjà fusionnée, **When** on tente de la viser, **Then** le refus de la base ressort **mot pour mot** : « Cibler la fiche finale ».
7. **Given** une personne dont la permission de fusion n'est pas globale, **When** elle tente une fusion, **Then** l'accès est refusé : il n'existe pas de fusion limitée à une édition.
8. **Given** une fusion aboutie, **When** on relit l'outbox, **Then** **un seul** événement de fusion y figure.

---

### Edge Cases

- **Le domaine partagé par deux fiches, dont l'une seulement est vérifiée.** C'est le cas exact des deux fiches OSED des données simulées. La vérifiée l'emporte ; proposer la fiche en doublon reviendrait à l'alimenter.
- **Une recherche d'un seul caractère, ou vide.** Elle ne doit ni balayer la table entière ni renvoyer n'importe quoi : en deçà de deux caractères, aucune recherche n'est lancée.
- **Une limite de résultats démesurée** demandée par un client (`limit=100000`). Elle doit être bornée par le service.
- **Une correspondance de nom exactement au seuil.** La fonction du modèle n'inscrit le motif « ressemblance de nom » qu'**au-dessus** de 0,3, alors qu'une correspondance trigramme entre au seuil : une fiche entrée à 0,300 exactement remonterait sans porter le motif. La lecture filtrée l'écarte, comme le fait l'écran aujourd'hui — conséquence connue et acceptée, jamais une exception à coder.
- **Une adhésion révoquée puis redemandée.** La base n'admet **qu'une ligne par (organisation, personne)**, révocations comprises : une seconde demande doit **reprendre la ligne**, jamais en insérer une autre.
- **Le dernier référent d'une organisation qui s'en va.** Plus personne ne peut approuver les demandes reçues. Il faut décider si le retrait est refusé, ou si la fiche remonte au back-office.
- **Une personne membre des deux fiches d'une fusion, référente d'un côté et simple membre de l'autre.** Le dédoublonnage supprime la ligne côté source : le rôle de référent peut disparaître si c'est la cible qui porte le rôle le plus faible.
- **Le nom légal retenu de la fiche absorbée.** L'unicité (nom, pays) porte sur les fiches vivantes : appliquer ce nom à la survivante **avant** que la source ne passe `merged` échoue. L'ordre des deux écritures n'est pas indifférent.
- **L'adresse d'URL de la fiche absorbée retenue.** Elle est unique sans condition de statut : la survivante ne peut pas la reprendre tant que la fiche absorbée existe — et la fiche absorbée existe toujours, c'est la promesse.
- **Une fusion concurrente.** Deux opérateurs ouvrent le même aperçu ; le second doit recevoir « déjà fusionnée », pas une erreur de base.
- **Une fiche `merged` visée par un rattachement.** Rejoindre une fiche absorbée doit mener à la fiche vivante, jamais échouer.
- **Une organisation modifiée dont l'ancien nom reste cherchable.** Changer le nom légal ajoute une dénomination sans retirer l'ancienne : c'est voulu, et cela doit rester vrai après une modification comme après une fusion.
- **La projection analytique en retard.** La liste du back-office se lit dans une vue matérialisée : une fiche vérifiée il y a dix minutes montrerait son ancien score jusqu'au prochain rafraîchissement.
- **Une personne sans aucun rôle attribué.** Rien n'attribue aujourd'hui le rôle d'utilisateur ordinaire à l'inscription : une lecture gardée par la permission de consultation des organisations refuserait tout nouvel inscrit.

---

## Requirements *(mandatory)*

### Le module et ses frontières

- **FR-001**: Le module DOIT vivre dans un crate propre sous les modules métier, ne dépendre que du noyau et des contrats d'événements, et n'être atteint par aucun autre crate de module. Le graphe DOIT rester vérifiable mécaniquement.
- **FR-002**: Le module DOIT être déclaré dans le registre des modules de la base et n'être monté au démarrage que s'il y est actif.
- **FR-003**: Les routes, les noms de champs et les formes de réponse DOIVENT être **exactement** ceux que le front consomme déjà. Ils ne se renégocient pas.
- **FR-004**: Aucun invariant déjà porté par la base NE DOIT être réimplémenté : le service **traduit** l'erreur PostgreSQL en erreur d'API à code stable et message français, à partir du code SQL et du nom de la contrainte.

### La recherche — écart n° 23 : deux lectures, une seule fonction

- **FR-005**: L'API DOIT exposer **deux lectures distinctes** au-dessus de la même fonction de recherche du modèle, et **documenter leur différence à l'endroit où elles sont déclarées**. La fonction NE DOIT PAS être modifiée.
- **FR-006**: La lecture **destinée à une personne** NE DOIT rendre que les fiches portant une **ressemblance de dénomination** avec ce qui a été tapé. Une fiche qui ne remonte que parce qu'elle partage le domaine de l'appelant en est **écartée**.
- **FR-007**: Le domaine de l'appelant DOIT néanmoins continuer d'**alimenter le score** : une fiche qui correspond aussi par le nom se hisse en tête, et la réponse énonce les motifs qui ont joué.
- **FR-008**: La lecture **destinée à la revue des doublons** DOIT rendre tous les signaux sans filtre — c'est la question « qu'est-ce qui pourrait être la même entité ». Elle est bornée aux personnes qui administrent, et sert le balayage de l'US7.
- **FR-009**: La recherche DOIT interroger **toutes** les dénominations — nom légal, sigle, nom court, traduction, ancien nom, faute de frappe connue — en une seule passe.
- **FR-010**: Une organisation NE DOIT apparaître qu'**une fois** dans les résultats, avec sa meilleure dénomination et le rang qui départage deux dénominations d'égal score.
- **FR-011**: Seules les fiches **vivantes** (en attente de rapprochement ou de référence) DOIVENT être rendues. Une fiche absorbée n'apparaît jamais ; son ancien nom, lui, mène à la fiche vivante.
- **FR-012**: Chaque résultat DOIT porter de quoi **reconnaître** la fiche sans requête supplémentaire : type, ville, pays, sceau de vérification, nombre de membres **actifs**, dénomination ayant déclenché la correspondance, score et motifs.
- **FR-013**: La recherche NE DOIT PAS être lancée en deçà de **deux caractères**, et le nombre de résultats DOIT être borné par le service, quelle que soit la limite demandée par le client (défaut 10, maximum 50).
- **FR-014**: La recherche DOIT exiger une **session** et rien de plus. Elle NE DOIT PAS être gardée par la permission de consultation des organisations, qu'aucun nouvel inscrit ne détient aujourd'hui.
- **FR-015**: Le seuil de « correspondance forte » (85) reste **une décision d'interface**. L'API rend le score ; elle ne classe pas les résultats en forts et faibles.

### Ce que le domaine d'une adresse révèle

- **FR-016**: L'API DOIT exposer une lecture rendant l'organisation qui détient le domaine de l'adresse de **la personne connectée**, avec la ligne de domaine qui l'a produite, le nombre de membres actifs, et le fait que le rattachement puisse être **immédiat**.
- **FR-017**: Le domaine DOIT être dérivé de la **session**, jamais du paramètre transmis par le client. Un client qui déclare une adresse ne DOIT rien pouvoir apprendre d'un domaine qui n'est pas le sien.
- **FR-018**: Un domaine de **messagerie grand public** DOIT être neutralisé : la réponse est vide, et aucun rapprochement n'en découle. La liste est celle de la base, jamais une liste écrite dans le code.
- **FR-019**: Quand plusieurs fiches déclarent le domaine, la fiche **vérifiée** l'emporte, puis celle en rattachement automatique.
- **FR-020**: Le rattachement est **immédiat** si et seulement si la ligne de domaine est **vérifiée** ET marquée rattachement automatique. La condition est lue dans la base, jamais devinée.

### Rejoindre une organisation

- **FR-021**: Une demande de rattachement DOIT produire une adhésion **active** quand le domaine l'autorise (FR-020), et **en attente** dans tous les autres cas. L'issue rendue DOIT distinguer les deux.
- **FR-022**: Une personne déjà membre ou déjà en attente DOIT recevoir « déjà membre » avec l'état de son adhésion, sans qu'aucune ligne ne soit créée.
- **FR-023**: Une adhésion **révoquée** DOIT être **reprise** par une nouvelle demande, jamais doublée : la base n'admet qu'une ligne par organisation et par personne, révocations comprises.
- **FR-024**: Une demande visant une fiche **absorbée** DOIT être portée sur la **fiche vivante**, par la fonction de résolution du modèle.
- **FR-025**: L'API NE DOIT PAS calculer elle-même le rattachement principal : la base l'attribue à la première adhésion active et met à jour la personne. Le service ne repose pas cette règle.
- **FR-026**: Une demande spontanée DOIT naître **sans direction d'invitation** — c'est ce qui la range dans la file du référent et non dans celle de la personne.

### Créer une organisation

- **FR-027**: Une fiche créée depuis un formulaire DOIT naître **en attente de rapprochement**, jamais de référence. C'est ce qui alimente la file de dédoublonnage.
- **FR-028**: L'adresse d'URL DOIT être **composée par l'API** depuis le nom, et rendue unique en cas de collision. Un nom que la normalisation efface entièrement DOIT tout de même produire une adresse valide.
- **FR-029**: La création DOIT accepter la liste des **fiches proches affichées** avant qu'on ne maintienne la création, et la **conserver** pour la revue. Créer sans rien avoir vu et créer en connaissance de cause ne se traitent pas pareil.
- **FR-030**: Le seul refus de création DOIT être le **doublon exact** que la base refuse — même nom normalisé, même pays, fiche vivante. La réponse DOIT porter la fiche en cause sous la forme d'un résultat de recherche, de quoi la rejoindre.
- **FR-031**: Une simple ressemblance NE DOIT **jamais** bloquer une création. L'API rend l'information visible ; elle ne l'empêche pas.
- **FR-032**: Le créateur DOIT devenir **référent** de la fiche, avec une adhésion **active** — personne d'autre ne peut l'approuver, et quelqu'un doit pouvoir accepter les adhésions suivantes (écart n° 24).
- **FR-033**: Le nom légal et le sigle NE DOIVENT PAS être recopiés à la main dans les dénominations : la base s'en charge par trigger.

### Adhésions — écart n° 33 : deux files, deux autorisations

- **FR-034**: Une adhésion en attente DOIT porter sa **direction** : émise par l'organisation (invitation) ou demandée par la personne. Les deux colonnes qui la portent vont ensemble.
- **FR-035**: **Approuver ou refuser une demande** DOIT être réservé à un **référent** de l'organisation, et NE DOIT jamais pouvoir porter sur une **invitation**.
- **FR-036**: **Accepter une invitation** DOIT être le geste de la personne invitée, par un **lien à usage unique** de finalité invitation, consommé à l'acceptation.
- **FR-037**: Un refus DOIT **révoquer** l'adhésion avec sa date, jamais supprimer la ligne.
- **FR-038**: L'invitation par adresse DOIT **créer la personne** si l'adresse est inconnue, **sans compte**, et **sans déduire de nom depuis l'adresse**.
- **FR-039**: Une seconde invitation vers la même organisation DOIT recevoir une réponse explicite — « déjà invitée, relançable » — et non l'erreur de contrainte de la base.
- **FR-040**: L'API DOIT rendre les **adhésions vivantes** d'une personne — actives et en attente — pour la personne elle-même ; y accéder pour autrui exige la permission de consultation des utilisateurs.
- **FR-041**: Le retrait du **dernier référent actif** d'une organisation DOIT être refusé tant qu'aucun autre n'a été désigné. Un administrateur détenant la gestion des organisations peut passer outre, et la fiche est alors signalée **sans référent** au back-office, où ses demandes en attente restent approuvables.
- **FR-042**: Une invitation DOIT porter le **rôle** proposé ; une demande spontanée naît **membre**. Le rôle de référent ne s'obtient que par création, par invitation explicite, ou par promotion d'un référent en place.

### Back-office — lecture et périmètre

- **FR-043**: La liste des organisations DOIT exiger la permission de **consultation des organisations sur une portée quelconque** **et** un périmètre d'administration **non vide**. Les trois cas du périmètre restent distincts : global, éditions listées, aucun droit → **refus explicite**, jamais une liste vide.
- **FR-044**: La permission de consultation ne suffit pas à elle seule : elle est **détenue par le rôle d'utilisateur ordinaire**. C'est la conjonction avec le périmètre d'administration qui garde le back-office.
- **FR-045**: Une liste restreinte NE DOIT contenir que les organisations ayant **déposé ou tenu une activité** dans les éditions administrées — porteuses comme co-organisatrices —, et la réponse DOIT **dire qu'elle est restreinte**.
- **FR-046**: Les facettes DOIVENT être comptées **sur le même jeu de lignes** que la liste rendue.
- **FR-047**: La liste DOIT porter, par fiche, le nombre de **paires de doublons non arbitrées** où elle figure et le nombre de fiches qu'elle a **absorbées**.
- **FR-048**: La liste se lit dans la **projection analytique**, mais le **statut**, la **vérification**, le **score de confiance** et le **pointeur de fusion** DOIVENT être relus sur la table vivante : une fiche vérifiée à l'instant ne doit pas afficher son ancien score (obligation n° 3 relevée en A11).
- **FR-049**: La fiche d'une organisation DOIT être rendue **en une réponse** : identité, sceau, fiche de performance, dénominations, domaines, membres, activités, historique, fusions, et paires de doublons ouvertes.
- **FR-050**: Une fiche **absorbée** DOIT s'ouvrir normalement et porter le renvoi vers la fiche vivante ; une fiche **hors périmètre** DOIT être refusée d'une réponse indiscernable de celle d'une fiche inexistante, y compris quand l'identifiant est forgé.
- **FR-051**: L'historique DOIT être lu par la fonction d'historique de la plateforme, et porter le **libellé d'auteur** dénormalisé, qui reste lisible après un effacement RGPD.

### Écritures de la fiche

- **FR-052**: Poser ou retirer le **sceau** DOIT exiger la permission de gestion des organisations. Poser le sceau sur une fiche en attente de rapprochement l'**admet du même geste** ; le retirer ne change **pas** le statut.
- **FR-053**: Vérifier un **domaine** à la main DOIT enregistrer la méthode « manuelle » ; la vérification par enregistrement DNS ou par courriel n'entre pas dans ce jalon.
- **FR-054**: Un domaine déjà vérifié pour une autre fiche DOIT produire un refus **nommant** la fiche qui le détient.
- **FR-055**: Activer le **rattachement automatique** d'un domaine non vérifié DOIT être refusé, en traduisant la vérification de la base.
- **FR-056**: **Confirmer une dénomination** ne décide que de son **affichage** : confirmée ou non, elle sert la recherche. Les dénominations **posées par la base** — nom légal, sigle — ne se retirent pas à la main.
- **FR-057**: Toute écriture de la fiche DOIT rendre **la fiche entière recomposée** : vérifier un domaine change le score, qui change le rang dans la liste, et poser le sceau change ce que la file affiche.

### Détection continue et score de confiance

- **FR-058**: Un travail différé **récurrent** DOIT balayer les fiches vivantes et consigner les paires suspectes, avec leur score et leurs motifs, ordonnées comme la base l'exige et **sans doublon de paire**.
- **FR-059**: Une paire **déjà arbitrée** NE DOIT PAS être ressuscitée par un balayage ultérieur. Une paire **non arbitrée** voit son score et ses motifs **mis à jour**.
- **FR-060**: Le seuil d'entrée dans la file DOIT être un **réglage d'exploitation** déclaré dans la configuration du service, et non une valeur en base. Valeur par défaut : **60**.
- **FR-061**: La tâche récurrente DOIT **se replanifier elle-même**, et le démarrage du binaire des travaux différés ne fait que **réarmer** la chaîne — le motif éprouvé en B1.
- **FR-062**: Une décision portée sur une paire DOIT distinguer « ce ne sont pas des doublons » de « pas maintenant », et la remise en circulation d'une paire reportée DOIT être possible.
- **FR-063**: Le **score de confiance** DOIT être recalculé par un travail différé, déclenché par les changements qui l'affectent — création et modification de fiche, sceau, vérification de domaine, adhésion devenue active ou révoquée, fusion. **Aucun trigger n'est ajouté au modèle** : le score est une aide au tri, pas un invariant, et un trigger sur les quatre tables qui l'alimentent renchérirait des chemins d'écriture fréquents (obligation n° 4 relevée en A11).
- **FR-064**: Ces recalculs DOIVENT porter une **clé d'unicité par organisation** afin que cent adhésions approuvées coup sur coup ne produisent pas cent recalculs.
- **FR-065**: La **projection analytique** DOIT être rafraîchie par un travail différé après les écritures qui la périment, sans jamais être rafraîchie dans la transaction qui les porte.

### Fusion

- **FR-066**: La fusion DOIT exiger la permission de fusion **sur la portée globale**. Il n'existe pas de fusion limitée à une édition : elle déplace des rattachements dans toutes les éditions à la fois.
- **FR-067**: L'aperçu DOIT être calculé **pour un sens donné** et recalculé à chaque inversion : le décompte n'est pas symétrique.
- **FR-068**: Le décompte DOIT être construit en parcourant le **registre des références** de la base, jamais une liste de tables écrite dans le code — le jour où un module s'y déclare, le décompte le compte sans modification.
- **FR-069**: Les **trois sorts** DOIVENT rester distincts : transférée, supprimée car déjà présente côté cible, supprimée par stratégie. Un chiffre unique mentirait sur les trois.
- **FR-070**: L'aperçu DOIT porter les **dénominations** et les **domaines** apportés, chacun disant s'il est déjà présent côté cible, et les **avertissements** qui doivent arrêter la main sans jamais bloquer — au premier rang, absorber une fiche portant le sceau dans une fiche qui ne l'a pas.
- **FR-071**: Le **nom de confirmation** DOIT être revérifié par l'API — masquer un bouton n'a jamais empêché une requête —, en ignorant la casse et les accents, et en acceptant aussi bien le nom légal que le sigle.
- **FR-072**: Le **motif** de fusion DOIT être obligatoire : c'est ce qu'on relira dans six mois.
- **FR-073**: Les **arbitrages de champ** sont une écriture de la fiche survivante, à faire **dans la même transaction** que la fusion. **Ils viennent APRÈS l'appel de fusion, et non avant** : tant que la fiche absorbée est vivante, l'unicité (nom normalisé, pays) interdit à la survivante de reprendre son nom légal. Cette précision **corrige l'ordre inscrit dans l'obligation n° 1 relevée en A11**.
- **FR-074**: L'**adresse d'URL** figure au comparatif mais **n'est pas arbitrable** : elle est unique sans condition de statut, et la fiche absorbée garde la sienne — c'est elle qui fait que les anciennes adresses continuent de mener quelque part. Retenir celle de la source DOIT être refusé par un code stable **nommant le champ**.
- **FR-075**: La réponse DOIT rendre le **décompte réel** des lignes déplacées, par schéma, table et colonne, et la liste des champs effectivement appliqués.
- **FR-076**: Le service **NE DOIT PAS émettre** l'événement de fusion : la fonction de base l'émet elle-même. C'est le piège éprouvé en B1 avec l'anonymisation — deux lignes s'écriraient sans qu'aucune erreur ne le signale.
- **FR-077**: Une cible **elle-même fusionnée** DOIT produire le refus du trigger, **repris mot pour mot** ; une fusion concurrente DOIT rendre « déjà fusionnée » et non une erreur de base.
- **FR-078**: Une fusion NE DOIT PAS marquer la paire de la file : la fonction de base le fait déjà.

### Événements, erreurs et documentation

- **FR-079**: Les changements d'état qui intéressent d'autres modules DOIVENT être annoncés par l'émission d'un événement **dans la même transaction** : fiche créée, fiche vérifiée, adhésion demandée, invitée, approuvée, révoquée. La fusion est annoncée par la base.
- **FR-080**: Les courriels — invitation, demande reçue, adhésion approuvée — DOIVENT partir du binaire des travaux différés à partir de ces événements, par le contrat d'envoi du noyau. **Aucun appel direct à un autre module.**
- **FR-081**: Toute erreur DOIT porter un **code stable** et un **message français** ; une erreur de validation DOIT désigner le champ fautif ; aucune erreur NE DOIT divulguer l'existence d'une donnée hors du périmètre de l'appelant.
- **FR-082**: Un refus **prévu par le contrat du front** comme membre d'union — « déjà membre », « nom déjà pris », « déjà invitée », « domaine déjà pris », « nom de confirmation incorrect », « déjà fusionnée » — DOIT sortir en **200** avec son discriminant. Les autres sortent en statut d'erreur avec corps d'erreur.
- **FR-083**: Toute écriture DOIT poser l'acteur et l'identifiant de requête **avant** sa première modification, par l'unique porte d'écriture du noyau.
- **FR-084**: La documentation OpenAPI DOIT être **engendrée** depuis le code, couvrir chaque route livrée et **chaque code d'erreur stable** ajouté par ce module.

---

### Key Entities *(include if feature involves data)*

Toutes existent déjà dans `docs/database/`. Aucune n'est créée, aucune n'est modifiée.

- **Organisation** (`org.organizations`) — la fiche. Porte son **statut** (en attente de rapprochement, de référence, absorbée, archivée, refusée), son **pointeur de fusion**, son **sceau de vérification** — distinct du statut —, son **score de confiance**, et deux formes canoniques calculées par la base. Deux fiches vivantes ne peuvent pas porter le même nom normalisé dans le même pays.
- **Dénomination** (`org.organization_names`) — toutes les façons de désigner l'organisation : nom légal, sigle, nom court, traduction, ancien nom, faute de frappe connue. Le nom légal et le sigle y sont recopiés par trigger. Une dénomination non confirmée **sert la recherche sans s'afficher**.
- **Domaine** (`org.organization_domains`) — le signal de dédoublonnage le plus fiable et le mécanisme de rattachement automatique. Un domaine **vérifié** appartient à une seule fiche.
- **Domaine de messagerie grand public** (`org.public_email_domains`) — vingt domaines qui ne prouvent aucune appartenance.
- **Adhésion** (`org.memberships`) — une personne dans une organisation, avec son rôle, son état et sa **direction d'attente**. Une seule ligne par organisation et par personne ; une seule organisation principale par personne.
- **Paire de doublons présumés** (`org.duplicate_candidates`) — deux fiches, un score, des motifs, et une décision. La paire est ordonnée par construction, et enregistrée une seule fois.
- **Registre des références** (`org.organization_references`) — les colonnes de **toute la plateforme** qui pointent vers une organisation, avec leur stratégie et leurs colonnes d'unicité. Huit fichiers du modèle l'alimentent ; la fusion le parcourt.
- **Journal des fusions** (`org.merge_log`) — la fiche absorbée telle qu'elle était, sa cible, l'auteur, le motif et le décompte réel.
- **Fiche de performance** (`analytics.mv_organization_scorecard`) — projection **matérialisée** : membres, dépôts, acceptations, ratio, séances, publications, dernière activité.
- **Permissions** (`identity.permissions`) — consultation, gestion et fusion des organisations. La consultation est détenue par le rôle d'utilisateur ordinaire ; la fusion, par l'administrateur seul.
- **Jeton à usage unique** (`identity.one_time_tokens`, finalité invitation) — le lien par lequel une invitation s'honore.
- **Événement de domaine** (`platform.outbox_events`) et **travail différé** (`platform.jobs`) — annonces et travaux de fond du module.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Les cinq façons de désigner une même organisation — nom complet, début du nom complet, sigle, traduction, ancien nom — ramènent **la même fiche**, une seule fois, sur un jeu de 5 000 organisations.
- **SC-002**: La recherche répond en **moins de 150 ms** au 95ᵉ centile sur 5 000 organisations, mesuré côté serveur, avec la limite par défaut.
- **SC-003**: Une recherche d'utilisateur ne ramène **aucune** fiche dépourvue de ressemblance de dénomination avec le terme saisi — vérifié sur le cas qui a fait naître l'écart n° 23.
- **SC-004**: La lecture de revue des doublons ramène, elle, **toutes** les fiches partageant le domaine, y compris sans ressemblance de nom : les deux lectures rendent des résultats **différents** sur la même requête, et c'est l'attendu.
- **SC-005**: Cent créations concurrentes du même nom dans le même pays produisent **une seule** fiche, les autres recevant « nom déjà pris » avec la fiche en cause.
- **SC-006**: Une personne dont le domaine est vérifié en rattachement automatique est membre **actif** dès sa demande ; dans tous les autres cas l'adhésion est **en attente** — vérifié sur les deux comptes des données simulées.
- **SC-007**: Aucune invitation ne peut être approuvée par l'organisation qui l'a émise, et aucune demande ne peut être acceptée par le lien d'invitation — sur l'ensemble des combinaisons.
- **SC-008**: Une adhésion refusée, puis redemandée, puis refusée à nouveau ne produit **jamais** plus d'une ligne, et son histoire reste lisible.
- **SC-009**: Un administrateur détaché sur une édition n'atteint **aucune** organisation hors de son périmètre, y compris en forgeant l'identifiant dans l'URL ; une personne sans droit d'administration reçoit **un refus**, jamais une liste vide.
- **SC-010**: Le décompte annoncé par l'aperçu de fusion **correspond exactement** au décompte rendu après la fusion, ligne de registre par ligne de registre — écart de zéro sur chacune.
- **SC-011**: Après une fusion, la fiche absorbée reste consultable et pointe vers la fiche vivante, **aucune** donnée rattachée n'est perdue, et l'outbox porte **exactement un** événement de fusion.
- **SC-012**: Une fusion dont l'arbitrage de champ échoue ne laisse **aucune** écriture : ni fiche absorbée, ni rattachement déplacé, ni trace au journal.
- **SC-013**: Un balayage rejoué dix fois sur la même base ne crée **aucune** paire en double et ne ressuscite **aucune** paire arbitrée.
- **SC-014**: Le score de confiance d'une fiche vérifiée est à jour **au premier rechargement** de la liste, et cent adhésions approuvées coup sur coup produisent **un seul** recalcul par organisation.
- **SC-015**: Les chemins et les noms de champs servis correspondent **exactement** à ce que le front consomme : le basculement des données simulées vers l'API ne demande aucune modification d'écran.
- **SC-016**: La documentation OpenAPI engendrée couvre **100 %** des routes livrées et **chaque** code d'erreur stable ajouté par ce module.
- **SC-017**: La porte de qualité passe : schéma rechargé de zéro, rapport de frontières de modules vide, aucune arête entre deux crates de module, mise en forme et analyse statique sans avertissement, tests d'intégration au vert sur base réelle.
- **SC-018**: Aucun fichier de `backend/` ne dépasse **1000 lignes**.

---

## Assumptions

Décisions prises faute de précision dans la demande, et à confirmer au besoin.

- **Le rôle de qui rejoint et de qui crée est confirmé tel que l'écran l'a tranché** (écart n° 24) : qui **crée** une fiche en devient référent, qui **rejoint** reste membre. Aucune autre règle n'attribue le rôle de référent.
- **Le départ du dernier référent est traité par le refus** (FR-041) plutôt que par l'abandon silencieux. C'est l'option qui ne perd rien : une fiche sans référent est une fiche dont les demandes d'adhésion ne seront jamais traitées, et personne ne s'en apercevrait. **À confirmer auprès du commanditaire.**
- **L'ordre des deux écritures de la fusion est inversé par rapport à l'obligation inscrite en A11** (FR-073). L'obligation disait « avant l'appel » ; l'unicité du nom sur les fiches vivantes l'interdit. L'exigence de la même transaction, elle, est conservée intacte — c'est ce qui la motivait.
- **L'adresse d'URL n'est pas arbitrable dans une fusion** (FR-074). Le contrat du front la range parmi les champs comparés ; l'API la compare et refuse de la déplacer. À refermer côté écran au raccordement (B7), ou à traiter par une table d'alias d'adresses si le besoin est confirmé.
- **Seuil d'entrée dans la file des doublons : 60.** Valeur alignée sur les données simulées, où une paire à 61,0 a été jugée digne d'être proposée puis écartée à la main. C'est un réglage, changeable sans redéploiement.
- **Le balayage de détection passe une fois par jour**, hors heures ouvrées, et se replanifie lui-même. La fréquence est un réglage.
- **La vérification d'un domaine par enregistrement DNS ou par courriel n'entre pas dans ce jalon** : seule la vérification manuelle est livrée, comme le contrat du front l'annonce déjà.
- **La recherche n'exige qu'une session**, non la permission de consultation des organisations — que rien n'attribue aujourd'hui à un nouvel inscrit. Le jour où l'inscription attribuera le rôle d'utilisateur ordinaire, cette exigence pourra être resserrée sans changer le contrat.
- **La liste ouverte des organisations est bornée** (défaut 50, maximum 200) et n'est consommée que par la page de guide de style. Elle est livrée pour ne pas la casser, non parce qu'un écran métier en dépend.
- **Le nom traduit d'une organisation n'est pas encore affiché à la place du nom légal** : c'est la question n° 2 en attente auprès du commanditaire depuis le 16/08. Les traductions sont **collectées et cherchables** ; leur affichage se décidera avec l'arbitrage.
- **Les courriels du module partent par la chaîne éprouvée en B1** — file de travaux, worker, route privée du site. Les modèles de message multilingues et le suivi des envois appartiennent au module Engagement (B6).
- **Aucun écran ne consomme aujourd'hui les lectures de dénominations, de domaines et de membres déclarées isolément** dans la couche d'accès du front : elles sont servies par la fiche complète du back-office et par l'espace organisation. Elles ne sont pas livrées séparément.
- **Dépendance d'environnement** : les requêtes étant vérifiées à la compilation, la base doit être démarrée et le schéma chargé pour construire. Le jeu de 5 000 organisations nécessaire à SC-002 est semé par le test lui-même, jamais versionné.

---

## Vérifications faites en écrivant cette spécification

- **`040_organizations.sql` n'a besoin d'aucune modification.** Relecture intégrale : les quatre verrous sont complets, la fonction de recherche a été corrigée le 17/08 (écarts n° 21 et 22) et le registre des références le 18/08 (dédoublonnage des domaines). L'écart n° 23 se traite **au-dessus** de la fonction, comme le prompt l'exige.
- **Le registre `org.organization_references` est complet en base**, contrairement à ce que le seul fichier `040` laisse croire : **huit fichiers SQL** y insèrent leurs lignes (040, 050, 060, 070, 075, 080, 090, 125), pour 18 entrées au total. Les données simulées les recopient à l'identique. Le décompte de fusion lit donc déjà toutes les tables de la plateforme.
- **La permission de consultation des organisations est détenue par le rôle d'utilisateur ordinaire** (`030_identity.sql` § 6, ligne `('standard', 'org.organization.read')`). Elle ne peut donc pas garder à elle seule le back-office : la conjonction avec un périmètre d'administration non vide est nécessaire, et suffit — le rôle d'utilisateur ordinaire n'accorde pas la permission sur laquelle le périmètre est calculé.
- **Rien n'attribue aujourd'hui le rôle d'utilisateur ordinaire.** Vérifié dans le service d'inscription livré en B1 et dans le semis : seul un super-administrateur global est semé. Garder la recherche derrière cette permission refuserait donc **tout nouvel inscrit** — d'où FR-014.
- **L'unicité des adhésions porte aussi sur les révocations.** `ux_memberships` est sur (organisation, personne), sans condition de statut : les données simulées, qui filtrent les révocations avant de créer une ligne, produiraient une violation de contrainte contre la vraie base. D'où FR-023.
- **L'unicité du nom ne porte que sur les fiches vivantes**, celle de l'adresse d'URL sur **toutes** les fiches. C'est cette asymétrie qui impose l'ordre des écritures de la fusion (FR-073) et qui interdit d'arbitrer l'adresse (FR-074).
- **La fonction de fusion émet elle-même son événement** et marque elle-même la paire de la file. Le service ne doit refaire ni l'un ni l'autre — c'est exactement le piège qui a coûté une correction en B1 avec l'anonymisation.
- **La fonction de fusion s'exécute avec les droits de son propriétaire et lit l'acteur dans le contexte de transaction** : elle doit donc être appelée depuis l'unique porte d'écriture du noyau, faute de quoi le journal des fusions n'aurait pas d'auteur.
- **Le score de confiance n'est appelé par aucun trigger** : la colonne est ordinaire et le commentaire du modèle dit « recalculé par le worker » sans que rien ne le garantisse. Confirmé par relecture du § 7 — d'où FR-063.
- **Le motif « ressemblance de nom » n'est posé qu'au-dessus de 0,3**, alors que l'opérateur trigramme fait entrer une ligne **à partir de** 0,3. La frontière est étroite mais réelle, et la lecture filtrée s'aligne sur le motif plutôt que sur le score, pour que l'API et l'écran écartent exactement les mêmes lignes.
- **L'anti-rebond du front est de 300 ms** (`UiSearchInput`), la valeur que le modèle annonce au § 5. Il est déjà en place ; ce jalon n'a rien à y ajouter.
- **Le score maximal atteignable est 175** (100 de ressemblance, 40 de domaine, 10 de pays, 25 de sigle exact) : la colonne de score des paires, en `numeric(5,1)`, le porte sans risque.
- **Aucun code Rust du module n'existe** : `backend/crates/modules/` ne contient que `identity`. Cette spécification ajoute un crate, elle n'en complète aucun.
