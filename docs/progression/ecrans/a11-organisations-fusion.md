# A11 — Organisations et fusion

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 18/08. **Le modèle a été corrigé sur un point, trouvé en éprouvant l'écran** : `org.organization_references` déclarait `organization_domains` sans `dedupe_on`, si bien qu'une fusion laissait la fiche absorbante avec deux lignes pour un même domaine — au moment même où l'on corrigeait un doublon. 4 pages (`/admin/organisations`, `/doublons`, `/fusion`, `/:id`), 9 composants, 2 utilitaires purs, 1 fichier de contrats, 1 dossier de mocks en cinq fichiers dont **`session.ts`, le journal d'écritures qui rejoue `org.resolve_organization()` par REDIRECTION plutôt que par réécriture**, 1 fabrique d'API, 8 fichiers de traduction. La liste est triée par score de confiance CROISSANT — la première ligne est toujours la fiche à regarder. **Le décompte de transfert est lu dans le registre `org.organization_references`**, jamais dans une liste de tables écrite dans un composant : les trois sorts (transféré, supprimé car déjà présent, supprimé) y sont distingués. **Deux défauts transverses corrigés** : `UiCounter` étendait la page de 163 px sous 375 px (son `sr-only` absolu n'avait aucun ancêtre positionné — la barre d'onglets d'A10 en souffrait aussi), et l'historique affichait ses instants et ses statuts en valeurs brutes

---

## Reprise du 20/08 — le bouton « Examiner la fusion »

Le défaut consigné le 20/08 — « le bouton ne navigue pas » — **ne se reproduit pas**. Huit conditions ont été
éprouvées au navigateur : chargement direct de l'URL, navigation interne depuis la liste des organisations, après
ouverture puis fermeture de la modale « ce ne sont pas des doublons », après un aller-retour vers l'écran de fusion,
à 375, 1280 et 1920 px, en français et en anglais, par un vrai clic souris comme par un clic programmé. L'écran de
fusion s'ouvre à chaque fois sur la paire OSED.

**La piste `localePath()` est écartée par la mesure, pas par le raisonnement.** Instrumenté dans la page, il rend
`/admin/organisations/fusion?gauche=…` en français et `/en/admin/organisations/fusion?…` en anglais : la forme objet
`{ path, query }` se résout correctement, parce que sous `prefix_except_default` la résolution par chemin se réduit à
`router.resolve`. Deux autres écrans l'emploient déjà — `admin/organisations/[id].vue` compose ainsi ses deux liens de
fusion.

**Une seule chose a été durcie, et elle est réelle** : `openMerge()` ignorait la promesse de `navigateTo`. Un refus de
navigation — garde de route, route disparue — n'aurait laissé aucune trace, ce qui est exactement le symptôme décrit.
Elle est désormais attendue.

**Un défaut trouvé en chemin, non corrigé faute de mandat** : « Remettre dans la file » ne remet pas la paire dans la
file. La répartition entre « à trancher » et « déjà tranchées » se fait sur `reviewed_at`, que toute décision
renseigne — y compris `deferred`. Consigné dans [`points-bloques.md`](../points-bloques.md).

---

## Écarts relevés en écrivant les organisations et la fusion (A11, 18/08)

**Un défaut du MODÈLE, corrigé avant d'écrire l'écran de fusion** (registre des références, voir le tableau plus haut).
Suivent cinq points qui ne se tranchent pas depuis un écran.

1. **`org.merge_organizations()` NE PREND AUCUN CHOIX DE CHAMP, et le prompt en demande.** La fonction réaffecte les
   rattachements et laisse la fiche cible telle quelle ; « choix de la valeur à conserver pour chaque champ divergent »
   est donc un `UPDATE org.organizations` sur la CIBLE, à faire **avant** l'appel et **dans la même transaction**.
   L'ordre n'est pas interchangeable : deux transactions laisseraient une fiche absorbante à moitié complétée si la
   fusion échouait ensuite. **Obligation inscrite au prompt B2.** Ajouter un paramètre à la fonction SQL serait le
   mauvais geste : la fusion est une opération de rattachement, pas un formulaire d'édition, et lui faire porter dix
   colonnes optionnelles la rendrait impossible à faire évoluer.

2. **Le périmètre d'administration ne peut pas se lire comme ailleurs : une organisation n'appartient à aucune
   édition.** La règle métier n° 8 a donc été prise par l'autre bout, en deux temps — `org.organization.read`, quelle
   que soit sa PORTÉE, ouvre l'écran ; la liste, elle, ne montre que les organisations ayant déposé ou tenu une
   activité dans les éditions administrées, et le dit en toutes lettres quand elle a restreint. La fusion, elle,
   exige la portée GLOBALE : elle déplace des rattachements dans toutes les éditions à la fois, il n'existe pas de
   fusion « limitée à une COP ». Cela a demandé une fonction de plus dans `utils/permissions.ts` —
   `hasPermissionOnAnyScope()` —, à n'employer que sur les écrans sans édition. **À confirmer au prompt B2.**

3. **`analytics.mv_organization_scorecard` est MATÉRIALISÉE, et l'écran ne le dit pas encore.** Le tableau de bord (A6)
   affiche l'âge du dernier rafraîchissement ; cette liste, non. Sur les mocks la question ne se pose pas — les
   chiffres sont recalculés à la lecture —, mais en production une fiche vérifiée il y a dix minutes montrera son
   ancien score de confiance jusqu'au prochain passage du worker. **À reprendre au prompt B2** : soit la liste
   affiche l'âge de la projection, soit `score_confiance` est relu depuis `org.organizations`, qui est à jour.

4. **`org.compute_trust_score()` n'est appelée par AUCUN trigger.** La fonction existe (§ 7) et
   `organizations.trust_score` est une colonne ordinaire : quelqu'un doit l'écrire. Le § 7 dit « recalculé par le
   worker », et rien dans le modèle ne le garantit. Le front rejoue donc le calcul pour que la liste bouge quand on
   pose un sceau ou qu'on vérifie un domaine — ce que fera le worker en production. **À trancher au prompt B2** : une
   tâche différée déclenchée par `platform.emit_event`, ou un trigger sur les quatre tables qui alimentent le score.

5. **Les statuts de SÉANCE n'avaient aucun bloc de traduction canonique.** Ceux d'un dossier existent depuis A7
   (`admin.proposals.status.*`) ; le planificateur, lui, nomme des actions et pas des états. La fiche d'une
   organisation les déclare donc dans son propre fichier (`activities.sessionStatus.*`). **Le jour où un deuxième
   écran en a besoin, ce bloc remonte dans `_common.json`** — c'est le sens du déplacement, jamais l'inverse.

6. **Le journal d'écritures des mocks ne survit pas à un rechargement de page, et ne peut pas.** Il vit dans un module
   (comme celui d'`organization-search.ts`) : une navigation interne le conserve, un rechargement complet repart du
   jeu de données écrit à la main. C'est assumé et documenté en tête de `mocks/admin-organizations/session.ts` — mais
   la fusion est la première écriture du jalon dont l'EFFET est le sujet de l'écran, et il faut le savoir en faisant
   une démonstration : la fusionner puis recharger la page la fait « revenir ».

---

## Ce qui a été vérifié le 18/08 sur les organisations et la fusion, et comment

`npm run typecheck` et `npm run build` au vert. Le reste au navigateur, connecté comme administratrice globale, sur les
données simulées.

- **La liste** rend les treize fiches, triées par score de confiance CROISSANT sans paramètre d'URL : « OSED — 10 »
  d'abord, la fiche créée sous son seul sigle et que personne n'a regardée. Les facettes sont comptées sur le jeu
  affiché (Burkina Faso (2), ONG / Association (5)…), et la pastille « File des doublons 1 » relie les deux écrans.
- **Le ratio d'acceptation** s'affiche en pourcentage entier là où il existe, et **en tiret** — avec son explication en
  infobulle — pour une organisation qui n'a rien déposé. Un « 0 % » y ferait passer qui n'a jamais essayé pour qui
  échoue à chaque fois.
- **La file** ouvre sur la paire OSED à 95 % — « Correspondance forte » —, motifs nommés du plus probant au moins
  (« Les deux fiches déclarent osed-sahel.org : c'est le signal le plus fiable »), et la paire IMRE / CUDCM rangée
  sous « Déjà tranchées », avec son bouton de remise en circulation.
- **L'écran de fusion, éprouvé de bout en bout.** Le sens est proposé (la fiche vérifiée absorbe) et inversable ;
  quatre champs sont marqués « à trancher », les six autres se répartissent entre « identique » et « renseigné d'un
  seul côté » ; le bouton de fusion reste inerte tant qu'un arbitrage manque OU que le motif est vide.
- **Le décompte lu dans le registre** : Dénominations 2, Domaines de courriel 1 **supprimé car déjà présent**,
  Membres 1, Rattachements principaux 1, Dossiers déposés 1, Intervenants 2, Co-organisations 1, Inscriptions 3 —
  « 12 éléments déplacés au total ». **C'est ce décompte qui a fait apparaître le défaut du modèle** : la ligne des
  domaines annonçait « 1 transféré » alors que l'écran affichait le même domaine « déjà déclaré » deux lignes plus bas.
- **La confirmation refuse un nom qui ne correspond pas** (« Verdeo » sur la fusion d'OSED : « Ce nom ne correspond
  pas à la fiche absorbée »), et accepte le SIGLE en minuscules — la casse et les accents n'entrent pas en compte,
  comme `platform.normalize_label()`.
- **Après la fusion** : « 11 éléments déplacés » (les 12 moins le domaine dédoublonné, qui n'est pas un déplacement),
  la file repasse à zéro paire à trancher et la paire porte « Fusionnée », la fiche conservée passe à 3 membres,
  4 dossiers, 5 dénominations, **un seul** domaine et une confiance de 95, et son onglet Historique porte la fusion
  avec son auteur, son motif et son décompte.
- **La fiche absorbée reste consultable** : bandeau « Cette fiche a été absorbée par … Elle reste consultable, et ses
  anciennes adresses continuent de fonctionner », compteurs à zéro, ratio en tiret, action de fusion retirée. C'est la
  promesse de `org.resolve_organization()`, tenue à l'écran.
- **Aucun défilement horizontal à 375 px** sur les quatre écrans — mesuré : `document.documentElement.scrollWidth`
  vaut **375**. Il valait **538** sur la fiche avant correction : le `sr-only` de `UiCounter` est en position absolue
  et n'avait aucun ancêtre positionné, il s'échappait donc de la barre d'onglets défilante et étendait la page.
  **Le défaut n'appartenait pas à cet écran** — les six onglets d'A10 en souffraient depuis le 18/08.
- **Anglais** : `/en/admin/organisations` rend l'écran complet, facettes de pays comprises (« Benin (1) »,
  « Morocco (1) »), sans clé brute.
- **Thème sombre à 375 px**, capture pleine page de l'écran de fusion : les trois natures de ligne se distinguent
  (fond jaune pâle pour ce qui est à trancher), le bloc « Ce que la fusion préserve » reste lisible sur fond bleuté.
- **Aucun fichier de code applicatif > 1000 lignes.** Le plus long des fichiers créés est
  `types/admin-organizations.ts` (635 lignes) ; `useApi.ts` atteint **987 lignes** — la prochaine fabrique d'API à en
  sortir devra l'être avant d'ajouter quoi que ce soit d'autre.
- **Un piège d'interaction, relevé au passage** : un clic simulé sur un `<input type="radio">` lié par `:checked`
  (et non `v-model`) n'était pas pris en compte par l'outil d'automatisation, alors qu'un `element.click()` du
  navigateur l'était. Le comportement réel est bon — vérifié à la souris et au clavier.

## 31/10 : l'équipe accepte une demande et change un rôle

Demande du commanditaire, transmise par une autre session : l'IFDD n'avait plus de référent actif, sa propre demande de rattachement attendait, et ni l'espace ni le back-office ne pouvaient l'accepter (`DetailMembers.vue` était en lecture seule par choix). Maquette validée : https://claude.ai/artifact/8ghG6Aa7gG9pkuRMwJvRjg.

- **API** : `PUT /admin/organizations/{id}/members/{membershipId}/decision` (`approved`, `role` facultatif) et `…/role` (`role`, `confirm_last_manager`). `org.organization.manage` en **portée globale** (`Requires<OrganizationManage>`) : qui parle au nom d'une organisation ne relève d'aucune édition. Service `service/admin_members.rs` ; l'adhésion doit appartenir à l'organisation du chemin (URL forgée : `not_found`).
- **Règles gardées** : une invitation ne s'accepte pas à la place de la personne (écart n° 33, refus `ORG_MEMBERSHIP_IS_INVITATION`, et `invited_at IS NULL` dans la requête elle-même) ; retirer son rôle au dernier référent actif rend `last_manager` sans `confirm_last_manager` (FR-041). Nouveau code `ORG_MEMBERSHIP_NOT_ACTIVE` (rôle d'une adhésion non active). Événement `org.membership.role_changed` ; l'acceptation émet `org.membership.approved` et met en file le courriel d'approbation, comme celle d'un référent. Audit : `tg_memberships_audit` (voir `modele.md`).
- **Écran** : bandeau « Cette organisation n'a aucun référent », colonne Actions (« Accepter… » et « Refuser » sur une demande, « Attend la réponse de la personne » sur une invitation, « Changer le rôle » sur un membre actif), fenêtre de choix du rôle (Référent coché d'office quand il n'y en a aucun), avertissement et bouton rouge « Retirer quand même » pour le dernier référent. Actions visibles seulement avec la permission en portée globale (`canManageMembers`).
- **Vérifié** : `adhesions_par_l_equipe` (4 tests : acceptation comme référent avec audit nommant l'équipe, invitation refusée, dernier référent sur confirmation, demande sans changement de rôle et URL forgée), `dernier_referent`, `adhesions_deux_files`, `back_office_liste_et_fiche` verts ; typecheck, contrat d'API, `test:site` (71). Au navigateur, sur les données d'exemple (fiche du ROAC, 1440 px) : retrait du seul référent avec avertissement, bandeau, acceptation de Céline Lambert comme référente, bandeau retiré.
- L'aide sous « Contributeur » (« Participe aux dossiers qu'on lui confie ») est une proposition : le modèle ne dit pas ce que ce rôle permet.

## 31/10 : seul l'administrateur nomme un référent ; le créateur d'une fiche en attente est provisoire

Trois règles validées par le commanditaire, transmises par la session epavillon-v2-8b. Raison : n'importe qui pouvait créer une fiche au nom d'une organisation qui n'est pas la sienne et en devenir référent actif d'emblée.

1. **Seul l'administrateur nomme ou change un référent**, depuis la fiche du back-office (section précédente). Une invitation au rôle de référent est désormais refusée : `ORG_MANAGER_APPOINTED_BY_TEAM` ; le choix « Référent » a quitté le formulaire d'invitation de l'espace. Les invitations au rôle de référent déjà envoyées et non acceptées restent telles quelles.
2. **Le domaine vérifié fait un membre, jamais un référent** : `service/join.rs` rattachait déjà en `Member`, rien à changer.
3. **Le créateur d'une fiche `candidate` est référent provisoire**, dérivé du statut de la fiche, sans état en base. Il dépose ses dossiers (le rôle `org_manager` posé par `tg_memberships_sync_role` reste), mais `invite` et `decide` rendent `ORG_PENDING_VALIDATION` tant que la fiche n'est pas validée. Rien n'est défait sur les fiches `candidate` existantes : seuls les gestes suivants sont refusés. Espace : badge « Référent provisoire », encadré « Votre fiche attend sa validation par l'équipe de l'ePavillon », boutons d'acceptation et formulaire d'invitation masqués. Back-office : même badge et encadré invitant, à la validation, à confirmer ce référent ou à en nommer un autre (confirmer = le laisser en place).

**Vérifié** : `referent_provisoire` (3 tests : pas d'invitation ni d'acceptation sur une fiche en attente, l'existant reste ; pas d'invitation comme référent), toute la suite du module sauf `decompte_de_fusion_exact` (19 tables au registre pour 18 attendues, sans lien avec ce travail) ; décomptes de `routes/openapi.rs` remis à jour (22 chemins, 23 opérations, 14 codes : ils échouaient depuis 8c47244a). Typecheck, contrat, `test:site` verts. Non vu au navigateur. Les données d'exemple hors ligne n'appliquent pas ces deux refus.

## 31/10 : l'équipe modifie la fiche

Demande du commanditaire transmise par epavillon-v2-8b, d'après la maquette validée `docs/maquettes/dossier-admin/` (panneau « Organisation », partie « Modifier la fiche ») : aucune route ne modifiait une fiche, l'ancienne plateforme permettait au moins le nom et le logo.

- **API** : `PUT /admin/organizations/{id}/profile` (`legal_name`, `acronym`, `organization_type_code`, `country_id`, `website`), permission de gestion bornée par le périmètre comme le sceau. Rend la fiche recomposée ; `name_taken` nomme la fiche vivante qui porte déjà ce nom dans ce pays (`ux_organizations_name_country`) ; type inconnu ou inactif : `VALIDATION_FAILED` sur `organization_type_code`. Événement `org.organization.updated` (colonnes changées), audit par `tg_organizations_audit`. L'adresse (`slug`) ne change pas. Le logo passe par le module média (garde `org.organization.manage`).
- **Règle n° 1** : l'ancien nom et l'ancien sigle deviennent des anciens noms, toujours trouvés (voir `modele.md`).
- **Écran** : bouton « Modifier la fiche » dans l'en-tête, tiroir `AdminOrganizationsProfileDrawer` (props `organizationId`, `v-model:open` ; événement `saved(detail)`), qui charge lui-même la fiche, les pays, les types, la règle et le logo : il s'ouvre aussi depuis la fiche d'un dossier. Avertissement « La fiche est partagée : la corriger ici la corrige pour ses N dossiers et ses N membres » (dossiers déposés et membres actifs du tableau de bord), logo, nom, sigle, pays, type, site, rappel que l'ancien nom reste connu.
- **Vérifié** : `fiche_corrigee` (2 tests : renommage avec ancien nom et ancien sigle en `former` et retrouvés par `find_similar_organizations`, un seul nom légal, événement ; homonyme nommé sans rien changer, type inconnu refusé), suite du module verte sauf `decompte_de_fusion_exact` ; typecheck, contrat, `test:site`. Au navigateur sur les données d'exemple (fiche du ROAC, 1440 px) : tiroir conforme, sigle changé et « Modification enregistrée ».
