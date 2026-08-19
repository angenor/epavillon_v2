# A7 — Liste des propositions

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 18/08. **Le modèle a été corrigé d'abord** : `v_proposal_dashboard` gagne treize colonnes (format, pays du porteur, thématiques prêtes à afficher, co-organisateurs, révisionnistes nommés, retards, lectures) et le fichier `programme.unread_proposals_for()`. 1 page, 4 composants sous `app/components/admin/proposals/`, 2 utilitaires purs, 1 fichier de contrats, 4 fichiers de mocks — dont **`permissions.ts`, qui rend enfin exécutable la règle « tester une permission, jamais un rôle »** —, 2 fichiers de traduction. Les 40 lignes, onze colonnes, tri sur chacune (note décroissante par défaut), huit filtres à facettes comptées, sélection multiple et trois actions groupées, export CSV, indicateur discret des dossiers non consultés. `UiTable` gagne `hideBelow` (paliers 1 024 / 1 280 / 1 536 px) et `rowLabelKey` (une case à cocher s'annonçait « Sélectionner la ligne 0198c1a0-… »)

---

## Écarts relevés en écrivant la liste des propositions (A7, 18/08)

Un défaut du modèle, **corrigé dans le SQL avant d'écrire une ligne d'interface** (voir « Modifications du modèle »). Trois autres points sont des manques que le modèle assume et qui appartiennent à l'API ou à un prompt ultérieur.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **46** | **RÉGLÉ le 18/08 — `v_proposal_dashboard` ne portait rien de ce que la liste AFFICHE.** Elle donnait l'avancement des revues, la note, le rang, les demandes de correction ouvertes — mais ni le format, ni le pays, ni les thématiques, ni les co-organisateurs, ni le nom des révisionnistes, ni le moindre signal de retard | `070` § 7 | Six lectures de plus par écran, et surtout la correspondance code → libellé de thématique refaite dans le frontend | **Treize colonnes ajoutées à la vue**, sur le modèle de `v_public_schedule` : `theme_codes` pour filtrer, `themes` pour afficher |
| **47** | **RÉGLÉ le 18/08 — « non consulté par moi » n'était calculable nulle part.** `proposal_reads` existe depuis l'origine, mais aucune fonction ne croisait une édition et un lecteur | `070` § 7 | L'écran aurait chargé la table entière des accusés de lecture pour en déduire un booléen par ligne | **`programme.unread_proposals_for()`**, fonction et non colonne : « non lu » dépend du lecteur |
| **48** | **LE MODÈLE N'A PAS DE PERMISSION « AFFECTER UN RÉVISIONNISTE ».** `030_identity.sql` sème `programme.proposal.decide` (décider) et `programme.review.write` (noter), rien pour la répartition de la charge | `030` § 3 | L'action groupée d'affectation devait choisir une permission existante | **`event.call.manage` retenue** : la composition du comité (`event.call_reviewers`) et la répartition relèvent de l'appel. Si l'IFDD veut un jour confier l'affectation SANS la décision, c'est une permission de plus dans le SQL, pas une règle dans un écran |
| **49** | **AUCUN DÉLAI D'ALERTE N'EST PORTÉ PAR LE MODÈLE, et le filtre « en retard » s'en passe.** Il lit `review_assignments.due_at`, qui existe ; c'est le tableau de bord (A6) qui avait dû inventer une fenêtre de 21 jours | `070` § 5 | Rien pour cet écran | Aucun changement. L'écart n°45 (fenêtre d'alerte en dur) reste ouvert et ne concerne que A6 |

---

## Ce qui a été vérifié le 18/08 sur la liste des propositions, et comment

Un tableau de bord de comité se prouve sur ce qu'il AFFICHE et sur ce que ses actions FONT, pas sur ce qu'il compile. Tout ce qui suit a été mesuré sur le rendu réel — serveur de développement, navigateur piloté, mesures dans le DOM — avec le compte d'une administratrice globale (Mme Bakayoko).

| Contrôle | Résultat |
|---|---|
| **Les ajouts au modèle tiennent-ils sur une base VIERGE ?** | Les 18 fichiers rechargés dans une base jetable créée à côté — `verif_a7`, supprimée ensuite : **le volume de développement n'a pas été détruit**, contrairement à ce que ferait `make check-db`. Résultat : **174 tables**, `cross_module_fk_report` **à zéro non conforme**, `v_proposal_dashboard` à **33 colonnes** avec les types attendus (`uuid[]`, `jsonb`, `text[]`, `character(2)`), `unread_proposals_for()` exécutée sans erreur. Les treize colonnes ont aussi été éprouvées sur la base de développement en place |
| **Les quarante lignes sont-elles là ?** | 40 dossiers COP31, 20 par page, deux pages. Le quarante-et-unième (COP30) **n'apparaît pas** : la liste est filtrée par édition, et le rang est désormais calculé PAR édition — le dossier de la COP30 consommait auparavant un rang dans le classement de la COP31 |
| **Le tri par défaut** | Note décroissante : 19,0 · 18,5 · 18,3 … et le rang suit (1, 2, 3). Les dossiers non notés portent « — » et non zéro, et ferment la liste dans les deux sens de tri (`NULLS LAST` de la vue, reproduit dans le tri du front) |
| **Les huit filtres et leurs décomptes** | Comptés sur le PÉRIMÈTRE, filtres non appliqués : Brouillon 5, Déposé 6, En évaluation 5, Corrections demandées 3, Retenu 16, Non retenu 3, Retiré 1, Annulé 1 — **somme 40**. Signaux : non évaluées 8, en retard 1, non consultées par moi 5 (les cinq brouillons, que personne n'ouvre) |
| **Le filtre venu du tableau de bord** | `/admin/propositions?filtre=non-evaluees` — l'URL exacte que pose la file d'actions d'A6 depuis le 17/08 : **8 lignes**, en tête `COP31-00028`. Le lien n'était pas mort, il ne menait simplement à rien avant aujourd'hui |
| **Recherche insensible aux accents** | `?q=cotiere` ramène **3 dossiers**, dont « Financer l'adaptation côtière en Afrique de l'Ouest ». C'est ainsi qu'on tape un titre dont on se souvient de loin |
| **Action groupée — affectation** | 8 dossiers non évalués confiés à Alizeta Kaboré : **« 8 dossiers affectés »**, et la colonne des revues de la première ligne passe à « 0/3 — Attendue de Alizeta Kaboré ». La charge de chacun est affichée dans la liste du dialogue (5, 7, 8, 9, 10 dossiers) |
| **Action groupée — statut, sélection hétérogène** | 20 dossiers retenus, transition « Non retenu » : **« 4 dossiers modifiés, 16 dossiers ont été écartés »**, chacun nommé avec son motif (« transition impossible depuis son statut actuel »). Les options proposées sont celles de `proposal_transitions_allowed` et portent leur portée réelle — « Retenu (3 sur 20) », « Annulé (16 sur 20) » |
| **Le motif est-il exigé quand la base l'exige ?** | « Non retenu » (`requires_reason`) : le champ apparaît, et valider sans lui affiche « Cette transition exige un motif » **sans requête partie**. Le trigger l'aurait refusée de toute façon |
| **Export CSV** | « 40 dossiers exportés » sur la liste entière, « Exporter la sélection » sur les lignes retenues. Quatorze colonnes, point-virgule, BOM |
| **L'indicateur « non consulté »** | Discret par construction : une puce cyan de 6 px et un numéro de dossier en gras, jamais une pastille d'alerte. Invisible pour Mme Bakayoko sur les dossiers déposés — elle a tout ouvert —, présent sur les brouillons |
| **Aucun défilement horizontal à 375 px** | Mesuré : `document.scrollWidth` vaut **375**. Quatre colonnes restent (case, numéro, titre, statut), le reste défile DANS le cadre du tableau |
| **Thème clair et thème sombre** | Rendus tous les deux : aucun composant de cet écran ne teste le thème, tout passe par les jetons |
| **Les états de l'écran** | Chargement (dix lignes squelettes à la forme du tableau), erreur avec reprise, **deux vides distincts** — « aucun dossier pour cette édition » et « aucun dossier ne correspond » avec le bouton qui retire les filtres —, accès refusé |
| `npm run typecheck` et `npm run build` | Les deux au vert, zéro erreur |

**Trois défauts trouvés en éprouvant l'écran, et ce qu'ils ont coûté**

| Défaut | Ce qu'on voyait | Correction |
|---|---|---|
| **`line-clamp-2` n'existe pas dans la feuille produite** | Les titres s'étalaient sur **six lignes** et la colonne tombait à 128 px de large. La classe était dans le balisage, sans aucune règle derrière — le pire des cas, puisque rien ne signale l'absence | Troncature écrite à la main dans le `<style scoped>` du composant, comme `ProgrammeCalendar` le faisait déjà. **Aucun autre écran n'utilise `line-clamp-*`** : le piège ne se reproduira pas ailleurs sans être vu |
| **Une pastille thématique insécable imposait le quart du tableau** | « Justice climatique et peuples autochtones » en `whitespace-nowrap` : colonne à 284 px, tableau à 1 501 px pour 1 130 disponibles | Repli autorisé DANS la cellule, par CSS local — sans toucher au composant partagé, et **sans jamais abréger le libellé**, que le guide de style exige complet |
| **« Aucun révisionniste affecté » sur un dossier portant trois revues** | Vrai littéralement — aucune ligne d'affectation —, absurde à l'écran : les trois revues étaient rendues | Le message ne s'affiche plus que si `review_count` vaut zéro aussi. L'affectation ORGANISE le travail, elle ne le conditionne pas |
