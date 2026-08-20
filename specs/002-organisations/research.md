# Phase 0 — Décisions techniques

**Fonctionnalité** : Organisations (B2) · **Date** : 2026-08-20 · **Spécification** : [spec.md](spec.md)

Dix-neuf décisions. Chacune porte ce qui a été retenu, pourquoi, et ce qui a été écarté. Les décisions de B1 qui tiennent encore — forme du jeton d'accès, transport de la session, traduction des erreurs, relais d'outbox, file de travaux, harnais de test — **ne sont pas rejouées** : elles vivent dans [`specs/001-socle-identite/research.md`](../001-socle-identite/research.md) et s'appliquent telles quelles.

---

## R1 — Les deux lectures de la recherche : le filtre est en SQL, au-dessus de la fonction

**Décision.** Les deux routes appellent `org.find_similar_organizations()` **sans la modifier**. La lecture destinée à une personne l'enveloppe :

```sql
SELECT * FROM org.find_similar_organizations($1, $2, $3, $4, $5)
WHERE 'name_similarity' = ANY(match_reasons)
```

La lecture destinée à la revue des doublons ne l'enveloppe pas.

**Conséquence qu'il ne faut pas manquer : la limite est appliquée À L'INTÉRIEUR de la fonction.** Filtrer après coup rendrait moins de lignes que demandé — dix demandées, trois écartées, sept rendues. La lecture filtrée **sur-lit** donc : elle passe `limite + marge` à la fonction, filtre, puis tronque à la limite demandée. La marge est petite et bornée (défaut **+5**), parce que le nombre de fiches qui remontent par le seul domaine est le nombre de fiches déclarant le domaine de l'appelant — une, deux dans le cas des deux fiches OSED.

**Pourquoi pas en Rust.** Filtrer côté application coûterait le même aller-retour et ferait vivre la règle à deux endroits — la lecture filtrée et le balayage de détection, qui appelle la même fonction avec l'intention inverse. En SQL, la différence entre les deux lectures tient dans une ligne, lisible à côté de l'autre.

**Alternatives écartées.**
- *Ajouter un paramètre de filtrage à la fonction* — le prompt l'interdit, et à raison : le back-office a besoin du comportement actuel, qui est juste pour lui.
- *Retirer le bonus de domaine* — c'est le signal le plus fiable du modèle ; il ne fait plus **entrer** une fiche sans rapport, il continue de **hisser** celle qui correspond aussi par le nom (FR-007).
- *Filtrer sur le score plutôt que sur le motif* — le motif n'est posé qu'au-dessus de 0,3 quand l'opérateur trigramme fait entrer à partir de 0,3 : filtrer sur le score garderait une ligne que l'écran écarte, et l'API divergerait de l'interface sur un cas invisible (écart n° 77).

---

## R2 — Tenir 150 ms sur 5 000 organisations : ce qu'on mesure, et ce qu'on fait si ça ne passe pas

**Décision.** La cible se **mesure** avant de se traiter. Un test d'intégration sème 5 000 organisations, lance cent recherches de formes variées — sigle, début de nom, deux lettres, mot du milieu, terme sans résultat — et exige le 95ᵉ centile sous 150 ms côté serveur.

**Ce qui devrait suffire, et qui existe déjà :**

| Signal | Index servant la recherche |
|---|---|
| trigramme `%` sur les dénominations | `ix_organization_names_trgm` (GIN, `gin_trgm_ops`) |
| similarité de mot `<%` | le même index |
| préfixe `LIKE q \|\| '%'` | le même index — `gin_trgm_ops` sert `LIKE` |
| domaine | `ix_organization_domains_lookup` |
| nombre de membres actifs | `ix_memberships_org (organization_id, status)`, et la sous-requête ne porte que sur les lignes **rendues**, donc au plus la limite |

**Le point d'incertitude est nommé** : la fonction pose son terme normalisé dans une expression de table (`WITH input`) référencée **plusieurs fois**, ce que PostgreSQL matérialise par défaut. Le terme n'est donc pas une constante au moment de la planification, et l'usage de l'index GIN dépend de la capacité du planificateur à paramétrer le parcours dans une boucle imbriquée. C'est vérifiable et non devinable : le test rend un `EXPLAIN (ANALYZE, BUFFERS)` dans son message d'échec.

**Si la mesure ne passe pas**, l'ordre des remèdes est fixé d'avance, du moins intrusif au plus :
1. réviser la façon d'**appeler** la fonction — passer le terme déjà normalisé, réduire la marge de sur-lecture ;
2. augmenter la statistique sur `name_normalized` (`ALTER … SET STATISTICS`), qui ne change ni le schéma ni les résultats ;
3. **et seulement alors** proposer une modification du SQL, avec la mesure comme justification, ce que le prompt autorise à condition de la motiver explicitement. La forme pressentie serait une expression de table non matérialisée, qui ne change aucun résultat.

Aucune de ces trois marches n'est prise à l'avance : c'est l'objet de la tâche de mesure.

**Alternative écartée.** *Mettre la recherche en cache* — l'anti-rebond du front (300 ms) et la limite par défaut (10) rendent la charge dérisoire ; un cache ferait vivre des résultats périmés sur l'écran qui sert précisément à ne pas créer un doublon.

---

## R3 — Le semis de 5 000 organisations doit ressembler à la réalité, sinon la mesure ment

**Décision.** Les noms semés sont composés à partir d'un petit corpus francophone — natures (« réseau », « institut », « observatoire », « union »), domaines (« climat », « désertification », « biodiversité »), et pays — de sorte que la distribution des trigrammes soit celle d'un vrai référentiel : **beaucoup de noms partagent leurs premiers mots**, ce qui est exactement le cas défavorable de la recherche. Chaque fiche reçoit une dénomination supplémentaire (sigle) une fois sur deux.

**Pourquoi cela compte.** 5 000 noms tirés au hasard n'ont presque aucun trigramme commun : le parcours d'index rendrait une poignée de lignes et la mesure serait excellente et fausse. C'est le même piège que les données simulées du front, qui ont été écrites pour rejouer la fonction plutôt que pour être plausibles.

Le semis vit dans le harnais de test, **jamais dans `docs/database/`** : ce n'est pas une donnée du modèle.

---

## R4 — Le décompte de fusion : une seule requête composée depuis le registre

**Décision.** L'aperçu de fusion lit `org.organization_references` puis **compose une seule requête** en `UNION ALL`, une branche par ligne du registre, rendant pour chacune les trois chiffres : lignes qui basculeront, lignes supprimées parce que la cible porte déjà la valeur, lignes supprimées par stratégie.

C'est du **SQL composé dynamiquement**, donc non vérifié à la compilation. C'est assumé, borné à un seul fichier, et justifié :

- énumérer les tables en Rust est exactement ce que le registre existe pour éviter — dix-huit lignes aujourd'hui, davantage quand les modules hors jalon vivront ;
- `merge_organizations()` compose déjà son SQL de la même façon et à partir de la même source : le décompte annoncé et le décompte réel sont alors calculés par le même raisonnement, ce qui est la seule façon de tenir SC-010 ;
- les identifiants proviennent d'une table alimentée par le DDL, jamais par un utilisateur — ils sont **néanmoins** cités par `quote_ident`, parce qu'une table de configuration reste une table.

**Une seule requête et non dix-huit** : dix-huit allers-retours coûteraient une dizaine de millisecondes pour un écran qui se recharge à chaque inversion du sens.

**Alternative écartée.** *Ajouter une fonction `org.count_merge_transfers()` au modèle* — ce serait la plus élégante, et elle demande une modification du SQL que rien n'impose : le besoin est entièrement satisfait depuis l'application. À reproposer si un second appelant apparaît.

---

## R5 — L'ordre des deux écritures de la fusion : les arbitrages viennent APRÈS

**Décision.** Dans une seule transaction ouverte par la porte d'écriture du noyau :

1. contrôle d'autorisation (fusion, portée globale) et lecture des deux fiches ;
2. contrôle du nom de confirmation, et refus des cas prévus par le contrat ;
3. **appel de `org.merge_organizations(source, cible, motif)`** ;
4. **puis** application des arbitrages de champ, par une modification de la fiche survivante ;
5. relecture du décompte réel dans le journal des fusions, écrit à l'étape 3 ;
6. validation.

**Pourquoi cet ordre, et pas celui inscrit en A11.** L'unicité (nom normalisé, pays) ne porte que sur les fiches **vivantes**. Tant que la fiche absorbée l'est encore, la survivante ne peut pas reprendre son nom légal — et le nom légal est le champ qu'on arbitre le plus souvent. L'obligation d'A11 disait « avant l'appel, dans la même transaction » : la seconde moitié était juste et le reste, la première ne l'était pas. Deux transactions laisseraient une fiche à moitié complétée ; **cet ordre-là ne le permet pas davantage** — si l'étape 4 échoue, la 3 est annulée avec elle.

**Effet de bord voulu** : modifier le nom légal de la survivante fait entrer l'ancien nom de la fiche absorbée dans ses dénominations, par le trigger du modèle. Une recherche sur l'ancien nom continue donc de trouver la bonne fiche, ce qui est la promesse de la fusion.

---

## R6 — L'adresse d'URL n'est pas arbitrable, et le refus le dit

**Décision.** Le champ figure au comparatif — le contrat du front le range parmi les dix. Retenir celle de la fiche absorbée est **refusé**, par un code stable qui **nomme le champ**, et la survivante garde la sienne.

**Pourquoi.** Contrairement au nom, l'unicité de l'adresse d'URL ne connaît **aucune condition de statut** : la fiche absorbée garde la sienne pour toujours, puisqu'elle survit. Trois issues, deux mauvaises :

| Issue | Ce qu'elle coûte |
|---|---|
| Libérer l'adresse de la source en la suffixant | Casse la promesse même de la fusion — les anciennes adresses cessent de mener quelque part |
| Échanger les deux adresses | L'ancienne adresse de la survivante mène désormais à la **fiche absorbée** : pire qu'une adresse morte, une adresse qui ment |
| **Refuser, explicitement** | Un refus informatif, qui n'arrive que si l'opérateur choisit délibérément celle de la source |

Le refus n'arrive jamais par défaut : le contrat prévoit qu'un champ absent du dictionnaire d'arbitrage garde la valeur de la fiche survivante.

**La vraie réponse serait une table d'alias d'adresses**, qui n'existe pas et qui n'appartient pas à ce jalon. Le point est consigné pour B7.

---

## R7 — Reprendre une adhésion révoquée : un seul ordre, et c'est la base qui arbitre

**Décision.** La demande de rattachement est un unique `INSERT … ON CONFLICT (organization_id, person_id) DO UPDATE`, dont la mise à jour est **conditionnée à l'état révoqué**. Si aucune ligne n'en ressort, c'est qu'une adhésion vivante existe : on la relit et la réponse est « déjà membre ».

**Pourquoi.** L'unicité porte sur le couple **sans condition de statut** (écart n° 72) : lire puis écrire laisserait une fenêtre où deux demandes simultanées produiraient une violation de contrainte au lieu d'une réponse propre. Un seul ordre supprime la fenêtre, et la base tranche — c'est le motif éprouvé en B1 sur les deux courses de concurrence de la connexion.

**Ce que la reprise remet à zéro** : l'état, le rôle demandé, la fonction déclarée, la date de révocation. Ce qu'elle ne touche pas : la date de création, qui reste celle de la première demande — l'histoire de l'adhésion se lit dans le journal d'audit, pas dans une ligne réécrite.

---

## R8 — Le service de jetons à usage unique remonte dans le noyau

**Décision.** Émission, vérification, consommation et purge des jetons à usage unique quittent le module `identity` pour `kernel::tokens`. Le module `identity` **est réécrit pour l'appeler** dans le même jalon.

**Pourquoi maintenant.** Le modèle déclare cinq finalités, et **trois n'appartiennent pas à `identity`** : l'invitation est le geste de ce module-ci, la confirmation d'un intervenant sera celui de B4. Or aucun crate de module ne peut dépendre d'un autre. Les issues étaient :

| Issue | Ce qu'elle coûte |
|---|---|
| `org` écrit lui-même dans la table des jetons | Deux implémentations de « consommer un jeton atomiquement », qui divergeront — et c'est la seule opération du lot où une divergence se paie en jeton rejouable |
| Le jeton d'invitation devient un lien signé, hors modèle | Contredit le modèle, qui a prévu la finalité `invitation` pour cela, et l'écart n° 33, qui l'impose |
| **Le noyau porte le service** | Un déplacement d'environ deux cents lignes, et B1 à revalider |

Le noyau connaît déjà le schéma `identity` — c'est là que vit le garde d'autorisation (B1, R16), pour exactement la même raison. **Et la moitié du travail est déjà faite** : les cinq durées de validité par finalité vivent dans la configuration du noyau depuis B1, où elles avaient déjà l'air d'être au bon endroit.

**Le risque est nommé** : ce déplacement touche du code livré et éprouvé. Il se fait à comportement constant, et les tests de B1 sur la vérification d'adresse, la réinitialisation et le rejeu de jeton sont la preuve — ils ne sont pas réécrits, ils doivent rester verts.

La purge récurrente, elle, **reste une tâche du module `identity`** : c'est une opération d'exploitation, elle a son travail différé qui tourne, et le déplacer n'apporterait rien.

---

## R9 — Une personne créée par invitation ne pouvait pas se créer de compte : correction de B1

**Décision.** Le service d'inscription de `identity` est corrigé : une personne **connue mais dépourvue de compte** obtient un compte et un lien de vérification, au lieu du rappel « vous avez déjà un compte ». La réponse rendue reste **invariable**, et le coût de hachage reste payé dans tous les cas.

**Pourquoi c'est nécessaire ici.** L'invitation par adresse crée une personne **sans compte** — c'est ce que la séparation personne / compte permet, et ce que l'écart n° 33 demande. Or l'inscription, telle que B1 l'a livrée, branche sur « adresse connue » et envoie un rappel : la personne invitée ne pourrait **jamais** se créer de compte, et l'invitation resterait une demi-fonctionnalité. Le défaut appartient à B1 ; il ne s'est vu qu'en écrivant B2.

**Pourquoi ce n'est pas une brèche.** Le chemin est celui de l'inscription ordinaire : l'adresse est prouvée par le lien de vérification avant que le compte ne serve à quoi que ce soit. Une personne sans compte n'a par définition aucun secret à voler. Et la réponse ne change pas de forme, donc le formulaire d'inscription ne devient pas l'annuaire des personnes.

**Alternative écartée.** *Faire créer le compte par la route d'acceptation de l'invitation* — la création de compte appartient à `identity`, et la faire depuis `org` serait la première entorse au principe II de tout le projet.

---

## R10 — Accepter une invitation n'exige pas de session

**Décision.** La route d'acceptation prend **le jeton**, et rien d'autre. Si une session existe, elle doit désigner la même personne que le jeton, sinon l'acceptation est refusée.

**Pourquoi.** Le jeton **est** la preuve d'adresse, comme pour la vérification d'adresse de B1, qui n'exige pas non plus de session. Exiger une session rendrait l'invitation inutilisable par la personne qu'elle vise le plus souvent : celle qui n'a pas encore de compte. Le contrôle de correspondance, lui, empêche le seul cas gênant — quelqu'un de connecté qui suit le lien reçu par un collègue et entre à sa place.

**Ce qui suit l'acceptation.** L'adhésion devient active, la base attribue le rattachement principal si c'est le premier, et l'adresse de la personne est **marquée vérifiée** si elle ne l'était pas : le lien l'a prouvée, et laisser la personne redemander un second lien pour la même adresse serait une formalité vide.

---

## R11 — Le balayage de détection : par tranches, avec curseur, et se replanifiant

**Décision.** Une tâche récurrente parcourt les fiches vivantes **par tranches** (défaut 200), chaque exécution posant la suivante avec son curseur ; la dernière tranche planifie le passage du lendemain. Pour chaque fiche, la lecture non filtrée de R1 est appelée avec ses propres nom, pays, adresse de contact et site, et chaque paire dont le score atteint le seuil est consignée :

```sql
INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
VALUES (LEAST($1,$2), GREATEST($1,$2), $3, $4)
ON CONFLICT (left_id, right_id) DO UPDATE
   SET score = EXCLUDED.score, reasons = EXCLUDED.reasons, detected_at = now()
 WHERE org.duplicate_candidates.reviewed_at IS NULL
```

Une seule ligne tient les deux moitiés de FR-059 : une paire **déjà arbitrée** n'est pas ressuscitée, une paire **en attente** est mise à jour.

**Pourquoi par tranches.** Cinq mille appels d'une recherche à quelques dizaines de millisecondes font une à trois minutes : c'est acceptable la nuit, mais pas dans une seule transaction, et pas dans un travail qu'un redémarrage ferait reprendre de zéro. La clé d'unicité porte le jour et le curseur, ce qui rend le rejeu inoffensif — le motif de la purge récurrente de B1, réemployé tel quel.

`LEAST`/`GREATEST` tiennent l'ordre que la base impose à la paire ; sans eux, une paire sur deux serait refusée par une vérification et le message parlerait d'une contrainte.

---

## R12 — Le score de confiance : travail différé, coalescé, écrit seulement s'il change

**Décision.** Un travail différé par organisation, **clé d'unicité `org.trust_score:<organisation>`**, déclenché à quelques secondes de délai par les écritures qui affectent le score. Il écrit :

```sql
WITH calcul AS (SELECT org.compute_trust_score($1) AS score)
UPDATE org.organizations o
   SET trust_score = c.score
  FROM calcul c
 WHERE o.id = $1 AND o.trust_score IS DISTINCT FROM c.score
```

**Pourquoi pas un trigger.** Le score est alimenté par quatre tables, dont les adhésions et les domaines : un trigger recalculerait un agrégat à **chaque adhésion approuvée**, sur un chemin d'écriture fréquent, pour une valeur qui ne sert qu'à trier une liste de back-office. Le principe VIII ne s'applique pas — rien n'est faux si le score a dix secondes de retard, ce n'est pas un invariant. Et le § 7 du modèle annonçait déjà « recalculé par le worker » : la décision rend vrai un commentaire qui ne l'était pas.

**La condition d'écriture n'est pas une optimisation** : sans elle, chaque recalcul poserait une ligne d'audit et remonterait la date de dernière modification de la fiche, donc son rang dans le tri « dernière activité ». L'historique de la fiche se remplirait de lignes que personne n'a écrites.

**L'acteur est absent, et c'est dit.** Un recalcul de système n'a pas d'auteur. C'est la deuxième trace anonyme légitime du projet, après l'inscription de soi-même ; elle est nommée ici pour qu'elle ne se découvre pas en lisant un journal d'audit.

---

## R13 — Le rafraîchissement de la projection : hors transaction, et débouncé

**Décision.** Un travail différé rafraîchit `analytics.mv_organization_scorecard` **en concurrence** (l'index unique requis existe), avec une clé d'unicité qui coalesce les demandes sur une fenêtre de quelques minutes.

**Pourquoi un travail séparé.** Un rafraîchissement en concurrence **ne peut pas s'exécuter dans un bloc de transaction** : il ne peut donc pas accompagner l'écriture qui le rend nécessaire. Et il n'a pas à le faire — la liste relit sur la table vivante les quatre colonnes qui bougent au geste de l'opérateur (FR-048), si bien que le retard de la projection ne porte que sur des compteurs que personne ne regarde en posant un sceau.

**Alternative écartée.** *Rafraîchir à chaque écriture, sans coalescence* — sur 5 000 organisations le rafraîchissement se compte en secondes ; l'enchaîner à chaque approbation d'adhésion ferait tourner le worker en permanence pour un tableau de bord.

---

## R14 — La liste du back-office : une requête, et une lecture qui franchit la frontière

**Décision.** La liste, ses facettes et ses compteurs de doublons tiennent en **une requête**, composée de la projection analytique, des quatre colonnes vives relues sur la table, et du filtre de périmètre. Le filtre s'écrit :

```sql
is_global
OR EXISTS (dossier déposé dans une édition administrée, comme porteuse ou co-organisatrice)
OR EXISTS (séance tenue dans une édition administrée)
```

**Cette condition lit le schéma `programme`.** C'est une **lecture qui franchit une frontière de schéma**, pas une dépendance de crate ni un appel de module à module — la distinction est celle que B1 avait relevée en écart n° 11 et laissée « à décider en B2 ». Elle est décidée ici, et dans le seul sens conforme : demander l'information au module `programme` serait un appel direct d'un module à un autre, que le principe IV interdit sans détour. Le registre `org.organization_references` déclare d'ailleurs ces mêmes colonnes depuis le premier jour : le modèle a prévu que `org` connaisse qui le référence.

**La règle qui en sort, et qui vaudra pour B3 à B6** : un module **lit** hors de son schéma quand la question porte sur ses propres entités ; il n'**écrit** jamais ailleurs, et il n'appelle jamais un autre module.

**Les facettes sont comptées dans la même requête**, sur le même jeu de lignes (FR-046) : les demander à part ferait diverger « Sénégal (3) » de ce qui s'affiche, au premier filtre ajouté.

---

## R15 — La fiche complète : plusieurs lectures dans une transaction, assemblées en Rust

**Décision.** La fiche d'une organisation est composée de huit lectures — identité et sceau, fiche de performance, dénominations, domaines et leurs partages, membres, activités, historique, fusions, paires ouvertes — exécutées dans une **seule transaction de lecture** et assemblées en Rust.

**Pourquoi pas une requête unique rendant du JSON.** Elle serait illisible, non vérifiée colonne par colonne à la compilation, et impossible à faire évoluer sans la relire en entier. La fiche n'est pas un chemin chaud : elle s'ouvre à la main, une fois. C'est le parti déjà retenu en B1 pour la fiche d'un utilisateur.

**Ce qui franchit la frontière ici** : les activités (dossiers et séances) et le nom des personnes. Même règle qu'en R14.

---

## R16 — Le garde d'autorisation : il n'y a rien à écrire

**Décision.** B2 n'ajoute **aucune** pièce d'autorisation. Le noyau porte déjà tout ce dont ce module a besoin, et B1 l'a construit pour lui :

| Besoin de B2 | Ce que le noyau offre déjà |
|---|---|
| La liste du back-office : permission **quelque part** + périmètre non vide | `require_permission_anywhere`, `require_perimeter`, extracteur `Perimeter` |
| La fusion : permission sur la portée **globale** | `require_permission(…, Scope::Global)` |
| Le sceau, les domaines, les dénominations : permission de gestion | `require_permission` |
| Une fiche hors périmètre, y compris par URL forgée | `Perimeter::ensure` |

Le seul ajout est la **déclaration des trois permissions du module** comme spécifications de permission, à côté de celles d'`identity`.

**Le référent d'une organisation n'est pas une permission mais une qualité** : inviter et décider se vérifient sur l'adhésion de l'appelant (rôle référent, état actif, dans **cette** organisation), lue en base à chaque écriture. Un rôle d'organisation existe bien dans le modèle (`org_manager`, portée organisation), mais rien ne l'attribue et il ne porte que deux permissions étrangères à ce module — soumettre une proposition, rédiger un article. Le tester serait tester un nom de rôle, ce que le principe V interdit ; et l'adhésion, elle, est la donnée que le modèle a prévue pour cette question.

---

## R17 — Statuts HTTP et codes d'erreur : la règle de B1, reprise sans changement

**Décision.** La règle établie en B1 s'applique telle quelle : **un refus prévu par le contrat du front comme membre d'union sort en 200 avec son discriminant** ; tout autre refus sort en statut d'erreur avec le corps d'erreur du noyau.

Six refus de ce module sont dans le contrat et sortent donc en 200 : « déjà membre », « nom déjà pris », « déjà invitée », « domaine déjà pris », « nom de confirmation incorrect », « déjà fusionnée ». Le catalogue des codes ajoutés est dans [`contracts/errors.md`](contracts/errors.md).

**Ce qui n'est pas dans le contrat sort en erreur** : le refus d'un champ non arbitrable (R6), l'absence de qualité de référent, le périmètre, l'autorisation.

---

## R18 — Tests : le harnais de B1, et les quatre obligations de la constitution

**Décision.** Les tests réutilisent `kernel::testing` — base modèle chargée une fois depuis `docs/database/`, recopiée par test. Aucun mock de base.

Les quatre obligations du principe X ont chacune leur test nommé :

| Obligation | Test |
|---|---|
| Chemin nominal de chaque route | `recherche_multi_signaux`, `rattachement_et_creation`, `adhesions_deux_files`, `back_office_liste_et_fiche`, `fusion_complete` |
| Refus par périmètre, **URL forgée comprise** | `perimetre_organisation_url_forgee` |
| Traduction d'un invariant de la base | `fusion_cible_deja_fusionnee` (message du trigger, mot pour mot) et `domaine_deja_verifie_ailleurs` |
| Écriture des événements attendus | `outbox_une_seule_fusion` — et il compte, il ne vérifie pas seulement la présence |

Deux tests de plus tiennent des critères que rien d'autre ne tiendrait : `recherche_150ms` (SC-002, avec son plan d'exécution en cas d'échec) et `adhesion_revoquee_puis_redemandee` (SC-008).

---

## R19 — Ce que ce module ne fait toujours pas

- **Valkey reste inutilisé.** Aucune exigence ne demande de limitation de débit ni de cache, et R2 explique pourquoi un cache de recherche serait nuisible ici. Le point reste nommé, comme en B1 (§ R12).
- **La vérification d'un domaine par enregistrement DNS ou par courriel** n'entre pas dans ce jalon : le modèle porte les trois méthodes, seule la manuelle est livrée, et c'est ce que le contrat du front annonce déjà.
- **L'affichage du nom traduit d'une organisation** dépend de la question n° 2 en attente auprès du commanditaire. Les traductions sont collectées, indexées et cherchables ; leur affichage se décidera avec l'arbitrage.
- **La composition riche et multilingue des courriels** appartient au module Engagement (B6). Les trois courriels de ce module empruntent la chaîne simple de B1.
