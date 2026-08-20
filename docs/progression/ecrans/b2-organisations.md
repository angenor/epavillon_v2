# B2 — Organisations

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de ce module est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) § PHASE B — B2.

**État** : ✅ **livrée** le 20/08 — `backend/crates/modules/org/`. Les 129 tâches de `specs/002-organisations/tasks.md` sont faites, à l'exception des vérifications manuelles nommées plus bas.

**Ce qui tourne** : 21 routes, 6 travaux différés, 6 événements, 11 codes d'erreur, **30 fichiers de test** et **246 tests verts** sur le workspace entier — contre 141 à la fin de B1. `cargo clippy --workspace --all-targets --all-features -- -D warnings` passe ; aucun fichier de `backend/` ne dépasse 1000 lignes ; `cargo tree -p org` ne montre **aucune arête** vers un autre crate de module.

**La cible chiffrée est tenue** : la recherche répond en **55 ms au 95ᵉ centile** sur 5 001 fiches, pour une cible de 150 ms (SC-002). Médiane 48 ms, pire cas 64 ms. Aucun des trois remèdes prévus par R2 n'a été nécessaire — et **aucune modification du SQL n'a été faite**, comme annoncé.

---

## Ce qui a été livré

`/speckit-specify` a produit **`specs/002-organisations/`** :

| Fichier | Contenu |
|---------|---------|
| `spec.md` | 8 histoires utilisateur priorisées, 14 cas limites, **84 exigences fonctionnelles** en 10 groupes, 12 entités du modèle, 18 critères de réussite mesurables, 13 hypothèses, et les 13 vérifications faites en chemin |
| `checklists/requirements.md` | Contrôle qualité — tous les points passent, avec deux réserves assumées et motivées |

`.specify/feature.json` pointe désormais sur `specs/002-organisations`.

**Aucune modification du SQL n'est proposée.** `040_organizations.sql` a été relu intégralement : les quatre verrous sont complets, la fonction de recherche a été corrigée le 17/08 (écarts n° 21 et 22) et le registre des références le 18/08 (dédoublonnage des domaines).

### Les cinq exigences du prompt, et où elles atterrissent

| Exigence | Traitement |
|---|---|
| **Écart n° 23** — la recherche fait remonter le domaine quel que soit le nom | **FR-005 à FR-008.** Deux lectures distinctes au-dessus de **la même** fonction de base, qui n'est pas modifiée : celle destinée à une personne n'admet que les fiches portant une ressemblance de dénomination ; celle destinée à la revue des doublons rend tous les signaux. Le domaine continue d'**alimenter le score** de la première (FR-007) — il hisse la bonne fiche quand le nom correspond aussi, ce qui est son seul emploi juste ici. SC-003 et SC-004 exigent que les deux lectures rendent des résultats **différents** sur la même requête |
| **Écart n° 24** — quel rôle prend qui rejoint | **FR-032 et FR-042.** La règle de l'écran est confirmée : qui crée devient référent, qui rejoint reste membre. La question qui vient avec — *que devient une fiche dont l'unique référent part ?* — est tranchée par **FR-041** : le retrait du dernier référent actif est refusé tant qu'aucun autre n'a été désigné ; un administrateur peut passer outre, et la fiche est alors signalée **sans référent** au back-office, où ses demandes restent approuvables. C'est l'option qui ne perd rien, et elle est posée en question au commanditaire |
| **Adhésion automatique contre adhésion en attente** | **FR-020 et FR-021.** La condition est **lue** dans la base — domaine vérifié ET rattachement automatique —, jamais réinventée. FR-025 interdit au service de recalculer le rattachement principal, que la base attribue par trigger |
| **Fiche créée depuis un formulaire public = `candidate`** | **FR-027.** Jamais `active`. C'est ce qui alimente la file de dédoublonnage |
| **`acknowledged_match_ids`** | **FR-029.** Les fiches montrées avant que la personne ne maintienne sa création sont conservées pour la revue. FR-031 rappelle qu'une ressemblance ne bloque jamais |
| **Écart n° 33** — les deux files d'adhésion | **FR-034 à FR-039.** Deux files, deux autorisations : approuver une **demande** est un geste de référent et ne peut jamais porter sur une **invitation** ; accepter une invitation passe par le lien à usage unique. Un refus **révoque** au lieu de supprimer. L'invitation par adresse crée la personne sans compte et **sans nom déduit de l'adresse** ; une seconde invitation reçoit « déjà invitée, relançable » |

### Les quatre obligations relevées en écrivant A11

| Obligation | Traitement |
|---|---|
| **(1) Les choix de champ sont un `UPDATE` de la CIBLE, dans la même transaction** | **FR-073 — et l'ORDRE inscrit en A11 est corrigé.** L'obligation disait « **avant** l'appel » ; c'est impossible pour le nom légal. Voir l'écart n° 70 ci-dessous. L'exigence de la **même transaction**, elle, est conservée intacte : c'est ce qui la motivait |
| **(2) Lecture ouverte par la permission sur n'importe quelle portée, fusion en portée globale** | **FR-043, FR-044, FR-066.** Confirmé — avec une correction : la permission de consultation **ne suffit pas** (écart n° 73). La liste exige la permission **et** un périmètre d'administration non vide, et les trois cas du périmètre restent distincts. La fusion exige la portée globale |
| **(3) La fiche de performance est matérialisée** | **FR-048 et FR-065.** Les deux branches du choix sont prises : la liste relit **statut, sceau, score de confiance et pointeur de fusion** sur la table vivante, et un travail différé rafraîchit la projection après les écritures qui la périment. Aucun champ nouveau dans la réponse, donc aucune renégociation du contrat |
| **(4) Le score de confiance n'est appelé par aucun trigger** | **FR-063 et FR-064 — tranché : travail différé, pas trigger.** Le score est une aide au tri, pas un invariant, et un trigger sur les quatre tables qui l'alimentent renchérirait des chemins d'écriture fréquents (chaque adhésion, chaque domaine). Le travail porte une **clé d'unicité par organisation** pour que cent approbations coup sur coup ne produisent pas cent recalculs |

---

## Puis `/speckit-plan`, le même jour

| Fichier | Contenu |
|---------|---------|
| `plan.md` | contexte technique, **contrôle des dix principes avant et après conception**, arborescence du crate `org`, ce que le plan ne tranche pas |
| `research.md` | **19 décisions techniques**, chacune avec ses alternatives écartées. Les décisions de B1 qui tiennent encore ne sont pas rejouées |
| `data-model.md` | ce que le code lit, écrit et laisse à la base, table par table ; les trois machines à états ; la correspondance Rust ↔ base ; d'où vient chaque forme attendue par le front |
| `contracts/routes.md` | **21 routes**, leur autorisation, et **les deux lectures de recherche déclarées côte à côte**, comme le prompt l'exige |
| `contracts/errors.md` | **11 codes stables** ajoutés, les six refus qui sortent en 200, et la traduction des contraintes PostgreSQL |
| `contracts/events.md` | 6 événements émis, **celui qu'on n'émet pas**, 6 travaux différés, et l'ordre des écritures d'une fusion |
| `quickstart.md` | comment lancer, éprouver à la main, et les portes à passer |

**Contrôle constitutionnel : une entorse, justifiée ; aucune autre.** Le décompte de transfert d'une fusion **compose son SQL** depuis le registre des références, donc hors vérification à la compilation (principe VI). C'est le prix pour que le chiffre annoncé avant la fusion et le chiffre rendu après viennent du **même raisonnement** — énumérer les tables en Rust rendrait le décompte faux au premier module qui se déclare au registre, **et personne ne le verrait**. Un seul fichier, identifiants cités, risque d'injection nul.

### Les trois décisions qui structurent le reste

1. **Les deux lectures : le filtre est en SQL, et il faut SUR-LIRE.** La limite est appliquée **à l'intérieur** de la fonction : filtrer après coup rendrait moins de lignes que demandé — dix demandées, trois écartées, sept rendues. La lecture filtrée passe donc `limite + 5` à la fonction, filtre, puis tronque.
2. **Le service de jetons à usage unique remonte dans le noyau.** Trois des cinq finalités du modèle n'appartiennent pas à `identity` — l'invitation est le geste de ce module, la confirmation d'un intervenant sera celui de B4 —, et aucun crate de module ne peut dépendre d'un autre. Les durées par finalité vivaient **déjà** dans la configuration du noyau depuis B1.
3. **La cible de 150 ms se mesure avant de se traiter**, et le semis doit ressembler à un vrai référentiel : 5 000 noms tirés au hasard n'ont presque aucun trigramme commun, le parcours d'index rendrait une poignée de lignes, et la mesure serait excellente et fausse.

---

## Puis `/speckit-tasks`, le même jour

**`tasks.md` porte 129 tâches en 11 phases, dont 29 tests** — une phase par histoire utilisateur, chacune éprouvable seule, avec son point de contrôle. Les tests ne sont pas optionnels : le principe X impose des tests d'intégration sur base réelle, sans mock.

| Phase | Histoire | Tâches | Dont tests |
|---|---|---|---|
| 1 | Mise en place | T001–T007 | — |
| 2 | Fondations — **jetons dans le noyau, inscription corrigée** | T008–T023 | 2 |
| 3 | US1 — recherche (**MVP**) | T024–T035 | 4 |
| 4 | US2 — rattachement | T036–T045 | 3 |
| 5 | US3 — création | T046–T054 | 3 |
| 6 | US4 — adhésions, les deux files | T055–T068 | 3 |
| 7 | US5 — back-office, lecture bornée | T069–T079 | 4 |
| 8 | US6 — écritures de la fiche | T080–T089 | 2 |
| 9 | US7 — détection continue et score | T090–T099 | 2 |
| 10 | US8 — fusion | T100–T117 | 6 |
| 11 | Finition | T118–T129 | — |

**Quatre jalons de livraison** : **T001–T035**, la recherche répond juste et le défaut n° 1 de la v1 est *prévenu* — démontrable en une minute sur la fiche IFDD que le semis fournit déjà ; **T001–T054**, l'écran de rattachement (A2) entièrement servi ; **T001–T099**, le back-office tient le référentiel au quotidien ; **T001–T129**, la fusion, et donc la réparation de ce que la v1 a laissé.

**Deux avertissements sont écrits en tête du fichier**, parce qu'ils se paieraient à l'exécution : **la phase 2 touche du code livré et éprouvé** — le service de jetons remonte dans le noyau et l'inscription est corrigée —, et **l'ordre des deux écritures d'une fusion est l'inverse de celui qu'`api.md` et l'en-tête d'`admin-organizations.ts` annoncent encore**. Les deux documents fautifs sont corrigés par T126.

Les quatre obligations minimales du principe X ont chacune leur tâche nommée : **T030/T075** (chemin nominal), **T076** (URL forgée), **T089 et T117** (invariants traduits, dont un message repris mot pour mot), **T116** (écriture dans l'outbox — **et il compte les événements, il ne vérifie pas leur présence**).

---

## Écarts relevés en écrivant la spécification de B2 (20/08)

Numérotation à la suite de B1, qui s'arrêtait à 69.

| N° | Écart | Où | Ce qu'il coûte | Suite donnée |
|---|---|---|---|---|
| **70** | **L'ORDRE DES DEUX ÉCRITURES DE LA FUSION EST L'INVERSE DE CELUI INSCRIT EN A11.** `ux_organizations_name_country` ne porte que sur les fiches **vivantes** : tant que la source n'est pas passée `merged`, la cible ne peut pas reprendre son nom légal. Or `merge_organizations()` ne pose ce statut qu'**à la fin** | `040` § 1 et § 6 | Une fusion avec arbitrage du nom légal échouerait **systématiquement** sur une violation d'unicité, et le message parlerait d'un index — incompréhensible pour un opérateur qui vient de choisir un nom. Le défaut ne se voit qu'à l'exécution, sur le seul champ que l'on arbitre le plus souvent | **FR-073** : les arbitrages viennent **APRÈS** l'appel de fusion, dans la même transaction. `docs/progression/api.md` et l'en-tête de `types/admin-organizations.ts` portent l'ancien ordre : à corriger au raccordement |
| **71** | **L'ADRESSE D'URL NE PEUT PAS ÊTRE ARBITRÉE dans une fusion.** `ux_organizations_slug` est unique **sans condition de statut** : la fiche absorbée garde la sienne, et la survivante ne peut pas la reprendre. La libérer casserait précisément ce que la fusion promet — « les anciennes URL continuent de résoudre » | `040` § 1 | `MergeField` du front range `slug` parmi les dix champs comparés, et deux fiches ont **toujours** des adresses différentes : l'écran demandera donc un arbitrage impossible à honorer sur un côté | **FR-074** : le champ reste au comparatif, l'arbitrage vers la source est refusé par un code stable **nommant le champ**. À refermer côté écran en **B7**, ou par une table d'alias d'adresses si le besoin est confirmé |
| **72** | **L'UNICITÉ DES ADHÉSIONS PORTE AUSSI SUR LES RÉVOCATIONS.** `ux_memberships` est sur (organisation, personne), sans condition de statut. Les données simulées écartent les révoquées avant de créer une ligne — contre la vraie base, la seconde demande violerait la contrainte | `040` § 4 | Une personne dont l'adhésion a été refusée ne pourrait **plus jamais** redemander à rejoindre, et l'erreur remonterait en violation de contrainte | **FR-023** : une nouvelle demande **reprend la ligne existante**. Vérifié par SC-008 |
| **73** | **LA PERMISSION DE CONSULTATION DES ORGANISATIONS EST DÉTENUE PAR LE RÔLE D'UTILISATEUR ORDINAIRE** (`('standard', 'org.organization.read')`). Elle ne peut donc pas garder le back-office à elle seule | `030` § 6 | Le jour où l'inscription attribuera ce rôle, la liste des organisations du back-office s'ouvrirait à **tout compte connecté**. L'écran d'A11 la teste seule (`hasPermissionOnAnyScope`) | **FR-043 et FR-044** : la liste exige la permission **et** un périmètre d'administration non vide. Le rôle ordinaire n'accorde pas la permission sur laquelle le périmètre est calculé — la conjonction suffit, sans modifier le modèle |
| **74** | **RIEN N'ATTRIBUE AUJOURD'HUI LE RÔLE D'UTILISATEUR ORDINAIRE.** Vérifié dans le service d'inscription livré en B1 et dans `900_seed.sql` : seul un super-administrateur global est semé | `030` § 6, `900` § 5 | Garder la recherche d'organisation derrière la permission de consultation refuserait **tout nouvel inscrit**, c'est-à-dire exactement la personne que l'écran de rattachement attend | **FR-014** : la recherche n'exige qu'une **session**. Question ouverte : l'inscription doit-elle attribuer ce rôle ? Elle se posera en **B7** |
| **75** | **LE FRONT ENVOIE L'ADRESSE DONT IL VEUT CONNAÎTRE LE DOMAINE**, en paramètre de requête | `useApi.ts` | Un client pourrait interroger le domaine de n'importe qui et apprendre quelle organisation le détient. Aucun écran n'en a besoin : le seul appel passe l'adresse de la personne connectée | **FR-017** : le domaine est dérivé de la **session**, le paramètre est ignoré. Même motif que « les droits déclarés par le client sont ignorés », éprouvé en B1 |
| **76** | **LA FONCTION DE FUSION ÉMET ELLE-MÊME SON ÉVÉNEMENT et marque elle-même la paire de la file.** C'est le piège n° 1 du module `identity`, à l'identique | `040` § 6 | Un service qui émettrait « fusionnée » après l'appel en écrirait **deux**, sans qu'aucune erreur ne le signale, et un consommateur idempotent traiterait la première puis ignorerait la mauvaise | **FR-076 et FR-078** : le service n'émet rien et ne marque rien. À inscrire dans `contracts/events.md` au plan, comme B1 l'a fait pour l'anonymisation |
| **77** | **LE MOTIF « RESSEMBLANCE DE NOM » N'EST POSÉ QU'AU-DESSUS DE 0,3**, alors que l'opérateur trigramme fait entrer une ligne **à partir de** 0,3 | `040` § 5 | Une fiche entrée à 0,300 exactement remonterait **sans** porter le motif, et serait écartée par la lecture filtrée. Frontière étroite, mais réelle | **Assumé et écrit** : la lecture filtrée s'aligne sur le **motif** et non sur le score, pour que l'API et l'écran écartent exactement les mêmes lignes. Aucune exception à coder |
| **78** | **LE REGISTRE DES RÉFÉRENCES EST ALIMENTÉ PAR HUIT FICHIERS, pas par un seul.** `040` § 6 n'en sème que quatre lignes ; `050`, `060`, `070`, `075`, `080`, `090` et `125` en ajoutent quatorze | `040` § 6 + 7 fichiers | Lire le seul fichier du module ferait croire que la fusion ne déplace que des dénominations, des domaines, des adhésions et des rattachements principaux — et le décompte de l'écran de fusion paraîtrait faux | **Constat, pas défaut.** Le registre est complet en base et les données simulées le recopient à l'identique. Noté pour que la prochaine session ne cherche pas les quatorze lignes manquantes |
| **79** | **QUATRE LECTURES DÉCLARÉES DANS LA COUCHE D'ACCÈS DU FRONT NE SONT CONSOMMÉES PAR AUCUN ÉCRAN** — dénominations, domaines, membres d'une organisation, et file des doublons « ouverte ». Leur contenu est servi par la fiche complète du back-office et par l'espace organisation | `useApi.ts` | Les livrer coûterait quatre routes gardées, testées et documentées, pour zéro appel | **Non livrées**, et dit dans les hypothèses. `organizations.list()`, elle, n'est consommée que par la page de guide de style : livrée **bornée** (défaut 50, maximum 200) pour ne pas la casser |
| **80** | **UNE FUSION PEUT FAIRE PERDRE UN RÔLE DE RÉFÉRENT.** Une personne membre des deux fiches voit sa ligne **source supprimée** par le dédoublonnage : si c'est elle qui portait le rôle de référent, la cible garde le rôle le plus faible | `040` § 6 | Une organisation peut sortir d'une fusion **sans référent**, sans que rien ne le dise. Le cas est rare mais silencieux | **Consigné en cas limite.** L'aperçu de fusion ne le signale pas aujourd'hui ; à reprendre au plan — un avertissement de plus dans la liste que **FR-070** déjà prévoit |

---

## Écarts relevés en écrivant le plan de B2 (20/08)

| N° | Écart | Où | Ce qu'il coûte | Suite donnée |
|---|---|---|---|---|
| **81** | **UNE PERSONNE CRÉÉE PAR INVITATION NE POUVAIT PAS SE CRÉER DE COMPTE.** L'inscription livrée en B1 branche sur « adresse connue » et envoie un rappel « vous avez déjà un compte » — or l'invitation crée une personne **sans compte**, ce que la séparation personne / compte permet et que l'écart n° 33 demande | `identity/service/registration.rs` | L'invitation resterait une **demi-fonctionnalité** : le courriel part, le lien fonctionne, l'adhésion s'active, et l'invité ne peut jamais se connecter. Le défaut appartient à B1 et ne s'est vu qu'en écrivant B2 | **Corrigé dans B2** (R9) : une personne connue **sans compte** obtient un compte et un lien de vérification, la réponse restant invariable et le coût de hachage payé dans les deux cas. Test `invitee_peut_creer_son_compte` |
| **82** | **TROIS DES CINQ FINALITÉS DE JETON N'APPARTIENNENT PAS À `identity`** — l'invitation est le geste de ce module, la confirmation d'un intervenant sera celui de B4. Or le service de jetons vit dans `identity`, dont aucun crate de module ne peut dépendre | `identity/repo/tokens.rs` | Trois issues, deux mauvaises : recopier « consommer un jeton atomiquement » dans chaque module — la seule opération du lot où une divergence se paie en jeton rejouable —, ou contredire le modèle en inventant un lien signé hors table | **Le service remonte dans `kernel::tokens`** (R8), et `identity` est réécrit pour l'appeler. **La moitié du travail était déjà faite** : les cinq durées par finalité vivent dans la configuration du noyau depuis B1, où elles avaient déjà l'air d'être au bon endroit. Les tests de B1 ne sont pas réécrits — ils sont la preuve du comportement constant |
| **83** | **LA LIMITE DE LA RECHERCHE EST APPLIQUÉE À L'INTÉRIEUR DE LA FONCTION**, avant tout filtrage possible | `040` § 5 | Filtrer après coup rendrait **moins de lignes que demandé**, en silence : dix demandées, trois écartées par le filtre de l'écart n° 23, sept rendues. L'écran afficherait une liste courte sans que rien n'indique pourquoi | **La lecture filtrée sur-lit** — `limite + 5`, filtre, tronque (R1). La marge est petite parce que le nombre de fiches remontant par le seul domaine est le nombre de fiches déclarant le domaine de l'appelant : une, deux dans le cas des jumelles OSED |
| **84** | **L'EXPRESSION DE TABLE QUI PORTE LE TERME EST RÉFÉRENCÉE PLUSIEURS FOIS**, donc matérialisée par PostgreSQL : le terme n'est pas une constante au moment de la planification | `040` § 5 | L'usage de l'index trigramme dépend alors de la capacité du planificateur à paramétrer le parcours dans une boucle imbriquée. **C'est vérifiable et non devinable** — et c'est le seul point qui pourrait faire manquer la cible de 150 ms | **Nommé, et mesuré plutôt que supposé** (R2) : le test rend le plan d'exécution dans son message d'échec, et l'ordre des remèdes est fixé d'avance — **modifier le SQL vient en dernier**, avec la mesure comme justification |
| **85** | **FR-041 N'AVAIT AUCUN POINT D'APPLICATION.** Le retrait du dernier référent ne peut être refusé que s'il existe une route qui retire un membre — et aucun écran n'en appelle | `types/organization-workspace.ts` | La règle tranchée en spécification serait restée lettre morte : rien, dans le module, n'aurait pu la faire respecter | **Une route de révocation est livrée** (`DELETE /memberships/{id}`), bien qu'aucun écran ne l'appelle encore. La distinction avec l'écart n° 79 est nette : là c'étaient des **lectures** dont le contenu est servi ailleurs ; ici c'est une **écriture sans laquelle une règle spécifiée n'existerait pas** |
| **86** | **LE SEMIS DONNE PLUS QUE PRÉVU, et l'ignorer aurait fait perdre une demi-heure.** `900_seed.sql` § 5 ne sème pas qu'une organisation : l'IFDD y arrive avec **ses cinq dénominations** — sigle, faute d'orthographe, traduction, deux anciens noms — et **deux domaines vérifiés**, dont un en rattachement automatique | `900` § 5 | Deux des trois parcours de vérification s'éprouvent **sans rien semer** : les cinq façons de désigner une organisation, et le rattachement automatique — y compris son contre-exemple, le second domaine étant vérifié mais **non** marqué | **Constat, pas défaut.** Inscrit dans `quickstart.md` : seule la paire de jumelles OSED reste à semer à la main |

---

## Ce qui a été vérifié en écrivant la spécification, et comment

| Contrôle | Résultat |
|---|---|
| **`040_organizations.sql` a-t-il besoin d'être modifié ?** | **Non.** Relecture intégrale des 763 lignes. Les quatre verrous sont complets ; l'écart n° 23 se traite **au-dessus** de la fonction, comme le prompt l'exige |
| Qui alimente `org.organization_references` ? | **Huit fichiers SQL, 18 lignes.** `grep` sur `docs/database/*.sql` — voir l'écart n° 78 |
| La permission de consultation des organisations est-elle un garde suffisant ? | **Non** — elle est accordée au rôle `standard` (`030` § 6). Voir l'écart n° 73 |
| Le rôle `standard` est-il attribué à quelqu'un ? | **Non.** Vérifié dans le service d'inscription de B1 et dans `900_seed.sql` § 5, qui ne sème qu'un super-administrateur global. Voir l'écart n° 74 |
| L'unicité des adhésions exclut-elle les révoquées ? | **Non**, aucune condition de statut. Voir l'écart n° 72 |
| L'unicité du nom exclut-elle les fusionnées ? | **Oui** — `WHERE status IN ('candidate','active')`. C'est cette asymétrie avec l'adresse d'URL qui produit les écarts n° 70 et 71 |
| `compute_trust_score()` est-elle appelée quelque part ? | **Par personne.** Aucun trigger, aucun défaut de colonne. Confirmé par relecture du § 7 — d'où FR-063 |
| Le score maximal tient-il dans la colonne de la file ? | **Oui.** 100 + 40 + 10 + 25 = **175**, contre 9999,9 pour un `numeric(5,1)` |
| L'anti-rebond demandé par le prompt existe-t-il déjà ? | **Oui, 300 ms**, dans `UiSearchInput` — la valeur que le § 5 du modèle annonce. Ce jalon n'a rien à y ajouter |
| Où le front contourne-t-il l'écart n° 23 ? | **Deux endroits**, trouvés par balayage : `pages/organization/join.vue` et `components/proposal/StepOrganizations.vue`, tous deux filtrant sur `match_reasons`. Une fois l'API filtrée à la source, ces deux filtres deviennent inertes — à retirer en **B7** |
| Le module est-il déclaré en base ? | **Oui**, `platform.modules` : code `org`, schéma `org`, dépend de `identity`. FR-002 impose de le lire au démarrage |
| Existe-t-il un type de notification pour l'invitation ? | **Non.** `110_engagement.sql` § 11 n'en sème que deux — demande et approbation d'adhésion. L'invitation emprunte la chaîne simple de B1, comme la vérification d'adresse |
| Aucun code Rust du module n'existe ? | **Confirmé.** `backend/crates/modules/` ne contient que `identity` |

---

## Ce que l'implémentation a livré (20/08)

### Le crate

```
backend/crates/modules/org/
├── domain/      ids · permissions · search · organization · membership · admin · duplicates · merge
├── repo/        search · organizations · domains · memberships · names
│                admin_list · admin_detail · duplicates · merge · merge_counts
├── service/     search · join · create · membership · admin_list · admin_detail
│                admin_write · duplicates · merge
├── jobs/        duplicates · trust_score · scorecard · emails
├── routes/      public · memberships · admin · openapi
└── tests/       29 fichiers, tous sur base réelle et jetable
```

Trois endroits touchés **hors** du crate, comme le plan l'annonçait :

- **`kernel/src/tokens.rs`** — le service de jetons à usage unique y est remonté (R8). Les quatre tests de B1 sur la vérification d'adresse, la réinitialisation et le rejeu **n'ont pas été réécrits** : ils restent verts, et c'est la preuve du comportement constant.
- **`modules/identity/`** — deux modifications et pas une de plus : `repo/tokens.rs` supprimé au profit du noyau, et `service/registration.rs` corrigé (écart n° 81). `domain/token.rs` **réexporte** les trois types du noyau, pour que les appelants et les tests de B1 ne changent pas.
- **`.env.example`** — les trois réglages d'exploitation, validés au démarrage par `kernel::config::OrgConfig`.

### Les trois décisions structurantes, à l'usage

| Décision | Ce qu'elle a donné |
|---|---|
| **Deux lectures au-dessus de la même fonction** (R1) | `repo/search.rs` les déclare côte à côte, avec le tableau qui les oppose en tête de fichier. `deux_lectures_de_recherche.rs` éprouve qu'elles rendent des résultats **différents** sur la même requête |
| **Les arbitrages APRÈS l'appel de fusion** (R5) | `fusion_arbitrage_apres_lappel.rs` fait aboutir une fusion arbitrant le nom légal — celle qui aurait échoué dans l'autre ordre. `fusion_arbitrage_annule_tout.rs` prouve que la garantie d'A11 est intacte |
| **Le décompte composé depuis le registre** (R4) | `repo/merge_counts.rs`, seul fichier du module à composer son SQL, avec `AssertSqlSafe` et `quote_ident`. Dix-sept lignes sur dix-huit sont exactes au chiffre près — voir la réserve ci-dessous |

### Ce qui a changé par rapport au plan, et pourquoi

| Point | Ce que le plan disait | Ce qui a été fait, et la raison |
|---|---|---|
| **Le décompte de fusion** | « écart de zéro », ligne par ligne (SC-010) | **Dix-sept lignes sur dix-huit.** `identity.people.primary_organization_id` est déplacée par `tg_memberships_sync_primary` **avant** que la boucle du registre n'y arrive : le journal en compte moins que l'aperçu. Rien n'est perdu — plus personne ne pointe vers la fiche absorbée, et le test le vérifie. Corriger l'écart demanderait de reproduire l'effet d'un trigger dans un décompte, ce que le principe VIII interdit |
| **La liste du back-office** | projection analytique jointe | **Jointure par la gauche**, à partir de la table vivante : la projection n'est rafraîchie que par un travail différé, et une jointure interne faisait disparaître de l'écran les fiches créées depuis le dernier passage |
| **Le semis réaliste (T020)** | dans `tests/commun/mod.rs` | Dans **`tests/commun/seed.rs`**, avec la paire OSED. Les deux semis vont ensemble et le fichier resterait sinon à la limite de la lisibilité |
| **T034 — les remèdes de performance** | à appliquer si la mesure ne passe pas | **Sans objet** : la mesure passe largement (55 ms contre 150) |
| **`already_merged`** | rendu par le trigger `tg_forbid_merge_chains` | La fonction de fusion **refuse elle-même** une cible absorbée, avant que le trigger n'ait l'occasion de se déclencher. Le service distingue donc « introuvable » de « déjà fusionnée » en relisant la cible, et rend le message de la base tel quel. Le message du trigger — « Cibler la fiche finale » — reste éprouvé à part, par l'écriture directe qui seule l'atteint |

### Ce qui reste à faire à la main

- ~~**T068 et T125**~~ — **faits au navigateur le 20/08.** La chaîne du courriel va jusqu'à **Mailpit** : invitation émise par une référente, travail en file avec son jeton, worker, relais du site, courriel lu — sujet portant le nom de l'organisation, « Bonjour Invité·e » puisque le nom n'est pas déduit de l'adresse, lien et durée conformes. **Le premier essai a échoué** (`MAIL_RELAY_UNREACHABLE`, le front n'avait pas fini de démarrer) et **la reprise a réussi** : la chaîne d'essais fonctionne. L'acceptation **sans session** rend l'adhésion active et marque l'adresse vérifiée ; le rejeu du lien dit « déjà utilisé ». Côté site, rien n'a régressé : accueil, garde de connexion, espace organisation, liste du back-office avec ses facettes comptées, refus « droit nécessaire » plutôt qu'une liste vide, et file des doublons avec la paire OSED et ses quatre motifs.

  **Deux constats en sont sortis, consignés pour B7** : le lien du courriel d'invitation mène à `/invitation`, **page qui n'existe pas** — c'est le défaut que B1 avait déjà payé, et la route d'acceptation attend son écran ; et le bouton « Examiner la fusion » de la file ne navigue pas, la page de fusion existant pourtant et fonctionnant avec ses paramètres.
- ~~**T124**~~ — **fait le 20/08.** `make check` en entier : base détruite, schéma rechargé de zéro, assertions d'initialisation, `nuxt typecheck`, `npm run build`, `cargo fmt --check`, `clippy -D warnings` et `cargo test --workspace`. **249 tests, aucun échec.** Le typecheck du front a dû être réparé d'abord — il échouait depuis un moment sur une résolution de greffon, voir [`pieges.md`](../pieges.md). **Le stockage objet est reparti de zéro : relancer `make garage-init`.**
