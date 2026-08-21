# Feature Specification: Propositions (B4)

**Feature Branch**: `004-propositions`

**Created**: 2026-08-20

**Status**: Draft

**Input**: User description: "Module PROPOSITIONS de l'API ePavillon v2 (Rust + Actix Web + SQLx). Soumission avec brouillon, co-organisateurs, intervenants, documents. Machine à états : le code ne réimplémente pas les transitions, il lit `programme.proposal_transitions_allowed`. Évaluation, scores, commentaires à visibilité contrôlée, historique. Plus les exigences issues des écarts n° 3, 4, 8, 27 à 32, 35, 37, 38 et 39. Livrable : `backend/crates/modules/programme`, monté par `api` et `worker`, qui existent depuis B1."

---

## Contexte

**Ce module est celui qui reçoit les dossiers de la COP31.** B3 a livré l'édition, l'appel et sa grille ; il n'y a plus rien entre une organisation et son dépôt, sinon ce module. C'est aussi le plus dense du jalon : cinq écrans du front en dépendent — le formulaire de soumission (A4), l'espace organisation (A5), la liste du back-office (A7), la fiche d'évaluation (A8) et, pour partie, le tableau de bord (A6).

Le modèle fait autorité et **n'est pas modifié**. `docs/database/070_programme_proposals.sql` a été relu en entier : ses huit sections portent le cycle de vie en données, le dossier, le journal des transitions et ses deux gardes, la co-organisation, les intervenants et les pièces, l'évaluation avec sa consolidation, les échanges à visibilité explicite, la vue de pilotage du comité, l'historique champ par champ. S'y ajoutent `060_events.sql` § 5 à 7 (l'appel, ses règles de recevabilité, sa grille, son comité), `020_reference.sql` § 4 (les thématiques, hors de la proposition), `030_identity.sql` § 6 (les quatre permissions `programme.*` et le périmètre d'administration), `050_media.sql` § 8 (le rôle de pièce jointe et ses bornes), `010_platform.sql` (audit, historique d'entité, outbox, file de travaux) et `910_migration_v1.sql` § 6.3 (la reprise des activités de la v1).

**Une décision structurante du modèle gouverne tout ce qui suit**, et elle est écrite en tête du fichier SQL : **la proposition est un DOSSIER, la session est une OCCURRENCE PROGRAMMÉE**. La v1 n'avait qu'une table pour les deux, et sa colonne unique de statut mélangeait des états de dossier et des états de diffusion. Il en découle une règle que ce module ne franchit jamais : **modifier un dossier retenu ne touche pas la séance qui en est née** — ni son créneau, ni sa salle, ni ses inscrits, ni ses rappels. La reprise d'un titre ou d'un horaire au programme est un geste de l'IFDD, pas un effet de bord d'une correction de forme.

**Une seconde décision du modèle porte tout le reste : la machine à états est une DONNÉE.** `programme.proposal_transitions_allowed` déclare quatorze chemins, chacun avec la permission qu'il exige, le droit du porteur et l'obligation de motif. Un déclencheur les vérifie, journalise ce qui passe et émet l'événement de domaine. Le service **ne rejoue pas ce graphe à l'écriture** : il tente la transition et traduit le refus.

Le front existe depuis le 18/08 et consomme des données simulées. Ses contrats — `frontend/app/types/programme/{proposal,review}.ts`, `types/proposal-form.ts`, `types/admin-proposals.ts`, `types/admin-review.ts`, `types/organization-workspace.ts`, `types/views.ts` — et les chemins déclarés dans `composables/api/proposals.ts`, `composables/api/proposal-review.ts` et `composables/api/organization-workspace.ts` **sont le contrat de cette API**. Ils ne se renégocient pas.

Le socle existe depuis B1, les frontières depuis B2, l'édition et l'appel depuis B3 : `kernel` (contexte de requête, erreurs à code stable, unique porte d'écriture, garde d'autorisation testant permission **et** portée, file de travaux, jetons, contrat d'envoi de courriel), `contracts`, `api`, `worker`. **Ce module ne les réinvente pas et ne dépend d'aucun autre crate de module.** La règle de frontière posée en B2 s'applique telle quelle : *un module lit hors de son schéma quand la question porte sur ses propres entités, il n'y écrit jamais, et il n'appelle jamais un autre module.*

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Une organisation dépose son dossier, et son brouillon la suit (Priority: P1)

Une chargée de projet ouvre le formulaire de dépôt de la COP31. Dès sa première frappe, un dossier existe et porte son numéro — celui qu'elle citera au téléphone. Elle le remplit en plusieurs fois, ajoute ses co-organisateurs, déclare ses intervenants, choisit ses thématiques, propose un créneau, puis dépose. La confirmation affiche le **même** numéro.

**Why this priority**: c'est la raison d'être du jalon. Sans cette histoire, l'appel de la COP31 est ouvert et personne ne peut y répondre.

**Independent Test**: créer un brouillon sur un appel ouvert, l'enregistrer trois fois, le compléter, le déposer, et constater que le numéro n'a pas changé ; recommencer sur un appel clos et sur une organisation ayant atteint son plafond.

**Acceptance Scenarios**:

1. **Given** une personne rattachée à une organisation active et détenant le droit de soumettre, **When** elle enregistre un dossier pour la première fois, **Then** le dossier est créé **en brouillon**, la réponse porte son **numéro de dossier** et l'instant d'écriture daté par le serveur.
2. **Given** ce brouillon, **When** elle l'enregistre à nouveau, **Then** le numéro est **inchangé** et aucun second dossier n'est créé.
3. **Given** un tout premier enregistrement où le titre est encore vide, **When** il arrive, **Then** le dossier est créé quand même — l'adresse d'URL qu'exige le modèle est **dérivée par le service**, jamais demandée au client.
4. **Given** deux dossiers de la même édition portant le même titre, **When** le second est enregistré, **Then** il est accepté : l'adresse d'URL est rendue unique par le service, sans que la personne ait à renommer son activité.
5. **Given** un dossier complet sur un appel **ouvert**, **When** il est déposé, **Then** il passe en « déposé », la réponse porte le nombre de revues attendues et la date d'annonce des résultats, **lues sur l'appel**.
6. **Given** un appel dont l'échéance est passée entre l'ouverture de la page et le clic, **When** le dépôt arrive, **Then** il est refusé par une **réponse nommée** portant l'échéance, et non par une erreur technique.
7. **Given** une organisation ayant atteint le plafond de dossiers de l'appel, **When** elle en dépose un de plus, **Then** le refus le dit et porte le plafond.
8. **Given** un appel réservé aux organisations vérifiées et une organisation qui ne l'est pas, **When** le dépôt arrive, **Then** il est refusé en le disant.
9. **Given** un dossier déclarant moins d'intervenants que l'appel n'en demande, ou plus qu'il n'en accepte, **When** il est déposé, **Then** il est refusé sur ce point : **aucun déclencheur ne porte cette règle**, c'est l'API qui la tient.
10. **Given** un dossier dont la durée sort des bornes de l'appel, ou dont le créneau souhaité déborde de la plage d'accueil du pavillon, **When** il est déposé, **Then** chacun de ces refus porte son propre champ.
11. **Given** un texte de présentation contenant un script ou un attribut d'événement, **When** il est enregistré, **Then** il est **assaini à l'écriture** et ce qui est stocké ne porte que les balises de la liste blanche.
12. **Given** un dossier dont un texte dépasse la longueur admise, **When** il est enregistré, **Then** le refus nomme le champ et la limite.
13. **Given** un créneau souhaité saisi en heure murale, **When** il est enregistré depuis un fuseau quelconque, **Then** l'instant retenu est celui du **fuseau de l'édition**.

---

### User Story 2 — La machine à états est lue, jamais réécrite (Priority: P1)

Devant un dossier, chacun ne voit que les actions qui lui sont réellement ouvertes : le déposant peut retirer, le comité peut demander des corrections, l'équipe peut retenir ou rejeter. Ce ne sont pas des règles écrites dans un écran — elles sont lues dans la base, croisées avec les permissions de la personne et sa qualité de porteur.

**Why this priority**: c'est l'écart n° 4, et c'est ce que la mise en données du graphe voulait éviter. Un front qui réimplémente les transitions diverge au premier chemin ajouté, et le contournement d'un contrôle d'accès devient une ligne de JavaScript.

**Independent Test**: demander les transitions offertes sur un même dossier successivement comme déposant, comme membre du comité et comme administrateur, et constater trois réponses différentes ; tenter ensuite une transition non offerte et constater le refus.

**Acceptance Scenarios**:

1. **Given** un dossier en brouillon et son déposant, **When** il demande les transitions offertes, **Then** il reçoit « déposer » et « retirer », chacune avec le fait qu'un motif soit exigé ou non.
2. **Given** le même dossier et un membre du comité, **When** il demande les transitions offertes, **Then** il n'en reçoit **aucune** : aucun chemin depuis le brouillon ne lui est ouvert.
3. **Given** un dossier en évaluation et une personne détenant le droit de décider, **When** elle demande les transitions offertes, **Then** elle reçoit « retenir » et « rejeter », la seconde signalée comme exigeant un motif.
4. **Given** un dossier en évaluation et une personne détenant le droit de noter mais pas celui de décider, **When** elle demande les transitions, **Then** elle reçoit « demander des corrections » et rien d'autre.
5. **Given** une transition qui n'est pas déclarée depuis l'état courant, **When** elle est tentée, **Then** le refus vient de la base, est traduit en français, porte un code stable, et **ne ressemble pas à une panne**.
6. **Given** une transition qui exige un motif, **When** elle est tentée sans motif, **Then** le refus le dit et désigne le champ du motif.
7. **Given** une transition acceptée, **When** elle aboutit, **Then** le journal du dossier porte une ligne de plus, avec son auteur, son motif et son instant.
8. **Given** une même personne et deux dossiers dans deux éditions dont elle n'administre qu'une, **When** elle demande les transitions, **Then** le dossier hors périmètre ne lui en offre aucune.

---

### User Story 3 — Le comité pilote sa liste, dans son périmètre (Priority: P1)

L'équipe de sélection ouvre la liste des dossiers de son édition : quarante lignes, leur avancement, leur note, leur rang, ce qui est en retard, ce qu'elle n'a pas encore ouvert. Elle filtre, elle trie, elle confie une sélection à un membre du comité, elle change l'état de plusieurs dossiers d'un coup — et l'API lui dit ce qui n'a pas suivi, dossier par dossier.

**Why this priority**: c'est l'écran de travail quotidien du jalon, et le premier endroit où la règle métier n° 8 s'applique à ce module. Une liste non bornée ouvre les dossiers de toutes les éditions à qui n'en administre qu'une.

**Independent Test**: charger la liste comme administratrice globale, puis comme responsable détachée sur une seule édition, puis avec un compte sans aucun droit ; forger l'identifiant d'une autre édition ; lancer les deux actions groupées sur une sélection hétérogène.

**Acceptance Scenarios**:

1. **Given** une personne administrant l'ensemble de la plateforme, **When** elle demande la liste d'une édition, **Then** elle reçoit les lignes, les facettes comptées **sur le même jeu de lignes**, le fuseau et la ville de l'édition, l'échéance effective de l'appel et le nombre de revues attendues.
2. **Given** une personne **sans aucun** droit d'administration, **When** elle demande la liste, **Then** elle reçoit un **refus explicite**, jamais une liste vide.
3. **Given** une personne détachée sur une édition, **When** elle demande la liste d'une **autre** édition en forgeant l'identifiant, **Then** elle est refusée.
4. **Given** la liste rendue, **When** on lit le titre d'une ligne, **Then** il vient **deux fois** — le document multilingue brut, et sa résolution française réservée au tri, au filtrage et à l'export.
5. **Given** une personne qui n'a jamais ouvert cinq dossiers de l'édition, **When** elle demande la liste, **Then** ces cinq identifiants lui sont désignés, et à elle seule.
6. **Given** une sélection de douze dossiers dont trois sont déjà confiés à la même personne et un dont elle s'est déportée, **When** l'affectation groupée est lancée, **Then** la réponse énumère ce qui a été fait **et** ce qui a été écarté, chaque écart portant sa raison et le numéro de son dossier.
7. **Given** une sélection hétérogène et une transition qui n'est pas ouverte à tous, **When** le changement d'état groupé est lancé, **Then** chaque dossier suit ou est écarté nommément, et un motif manquant est une raison d'écart comme une autre.
8. **Given** un dossier effacé, **When** la liste est demandée, **Then** il n'y figure pas.

---

### User Story 4 — La fiche d'évaluation compose tout, et le voile tient (Priority: P1)

Un membre du comité ouvre un dossier : le dossier lui-même, son édition, son appel, sa grille, ses organisations et leur historique de participation, ses intervenants, ses pièces, son journal, ses échanges, l'avancement du comité, sa propre revue. Tant qu'il n'a pas déposé la sienne, il ne reçoit **pas** celles de ses pairs.

**Why this priority**: c'est l'écran le plus dense du jalon, et le seul où une erreur de composition fait sortir une note interne. Le voile de l'évaluation en aveugle et le filtrage des trois visibilités ne peuvent être portés par aucune vue : ils dépendent de qui regarde.

**Independent Test**: ouvrir la même fiche comme membre du comité affecté n'ayant pas noté, comme membre ayant noté, comme administrateur qui décide sans noter, et comparer ce que chacun reçoit ; vérifier qu'aucune réponse ne contient une note qu'elle ne devait pas porter.

**Acceptance Scenarios**:

1. **Given** un appel en évaluation en aveugle, un membre affecté et sa revue non déposée, **When** il ouvre la fiche, **Then** les revues de ses pairs **ne sont pas dans la réponse**, et le nombre de revues qu'il ne voit pas lui est dit.
2. **Given** le même membre, **When** il dépose sa revue, **Then** les revues de ses pairs lui sont rendues à la lecture suivante.
3. **Given** un administrateur qui décide sans noter, **When** il ouvre la fiche, **Then** il reçoit les revues : l'ancrage vise celui qui va poser une note.
4. **Given** un appel sans évaluation en aveugle, **When** un membre non encore noteur ouvre la fiche, **Then** il reçoit les revues.
5. **Given** une note posée sur un critère, **When** elle dépasse le maximum de **ce** critère, **Then** elle est refusée en nommant le critère et son maximum.
6. **Given** une revue enregistrée sans être déposée, **When** on lit les agrégats du dossier, **Then** elle n'y compte pas et aucun pair ne la voit.
7. **Given** une revue déposée, **When** elle aboutit, **Then** les agrégats du dossier sont **recalculés dans la même transaction** et rendus dans la réponse — moyenne, moyenne pondérée, nombre de revues, élimination.
8. **Given** un critère éliminatoire noté zéro sur une revue déposée, **When** les agrégats sont relus, **Then** le dossier est marqué éliminé.
9. **Given** une personne détenant le droit de noter mais **non affectée** au dossier, **When** elle tente d'enregistrer une revue, **Then** elle est refusée : lire un dossier qu'on ne lui a pas confié est permis, le noter ne l'est pas.
10. **Given** un membre qui se déporte, **When** il envoie son déport **sans motif**, **Then** il est refusé — la trace de l'impartialité est le sujet même de ce geste.
11. **Given** un message écrit par le comité en visibilité « comité », **When** le déposant lit le fil de son dossier, **Then** ce message **ne lui parvient pas**.
12. **Given** une demande de correction, **When** elle est écrite, **Then** elle est **partagée avec le déposant**, quelle que soit la visibilité demandée : une demande qu'il ne verrait pas bloquerait son dossier sans qu'il sache pourquoi.
13. **Given** une note personnelle, **When** un autre membre du comité lit le fil, **Then** elle ne lui parvient pas.
14. **Given** un dossier jamais ouvert par cette personne, **When** elle l'ouvre, **Then** l'accusé de lecture est posé et la réponse dit l'état **d'avant** la visite.
15. **Given** une pièce jointe en quarantaine, **When** la fiche est rendue, **Then** la pièce est **annoncée** sans adresse de téléchargement, plutôt qu'absente ou proposée en lien mort.

---

### User Story 5 — L'organisation suit son dossier, sans jamais voir sa note (Priority: P2)

Sur son espace, une organisation lit où en est chacun de ses dossiers : l'édition visée, la frise de son avancement, le nombre de corrections qui l'attendent, les séances programmées si le dossier a été retenu. Elle répond aux demandes du comité et marque ce qu'elle a corrigé. Elle ne voit ni note, ni rang, ni le nom de qui l'évalue.

**Why this priority**: c'est l'écart n° 8, ouvert depuis le 16/08, et la seule question du modèle qui ait été tranchée par un écran. Elle ne bloque pas l'ouverture de l'appel, mais elle décide de ce qu'un déposant apprend de l'évaluation.

**Independent Test**: charger l'espace d'une organisation ayant des dossiers dans deux éditions, en balayer la réponse à la recherche d'une note, d'un rang ou d'un nom de membre du comité ; poser puis retirer une résolution.

**Acceptance Scenarios**:

1. **Given** une organisation ayant déposé, **When** son espace est chargé, **Then** la réponse porte ses dossiers, leur édition, leur journal de transitions, le nombre de leurs demandes de correction **ouvertes** et leurs séances programmées.
2. **Given** cette même réponse, **When** on la parcourt entièrement, **Then** elle ne contient **ni note, ni note pondérée, ni rang, ni nom de membre du comité, ni liste nominative d'inscrits**.
3. **Given** un dossier dont le comité a écrit trois messages, dont un seul partagé, **When** le fil est demandé côté organisation, **Then** **un seul** message est rendu, et le filtre est appliqué **à la source**.
4. **Given** une demande de correction, **When** le déposant la marque résolue, **Then** la résolution est enregistrée avec son auteur et son instant.
5. **Given** une résolution posée par erreur, **When** le déposant la retire, **Then** elle est retirée, sans qu'un courriel à l'IFDD soit nécessaire.
6. **Given** une personne du comité, **When** elle pose ou retire une résolution de son côté, **Then** elle en a le droit par sa permission, et non par le formulaire qu'elle utilise.
7. **Given** une personne étrangère à l'organisation, **When** elle demande cet espace, **Then** elle est refusée.
8. **Given** une réponse du déposant à un message du comité, **When** elle est écrite, **Then** elle est **toujours** partagée et n'est **jamais** une demande de correction.

---

### User Story 6 — Un dossier se rouvre, se corrige et se renvoie (Priority: P2)

Le comité demande des corrections. L'organisation rouvre son dossier dans le formulaire — tel qu'elle l'avait saisi, en français, avec son créneau à l'heure de l'édition —, corrige, et le renvoie. L'appel est clos depuis trois semaines : cela n'empêche rien, parce que le renvoi n'est pas un dépôt.

**Why this priority**: ce sont les écarts n° 38 et n° 39. Le premier a déjà coûté un dossier définitivement bloqué en base ; il est corrigé dans le SQL et doit l'être aussi dans le contrat, faute de quoi le contrôle revient par la porte.

**Independent Test**: rouvrir un dossier déposé il y a deux mois par quelqu'un d'autre de la même organisation, comparer champ à champ avec ce qui avait été saisi, le renvoyer sur un appel **clos**, et vérifier qu'une séance déjà programmée n'a pas bougé.

**Acceptance Scenarios**:

1. **Given** un dossier existant, **When** on demande sa forme éditable, **Then** on reçoit ce que le formulaire attend : les textes ramenés à leur français, le créneau redevenu une **heure murale dans le fuseau de l'édition**, les co-organisateurs, les intervenants, les thématiques par leurs codes et les pièces.
2. **Given** un intervenant qui possède un compte, **When** le dossier est rouvert, **Then** il est **signalé comme tel**, ce qui verrouille son identité côté formulaire.
3. **Given** un créneau enregistré à 14:30 heure de l'édition, **When** le dossier est rouvert depuis un autre fuseau, **Then** l'heure rendue est **14:30**, et non l'heure locale du lecteur.
4. **Given** un dossier en « corrections demandées » et un appel **clos**, **When** il est renvoyé, **Then** le renvoi aboutit — la fenêtre de l'appel ne s'applique qu'au **premier** dépôt.
5. **Given** ce même renvoi, **When** l'organisation a atteint le plafond de dossiers, **Then** il est refusé : le plafond compte des dossiers, pas des envois.
6. **Given** un dossier en évaluation, **When** l'organisation le corrige, **Then** la correction est enregistrée **sans changer son état** : corriger n'est pas déposer.
7. **Given** un dossier **retenu** dont une séance est programmée, **When** son titre et son créneau souhaité sont corrigés, **Then** la séance conserve son créneau, sa salle, ses inscrits et ses rappels — **rien n'est propagé**.
8. **Given** un dossier rejeté, retiré ou annulé, **When** l'organisation tente de le modifier, **Then** elle est refusée : il n'est plus en course, et aucune transition ne l'y ramène de son initiative.
9. **Given** une édition terminée, **When** l'organisation tente de modifier un de ses dossiers, **Then** elle est refusée.

---

### User Story 7 — Les pièces du dossier existent, et ce qu'elles montrent est borné (Priority: P2)

Une organisation joint une pièce à son dossier ; le comité la consulte. Une pièce peut être publique une fois l'activité publiée, ou rester interne au dossier d'évaluation.

**Why this priority**: le modèle porte la table et le rôle de téléversement depuis l'origine, la fiche d'évaluation les affiche, et l'étape du formulaire est seulement **masquée** — pas supprimée. Le dépôt de fichier lui-même appartient à B6 ; le rattachement appartient ici.

**Independent Test**: rattacher un objet déjà stocké à un dossier, le lire depuis la fiche d'évaluation, le détacher, et vérifier que l'objet stocké n'a pas été détruit.

**Acceptance Scenarios**:

1. **Given** un objet déjà déposé sur le stockage, **When** il est rattaché au dossier, **Then** la pièce apparaît avec son titre, son type et son ordre.
2. **Given** une pièce rattachée, **When** elle est détachée, **Then** le lien disparaît et **l'objet stocké demeure** : le module ne détruit pas ce qu'il n'a pas créé.
3. **Given** une pièce dont l'objet n'est pas servi — quarantaine, purge, téléversement inachevé —, **When** le dossier est lu, **Then** la pièce est rendue **sans adresse**, et cette absence est ce qui commande l'avertissement.
4. **Given** une personne étrangère au dossier et sans droit de lecture générale, **When** elle demande les pièces, **Then** elle est refusée.

---

### User Story 8 — L'historique dit vrai, même pour les dossiers repris de la v1 (Priority: P3)

Le back-office lit l'historique d'un dossier champ par champ, et sa frise d'avancement se compose de son journal. Les dossiers repris de la v1 n'ont pas de journal : leur frise mentirait, en affichant « non concernée », barrée, l'étape d'évaluation d'un dossier pourtant retenu.

**Why this priority**: c'est l'écart n° 37. Il ne bloque rien tant que la reprise n'a pas eu lieu, mais chaque écran qui refera la déduction à sa façon la fera différemment.

**Independent Test**: lire l'historique d'un dossier modifié plusieurs fois ; créer un dossier sans journal, lancer la déduction, et constater que la frise devient exacte.

**Acceptance Scenarios**:

1. **Given** un dossier modifié dix fois, **When** son historique est demandé, **Then** chaque modification est rendue avec son auteur, son instant, son champ et ses deux valeurs — et **les champs recalculés en sont écartés**.
2. **Given** un dossier repris de la v1, sans aucune ligne de journal mais portant une date de création, une date de dépôt et une date de décision, **When** la déduction est lancée, **Then** trois lignes de journal sont semées, dans l'ordre, avec ces instants-là.
3. **Given** un dossier possédant déjà son journal, **When** la déduction est lancée, **Then** **rien n'est ajouté** : l'opération se rejoue sans dupliquer.
4. **Given** un dossier effacé, **When** l'effacement aboutit, **Then** ses liens de thématiques sont **purgés** — aucune contrainte référentielle ne le fera.

---

### Edge Cases

- **Un brouillon dont le titre reste vide.** L'adresse d'URL est exigée par le modèle et se dérive du titre : le service pose un repli, et la remplace dès qu'un titre existe.
- **Deux dossiers homonymes dans la même édition.** L'unicité de l'adresse d'URL est par édition ; le service la rend unique sans renommer l'activité.
- **Un dossier créé directement dans un état autre que « brouillon ».** Rien en base ne l'interdit ; le service crée **toujours** en brouillon.
- **Un retrait motivé par l'organisation.** Le motif est écrit dans la colonne de la décision du comité, qu'il écrase. Le journal, lui, garde chaque motif : c'est lui qu'on lit.
- **Une transition dont la permission n'est détenue par aucun rôle par défaut.** Demander des corrections exige le droit de noter, que le rôle d'administration ne détient pas d'origine. Le service n'invente rien : il offre ce que la base déclare.
- **Un porteur principal ajouté comme co-organisateur.** Refusé : le rôle de porteur est tenu en cohérence par un déclencheur, et l'accepter le ferait basculer en silence.
- **Un intervenant déjà déclaré sur le dossier, avec le même rôle.** Refusé par une réponse nommée, pas par une erreur de contrainte.
- **Une adresse inconnue pour un intervenant.** La personne est créée, et **ni son prénom ni son nom ne sont déduits de l'adresse**.
- **Un code de thématique qui n'appartient pas à la taxonomie attendue.** Refusé en nommant le code : le triplet d'entité est posé par le service et n'est jamais accepté du client.
- **Un client qui enverrait lui-même le schéma, la table ou l'identifiant d'entité des thématiques.** Ces champs n'existent pas dans le contrat ; ils sont ignorés ou refusés, jamais honorés.
- **Une note posée sur un critère qui n'appartient pas à l'appel du dossier.** Refusée.
- **Une revue déposée deux fois.** La seconde met à jour la première : une personne n'a qu'une revue par dossier.
- **Un membre du comité déporté qui tente de noter.** Refusé.
- **Un critère supprimé alors que des notes s'y appuient.** Le modèle détruirait les notes en cascade ; c'est B3 qui refuse, et ce module compte sur ce refus plutôt que de le redire.
- **Une demande de correction résolue puis rouverte par le comité.** Les deux gestes sont tracés ; le compteur de demandes ouvertes suit.
- **Un dossier effacé dont les thématiques resteraient rattachées.** Purge à l'effacement, logique compris.
- **Une reprise de la v1 relancée deux fois.** Aucune ligne de journal en double.

## Requirements *(mandatory)*

### Frontières, socle et contrat

- **FR-001**: Le module DOIT vivre dans un crate propre sous les modules métier, ne dépendre que du noyau et des contrats d'événements, et n'être atteint par aucun autre crate de module. Le graphe DOIT rester vérifiable mécaniquement.
- **FR-002**: Ses routes DOIVENT être montées par l'application existante et ses travaux différés déclarés auprès du service de travaux existant, sans que l'un ni l'autre soient réécrits ; le module est déjà déclaré au registre des modules de la base.
- **FR-003**: Toute écriture DOIT poser l'auteur et l'identifiant de requête en début de transaction, afin d'alimenter l'audit et l'historique.
- **FR-004**: Toute erreur rendue DOIT porter un code stable et un message français exploitable, et les refus métier DOIVENT être des réponses nommées plutôt que des pannes.
- **FR-005**: Toutes les requêtes DOIVENT être vérifiées à la compilation ; aucune requête ne DOIT être composée dynamiquement sans justification écrite.
- **FR-006**: Le module NE DOIT PAS réimplémenter un invariant déjà porté par la base ; il DOIT traduire les refus de la base en messages français.
- **FR-007**: Le module PEUT lire hors de son schéma lorsque la question porte sur ses propres entités — l'appel, l'édition, l'organisation, la personne, l'objet stocké — mais NE DOIT PAS y écrire, à la seule exception des liens de thématiques, dérogation bornée et isolée.
- **FR-008**: La documentation OpenAPI DOIT être engendrée depuis les gestionnaires et le catalogue d'erreurs, jamais écrite à la main.
- **FR-009**: Les noms de champs du contrat DOIVENT être ceux que le front déclare déjà ; aucun ne DOIT être renégocié.

### Le dossier, du brouillon au dépôt (US1)

- **FR-010**: Le premier enregistrement DOIT créer le dossier et rendre son numéro, son état et l'instant d'écriture daté par le serveur.
- **FR-011**: Le numéro de dossier DOIT être celui que la base attribue à l'insertion, et NE DOIT jamais être recalculé ni remplacé.
- **FR-012**: Le dossier DOIT être créé en **brouillon**, quel que soit l'état demandé.
- **FR-013**: L'adresse d'URL du dossier DOIT être dérivée par le service depuis le titre, avec un repli lorsque le titre est vide, et rendue unique au sein de l'édition.
- **FR-014**: Les textes du dossier DOIVENT être enregistrés avec leur français, la base exigeant cette clé.
- **FR-015**: Les publics visés DOIVENT être enregistrés **un par entrée**, jamais concaténés.
- **FR-016**: La présentation détaillée DOIT être assainie **à l'écriture** contre une liste blanche de balises structurelles ; ni police, ni couleur, ni script, ni attribut d'événement ne DOIVENT survivre.
- **FR-017**: Les longueurs maximales des textes DOIVENT être vérifiées par l'API, à un seul endroit, et le refus DOIT nommer le champ et la limite.
- **FR-018**: Le créneau souhaité DOIT être reçu en heure murale et converti dans le fuseau de l'édition.
- **FR-019**: La durée proposée DOIT être vérifiée contre les bornes de l'appel, distinctes du garde-fou large de la base.
- **FR-020**: Le créneau souhaité DOIT tenir dans la plage d'accueil quotidienne de l'appel, en heure locale de l'édition, **fin comprise**.
- **FR-021**: Le format DOIT appartenir aux formats admis par l'appel.
- **FR-022**: Le porteur principal DOIT être une organisation dont la personne est membre actif.
- **FR-023**: Les co-organisations DOIVENT être enregistrées avec un rôle autre que porteur, sans doublon, et le porteur principal NE DOIT PAS pouvoir y figurer.
- **FR-024**: Une co-organisation DOIT naître non confirmée : elle engage un tiers.
- **FR-025**: Un intervenant DOIT être rattaché à une personne existante lorsque son adresse est connue, et la personne DOIT être créée sinon.
- **FR-026**: Lors d'une création de personne, ni le prénom ni le nom NE DOIVENT être déduits de l'adresse.
- **FR-027**: L'identité d'une personne possédant un compte NE DOIT PAS être modifiable par le déposant ; la fonction et l'organisation déclarées pour l'activité, elles, DOIVENT rester modifiables.
- **FR-028**: Le nombre d'intervenants DOIT être vérifié contre les bornes de l'appel au dépôt, cette règle n'étant portée par aucun déclencheur.
- **FR-029**: Les thématiques DOIVENT être reçues sous forme de **codes seuls** ; le triplet d'entité DOIT être posé par le service et JAMAIS accepté depuis la requête.
- **FR-030**: Chaque code de thématique reçu DOIT être vérifié comme appartenant à la taxonomie attendue, et le refus DOIT nommer le code fautif.
- **FR-031**: Les thématiques DOIVENT être rendues sur le dossier sous le nom de champ que le front attend, lues par la fonction du référentiel.
- **FR-032**: Le contact du dossier DOIT valoir le déposant par défaut, règle explicite et non implicite.
- **FR-033**: Le dépôt DOIT rendre le nombre de revues attendues et la date d'annonce des résultats, lus sur l'appel.
- **FR-034**: Les refus de recevabilité — appel clos, plafond atteint, organisation non vérifiée — DOIVENT être des réponses nommées portant la valeur en cause.
- **FR-035**: Le dépôt DOIT exiger le droit de soumettre.
- **FR-036**: Un enregistrement de brouillon NE DOIT PAS changer l'état du dossier.

### La machine à états (US2)

- **FR-037**: Les règles de transition DOIVENT être servies telles qu'elles sont déclarées en base, sans être recopiées dans le code.
- **FR-038**: Les transitions offertes **pour ce dossier et cette personne** DOIVENT être exposées, en croisant les règles déclarées, la permission détenue **sur la portée de l'édition** et la qualité de porteur.
- **FR-039**: Chaque transition offerte DOIT porter son état cible et le fait qu'un motif soit exigé.
- **FR-040**: Le service NE DOIT PAS rejouer le graphe à l'écriture : il DOIT tenter la transition et traduire le refus de la base.
- **FR-041**: Le refus d'une transition non déclarée DOIT sortir sous un code stable distinct de celui d'un motif manquant.
- **FR-042**: Le motif d'une transition DOIT être lu dans le journal, la colonne du dossier n'en gardant que le dernier.
- **FR-043**: Le service NE DOIT PAS émettre lui-même les événements de domaine des changements d'état : le déclencheur les émet déjà.
- **FR-044**: Une transition demandée sur un dossier hors du périmètre d'administration DOIT être refusée.
- **FR-045**: Un changement d'état groupé DOIT rendre la liste des dossiers modifiés **et** celle des dossiers écartés, chacun avec son numéro et sa raison, parmi le vocabulaire d'écarts que le front déclare.

### La liste du comité (US3)

- **FR-046**: La liste DOIT être servie depuis la vue de pilotage du modèle, sans recomposer ses jointures.
- **FR-047**: Le titre DOIT être exposé sous ses **deux** noms — le document multilingue brut et sa résolution — et ces noms NE DOIVENT PAS changer.
- **FR-048**: La liste DOIT être bornée par le périmètre d'administration ; un périmètre vide DOIT valoir refus, jamais liste vide.
- **FR-049**: Un identifiant d'édition forgé hors périmètre DOIT être refusé de la même façon qu'un identifiant inexistant.
- **FR-050**: La réponse DOIT porter les facettes de chaque filtre avec leur décompte, mesurées **sur le même jeu de lignes** que la liste.
- **FR-051**: La réponse DOIT porter les dossiers que la personne connectée n'a jamais ouverts, comme une liste d'identifiants et non comme une colonne de ligne.
- **FR-052**: La réponse DOIT porter le fuseau de l'édition, sa ville, l'échéance effective de l'appel et le nombre de revues attendues.
- **FR-053**: La composition du comité et la charge de chacun DOIVENT être exposées.
- **FR-054**: L'affectation d'un membre du comité DOIT exiger la permission de gestion de l'appel, sur la portée de l'édition.
- **FR-055**: Une affectation groupée DOIT rendre les dossiers affectés et ceux qui ont été écartés, avec leur raison.
- **FR-056**: Les dossiers effacés NE DOIVENT PAS figurer dans la liste.

### La fiche d'évaluation (US4)

- **FR-057**: La fiche DOIT être composée **en une réponse** : dossier, édition, appel, grille, organisations et leur historique de participation, intervenants, pièces, thématiques, journal, historique, avancement du comité, revue de la personne, échanges, droits.
- **FR-058**: L'historique de participation d'une organisation NE DOIT exposer que le sous-ensemble de colonnes que la fiche utilise.
- **FR-059**: Le voile de l'évaluation en aveugle DOIT être appliqué **à la source** : les revues des pairs ne DOIVENT PAS figurer dans la réponse tant qu'il est baissé.
- **FR-060**: Le voile DOIT être baissé lorsque l'appel est en aveugle, que la personne est affectée et que sa revue n'est pas déposée ; il NE DOIT PAS l'être pour qui décide sans noter.
- **FR-061**: Le nombre de revues masquées DOIT être rendu : compter n'ancre pas.
- **FR-062**: La grille DOIT être celle de l'appel, avec la note maximale atteignable lue en base.
- **FR-063**: Une note absente NE DOIT PAS être traitée comme un zéro.
- **FR-064**: Une revue enregistrée sans être déposée NE DOIT compter dans aucun agrégat et NE DOIT être visible d'aucun pair.
- **FR-065**: Le dépôt d'une revue DOIT déclencher la consolidation des notes du dossier **dans la même transaction**, la base ne l'appelant nulle part.
- **FR-066**: La réponse d'une notation DOIT rendre les agrégats recalculés du dossier.
- **FR-067**: Noter DOIT exiger le droit de noter **et** une affectation non déportée ; lire un dossier non confié reste permis.
- **FR-068**: Un déport DOIT exiger un motif.
- **FR-069**: Les échanges rendus DOIVENT être filtrés à la source selon les trois visibilités du modèle : le comité pour les uns, le déposant pour les autres, l'auteur seul pour les notes personnelles.
- **FR-070**: Une demande de correction DOIT être forcée en visibilité partagée avec le déposant.
- **FR-071**: L'ouverture d'un dossier DOIT poser l'accusé de lecture, et la réponse DOIT dire l'état **d'avant** la visite.
- **FR-072**: Le nombre de membres du comité ayant ouvert le dossier DOIT être rendu.
- **FR-073**: La décision DOIT rendre trois issues distinctes — appliquée avec la ligne de journal, transition impossible, motif exigé — et non une exception.
- **FR-074**: Les droits de la personne sur cet écran DOIVENT être résolus une fois à la source, sans dispenser l'API de refaire le contrôle à l'écriture.
- **FR-075**: Une pièce dont l'objet n'est pas servi DOIT être rendue **sans adresse** de téléchargement.

### L'espace organisation (US5)

- **FR-076**: L'espace DOIT être servi par une composition propre au soumissionnaire, et NON par la vue destinée au comité.
- **FR-077**: Cette composition NE DOIT contenir ni note, ni note pondérée, ni rang, ni nom de membre du comité, ni liste nominative d'inscrits.
- **FR-078**: Elle DOIT porter le dossier, son édition, son journal de transitions, le nombre de ses demandes de correction **ouvertes** et ses séances programmées.
- **FR-079**: Le fil rendu au déposant DOIT être filtré sur la visibilité partagée **à la source**.
- **FR-080**: Une réponse du déposant DOIT toujours être partagée et NE DOIT jamais être une demande de correction.
- **FR-081**: La résolution d'une demande de correction DOIT pouvoir être posée **et retirée** par le déposant, et cette faculté DOIT être portée par une permission, non par un formulaire.
- **FR-082**: Le comité DOIT conserver la faculté de poser et de retirer une résolution de son côté.
- **FR-083**: L'accès à l'espace DOIT exiger une adhésion active à l'organisation.
- **FR-084**: Les décomptes d'inscrits d'une séance rendus à l'organisation DOIVENT être des nombres, jamais une liste nominative.

### Correction et renvoi (US6)

- **FR-085**: Le dossier DOIT pouvoir être rendu **recomposé sous la forme que le formulaire attend**, et cette recomposition DOIT exister à un seul endroit.
- **FR-086**: Cette recomposition DOIT ramener les textes à leur français, rendre le créneau en heure murale du fuseau de l'édition, et signaler pour chaque intervenant s'il possède un compte.
- **FR-087**: Un dossier DOIT rester modifiable tant que son édition n'est pas terminée.
- **FR-088**: Un dossier rejeté, retiré ou annulé NE DOIT PAS être modifiable.
- **FR-089**: Le renvoi d'un dossier corrigé DOIT être une **route distincte** du dépôt.
- **FR-090**: Le renvoi NE DOIT PAS être soumis à la fenêtre de l'appel ; le plafond par organisation DOIT l'être dans les deux cas.
- **FR-091**: Une modification de dossier NE DOIT propager AUCUN champ vers une séance programmée.
- **FR-092**: Une modification NE DOIT PAS provoquer de transition d'état.

### Les pièces (US7)

- **FR-093**: Une pièce DOIT être rattachée à partir d'un objet **déjà stocké**, jamais téléversée par ce module.
- **FR-094**: Le détachement d'une pièce NE DOIT PAS détruire l'objet stocké.
- **FR-095**: Le caractère public d'une pièce DOIT être porté par le lien, et une pièce interne NE DOIT PAS sortir sur une route publique.
- **FR-096**: L'adresse de téléchargement DOIT être composée en base et non fabriquée par le service.

### Historique, lectures et reprise (US8)

- **FR-097**: L'historique champ par champ DOIT être servi par la fonction du modèle, les champs recalculés en étant écartés.
- **FR-098**: Le journal des transitions DOIT être exposé tel quel, avec auteur, motif et instant.
- **FR-099**: Une opération DOIT permettre de semer les transitions **déductibles** d'un dossier qui n'en a aucune — création, dépôt, décision —, à partir des dates que le dossier porte.
- **FR-100**: Cette opération DOIT être rejouable sans créer de doublon et NE DOIT rien semer sur un dossier possédant déjà son journal.
- **FR-101**: L'effacement d'un dossier DOIT être **logique**, avec son auteur et son motif.
- **FR-102**: L'effacement DOIT purger les liens de thématiques du dossier, aucune contrainte référentielle ne le faisant.
- **FR-103**: Les compteurs de lecture DOIVENT rester collectifs ; « ce dossier, moi, l'ai-je ouvert ? » DOIT rester une question posée avec le lecteur en paramètre.

### Key Entities

- **Proposition** : le dossier déposé par une organisation — numéro lisible, textes multilingues, format, créneau souhaité, état, agrégats de notes. Rattachée à une édition, éventuellement à un appel, à une organisation porteuse et à un déposant.
- **Règle de transition** : un chemin déclaré entre deux états, avec la permission qu'il exige, le droit du porteur et l'obligation de motif. C'est une **donnée**, pas du code.
- **Transition** : une ligne de journal — d'où, vers où, par qui, pourquoi, quand.
- **Organisation associée** : le lien entre un dossier et une organisation, avec son rôle — porteur, co-organisateur, partenaire, soutien — et sa confirmation.
- **Intervenant** : le lien entre un dossier et une personne, avec son rôle, sa fonction et son organisation **au moment de l'activité**.
- **Pièce** : le lien entre un dossier et un objet stocké, avec son titre, son type et son caractère public.
- **Affectation de revue** : qui doit évaluer quoi, pour quand, et le déport le cas échéant.
- **Revue** : l'avis complet d'un membre du comité — recommandation, points forts et faibles, note personnelle, notes par critère —, brouillon tant qu'elle n'est pas déposée.
- **Note par critère** : la justification chiffrée d'une décision contestée, bornée par le maximum du critère.
- **Échange** : un message sur le dossier, avec sa visibilité explicite, éventuellement une demande de correction et sa résolution.
- **Accusé de lecture** : qui a ouvert le dossier, combien de fois, la première et la dernière.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Une organisation peut déposer un dossier complet de bout en bout, du premier enregistrement à la confirmation, sans qu'aucun écran n'ait à composer une jointure.
- **SC-002**: Le numéro de dossier annoncé au premier enregistrement est **identique** à celui de la confirmation de dépôt.
- **SC-003**: Deux dossiers de même titre dans une même édition sont acceptés, sans que la seconde organisation ait à renommer son activité.
- **SC-004**: Aucun dossier ne peut naître dans un autre état que « brouillon », y compris par un appel forgé.
- **SC-005**: Les quatorze chemins déclarés en base sont offerts ou refusés **sans qu'aucune liste de transitions ne soit écrite dans le code** : ajouter une ligne en base ajoute une action.
- **SC-006**: Trois personnes de droits différents interrogeant les transitions d'un même dossier reçoivent trois réponses différentes, chacune conforme à ce que la base déclare.
- **SC-007**: Une transition non déclarée est refusée par un message français nommant les deux états, et sous un code distinct de celui d'un motif manquant.
- **SC-008**: Aucun événement de domaine de changement d'état n'est émis en double.
- **SC-009**: Un compte sans aucun droit d'administration reçoit un refus explicite sur la liste, et jamais une liste vide.
- **SC-010**: Six identifiants forgés hors périmètre mènent au même refus qu'un identifiant inexistant.
- **SC-011**: Les décomptes des facettes correspondent exactement, filtre par filtre, aux lignes rendues.
- **SC-012**: Une action groupée sur une sélection hétérogène rend un compte d'appliqués et un compte d'écartés dont la somme égale la taille de la sélection, chaque écart portant sa raison.
- **SC-013**: Une réponse de fiche d'évaluation servie sous voile ne contient, à l'inspection complète de sa charge utile, **aucune** note, recommandation ou nom de pair.
- **SC-014**: Le voile se lève à la lecture qui suit le dépôt de la revue, sans qu'aucun réglage d'affichage n'intervienne.
- **SC-015**: Après le dépôt d'une revue, les agrégats rendus dans la réponse sont **égaux** à ceux relus en base immédiatement après.
- **SC-016**: Une note supérieure au maximum de son critère est refusée en nommant le critère et sa borne.
- **SC-017**: Une personne détenant le droit de noter mais non affectée est refusée à l'enregistrement d'une revue, tout en pouvant lire le dossier.
- **SC-018**: Une réponse d'espace organisation ne contient, à l'inspection complète de sa charge utile, **aucune** note, aucun rang, aucun nom de membre du comité, aucun nom d'inscrit.
- **SC-019**: Un message du comité non partagé n'apparaît dans aucune réponse servie au déposant.
- **SC-020**: Le déposant peut poser puis retirer la résolution d'une demande de correction, et le compteur de demandes ouvertes suit les deux gestes.
- **SC-021**: Un dossier renvoyé après correction est accepté alors que l'appel est **clos** depuis plusieurs semaines.
- **SC-022**: Le plafond par organisation refuse aussi bien un dépôt qu'un renvoi excédentaire.
- **SC-023**: Un créneau enregistré à 14:30 dans le fuseau de l'édition est rouvert à **14:30** depuis n'importe quel fuseau.
- **SC-024**: La correction d'un dossier retenu laisse la séance qui en est née strictement inchangée — créneau, salle, inscrits, rappels.
- **SC-025**: Un dossier sans journal reçoit ses trois transitions déductibles, et une seconde exécution n'en ajoute aucune.
- **SC-026**: L'effacement d'un dossier ne laisse aucun lien de thématique derrière lui.
- **SC-027**: Le contrôle des frontières de modules reste sans écart, et le graphe de dépendances du crate ne porte **aucune arête** vers un autre crate de module.

## Assumptions

- **Le contact du dossier vaut le déposant** tant que le formulaire ne le demande pas (écart n° 30). La règle est explicite dans le service ; elle sera remplacée le jour où l'étape des organisations posera la question.
- **Les longueurs maximales des textes sont celles que l'écran a retenues** (écart n° 28) : elles deviennent des règles d'API, tenues à un seul endroit, et non des règles de base.
- **La liste blanche d'assainissement est celle de la barre d'outils de l'éditeur** (écart n° 32) : gras, italique, sous-titres, listes, citation, lien, séparateur.
- **Noter exige une affectation.** Le contrat du front distingue explicitement la permission de l'affectation, et dit qu'un membre du comité peut lire un dossier qu'on ne lui a pas confié « sans le noter ».
- **La résolution posée par le déposant vaut déclaration**, non clôture, tant que le commanditaire n'a pas tranché (écart n° 35). Le comité conserve la faculté de la retirer.
- **La déduction des transitions d'un dossier repris de la v1 est une opération de ce module**, et non une ligne du fichier de reprise : le corps de la fonction de migration n'existe pas dans le dépôt (écart n° 100 ci-dessous).
- **Le plafond par organisation compte les dossiers portés**, non les co-organisations. C'est ce que fait le contrôle en base, et ce n'est pas « amélioré ».
- **Le téléversement de fichier appartient à B6** : ce module rattache un objet déjà stocké et ne compose aucune adresse lui-même.
- **Les séances et leurs décomptes appartiennent à B5** : ce module les nomme dans la composition de l'espace organisation, et le contrat qu'il expose est celui que le front déclare déjà.
- **Le module ne modifie aucun fichier de `docs/database/`.** Les huit écarts relevés ci-dessous sont traités dans le service ou consignés.

## Dépendances et frontières

| Ce que ce module lit hors de son schéma | Pourquoi c'est admis |
|---|---|
| `event.calls_for_proposals`, `event.review_criteria`, `event.call_reviewers`, `event.events` | La question porte sur **ses** entités : la recevabilité d'un dossier, la grille qui le note, le comité qui l'évalue, le fuseau qui date son créneau |
| `org.organizations`, `org.memberships` | Qui porte le dossier, et qui a le droit d'écrire en son nom |
| `identity.people`, `identity.has_permission()`, `identity.administered_events()` | Les intervenants, l'autorisation et le périmètre |
| `reference.taxonomy_terms`, `reference.terms_of()`, `reference.term_badges()` | Les thématiques du dossier |
| `media.assets`, `media.object_url()` | Les pièces du dossier |
| `analytics.mv_organization_scorecard` | L'historique de participation affiché sur la fiche d'évaluation |

| Ce que ce module écrit hors de son schéma | Statut |
|---|---|
| `reference.entity_terms` | **Dérogation assumée et bornée**, identique à celle de B3 : la table est polymorphe et sans clé étrangère, aucun autre module ne peut poser les thématiques d'une proposition. À isoler dans un seul fichier |
| `identity.people` (création d'un intervenant inconnu) | **À trancher au plan.** Le modèle l'exige — la clé de l'intervenant est NOT NULL — et le formulaire le suppose. Deux voies : la dérogation bornée, ou un contrat d'événement consommé par le module Identité. La seconde interdit la création synchrone dont le formulaire a besoin |

---

## Écarts relevés en écrivant la spécification de B4 (20/08)

Numérotation à la suite de B3, qui s'arrêtait à 92. **Aucune modification du modèle n'est proposée** : les huit points ci-dessous se traitent dans le service ou se consignent.

| N° | Écart | Où | Ce qu'il coûte | Suite donnée |
|---|---|---|---|---|
| **93** | **LE DÉCLENCHEUR D'ÉTAT ÉMET DÉJÀ L'ÉVÉNEMENT DE DOMAINE.** `tg_guard_proposal_status()` appelle `platform.emit_event()` à chaque transition acceptée, avec le numéro de dossier, l'édition, l'organisation et les deux états. C'est **l'inverse de B3**, où aucun déclencheur n'émettait rien — et le retour du piège n° 1 de B1 et B2 | `070` § 3 | Un service qui émettrait à son tour produirait **deux** événements par transition. Les consommateurs — notifications, courriels, tableau de bord — enverraient tout en double, et le doublon ne se verrait qu'en production | **Inscrit à la spécification (FR-043)** : le service n'émet rien sur les changements d'état. Vérifié en lisant le corps du déclencheur, pas supposé |
| **94** | **`platform.purge_term_links()` N'EXISTE PAS.** Le commentaire d'en-tête de `reference.entity_terms` annonce que « le nettoyage est assuré par `platform.purge_term_links()` » ; **aucune fonction de ce nom n'est définie dans les dix-neuf fichiers** | `020` § 4 | Le commentaire fait croire que le ménage est fait. Il ne l'est nulle part : la table est polymorphe, aucune clé étrangère ne pointe vers les propositions, et un dossier effacé laisserait ses liens derrière lui | **Le service purge lui-même** (FR-102), ce que l'écart n° 3 demandait déjà. Le commentaire du modèle reste inexact : consigné, non corrigé — le prompt interdit de modifier le SQL sans justification, et l'inexactitude ne coûte rien tant qu'elle est connue |
| **95** | **L'ADRESSE D'URL D'UN DOSSIER EST OBLIGATOIRE, UNIQUE PAR ÉDITION, ET LE FORMULAIRE NE LA PORTE PAS.** `proposals.slug` est NOT NULL sous `ux_proposals_slug (event_id, slug)` ; `ProposalDraft` n'a aucun champ correspondant. Pire : le premier enregistrement automatique a lieu **à la première frappe**, quand le titre est encore vide, et `platform.slugify('')` rend NULL | `070` § 2, `000` § 5.2 | À la lettre du contrat, le tout premier enregistrement d'un brouillon **échoue** sur une violation de non-nullité, et deux dossiers homonymes dans une édition échouent sur une violation d'unicité. Les deux cas sont ordinaires, pas exotiques | **Traité dans le service** (FR-013) : dérivation depuis le titre, repli quand il est vide, unicité rendue par suffixe au sein de l'édition. À ne pas demander au client : il n'a pas de quoi la calculer |
| **96** | **UN DOSSIER PEUT NAÎTRE DANS N'IMPORTE QUEL ÉTAT.** Le garde de la machine à états est posé `BEFORE UPDATE OF status` ; à l'insertion, le second déclencheur ne fait que **journaliser** l'état de départ. Le contrôle de recevabilité, lui, ne se déclenche que si l'état inséré vaut « déposé » | `070` § 3 | Un `INSERT` direct en « retenu » passe tous les contrôles : ni transition vérifiée, ni fenêtre d'appel, ni plafond. La machine à états ne protège que ce qui **change** d'état, pas ce qui naît déjà arrivé | **Traité dans le service** (FR-012) : la création pose toujours « brouillon », quel que soit l'état demandé. Consigné pour la reprise v1, qui insérera des dossiers dans leur état final et **court-circuitera donc la machine** — c'est voulu là, et c'est précisément pourquoi l'écart n° 37 existe |
| **97** | **LE MOTIF D'UNE TRANSITION S'ÉCRIT DANS LA COLONNE DE LA DÉCISION DU COMITÉ.** Le garde exige `decision_reason` pour les six transitions à motif — dont **le retrait par l'organisation** — et l'écrase à chaque fois | `070` § 2 et § 3 | Un dossier retiré puis remis en course porte, dans sa colonne de décision, le motif du retrait écrit par l'organisation. Un écran qui afficherait « motif de la décision » depuis la ligne du dossier montrerait le mauvais texte, sans erreur | **Traité dans le service et au contrat** (FR-042) : le motif se lit dans le **journal**, qui garde chacun avec son auteur et son instant. La colonne du dossier n'est écrite que parce que le déclencheur l'exige |
| **98** | **RIEN N'APPELLE `refresh_proposal_score()`.** La fonction existe, son commentaire dit « à appeler après toute saisie de note », et **aucun déclencheur ne la déclenche** — ni sur les revues, ni sur les notes par critère | `070` § 5 | Sans appel explicite, la note du dossier, sa moyenne, son nombre de revues et son élimination restent aux valeurs de la ligne. La liste du back-office trie sur une note qui ne bouge jamais, et le classement est faux sans que rien ne le signale | **Inscrit à la spécification (FR-065)** : le service l'appelle dans la **même transaction** que le dépôt d'une revue, et rend les agrégats relus. Le vérifier plutôt que le supposer est le sujet du critère SC-015 |
| **99** | **UNE DEMANDE DE CORRECTION PEUT ÊTRE INVISIBLE DE CELUI QU'ELLE VISE, ET COMPTER QUAND MÊME.** `is_change_request` est indépendant de `visibility` ; la vue de pilotage et l'index des demandes ouvertes comptent **toutes** les demandes non résolues, quelle que soit leur visibilité | `070` § 6 et § 7 | Une demande écrite en visibilité « comité » bloque le dossier, s'affiche « 1 point à corriger » chez le déposant — qui ne peut pas lire le message. C'est exactement le cas que l'écart n° 38 a déjà produit sous une autre forme | **Traité dans le service** (FR-070) : une demande de correction est **forcée** en visibilité partagée, comme le contrat du front l'exige déjà. Le compteur redevient honnête par construction |
| **100** | **LA REPRISE DES ACTIVITÉS DE LA V1 EST UNE COQUILLE VIDE.** `legacy.migrate_activities()` ne fait que journaliser la correspondance des statuts et renvoyer deux zéros ; son corps effectif est annoncé dans `scripts/migration/03_activities.sql`, **qui n'existe pas dans le dépôt** — le dossier `scripts/` non plus | `910` § 6.3 | L'écart n° 37 demande que la reprise sème les transitions déductibles. Écrite dans un fichier qui n'existe pas, l'exigence ne serait exécutée par personne, et chaque écran referait la déduction à sa façon — ce qu'`utils/proposal-timeline.ts` fait déjà côté front | **La déduction devient une opération de CE module** (FR-099, FR-100), rejouable et sans doublon, plutôt qu'une ligne d'un script absent. Elle reste utilisable par la reprise le jour où celle-ci s'écrira |

**Deux écarts antérieurs sont confirmés et traités ici** : le n° 27 (les bornes d'intervenants de l'appel ne sont vérifiées par aucun déclencheur — FR-028) et le n° 30 (rien ne remplit le contact du dossier — FR-032, règle par défaut explicite, en attendant l'arbitrage inscrit aux points bloqués).

**Un écart antérieur reste ouvert et n'est pas de ce module** : le n° 29 (le français est obligatoire, et une organisation anglophone le découvre à l'étape 2). Il appelle un arbitrage du commanditaire, pas une ligne de code.

---

## Ce que la spécification laisse au plan

- **La création d'une personne inconnue** depuis le formulaire : dérogation bornée au schéma `identity`, ou contrat d'événement. La seconde voie interdit la création synchrone dont le formulaire a besoin ; il faudra le dire et non le subir.
- **Le découpage des fichiers** du crate sous le garde-fou de mille lignes : la fiche d'évaluation compose onze tables, c'est le plus gros assemblage du jalon.
- **La forme exacte de l'opération de déduction** des transitions v1 : route d'administration, travail différé, ou les deux.
- **La liste des travaux différés** — avis de dépôt, relance des co-organisations non confirmées, rappel des revues en retard — dont aucun n'est exigé par le prompt et dont certains appartiennent peut-être à B6.
