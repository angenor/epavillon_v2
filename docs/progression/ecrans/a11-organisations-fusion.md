# A11 — Organisations et fusion

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 18/08. **Le modèle a été corrigé sur un point, trouvé en éprouvant l'écran** : `org.organization_references` déclarait `organization_domains` sans `dedupe_on`, si bien qu'une fusion laissait la fiche absorbante avec deux lignes pour un même domaine — au moment même où l'on corrigeait un doublon. 4 pages (`/admin/organisations`, `/doublons`, `/fusion`, `/:id`), 9 composants, 2 utilitaires purs, 1 fichier de contrats, 1 dossier de mocks en cinq fichiers dont **`session.ts`, le journal d'écritures qui rejoue `org.resolve_organization()` par REDIRECTION plutôt que par réécriture**, 1 fabrique d'API, 8 fichiers de traduction. La liste est triée par score de confiance CROISSANT — la première ligne est toujours la fiche à regarder. **Le décompte de transfert est lu dans le registre `org.organization_references`**, jamais dans une liste de tables écrite dans un composant : les trois sorts (transféré, supprimé car déjà présent, supprimé) y sont distingués. **Deux défauts transverses corrigés** : `UiCounter` étendait la page de 163 px sous 375 px (son `sr-only` absolu n'avait aucun ancêtre positionné — la barre d'onglets d'A10 en souffrait aussi), et l'historique affichait ses instants et ses statuts en valeurs brutes

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
