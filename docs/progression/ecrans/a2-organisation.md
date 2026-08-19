# A2 — Rattachement à une organisation

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 17/08. 1 page, 6 composants, 1 fichier de contrats (`types/organization-join.ts`), 1 fichier de mocks qui rejoue `find_similar_organizations()`, 4 fichiers de traduction (2 × 2 locales). **Porte aussi la règle transverse arrêtée le 17/08** : le rattachement est facultatif pour avoir un compte, mais exigé par certaines actions — d'où le middleware `requires-organization`, le store `membership` et l'étape intermédiaire. La première page gardée sera A4. **Le modèle a été corrigé d'abord** : la recherche ne trouvait pas un début de nom complet, et ne renvoyait pas de quoi reconnaître une fiche. Deux écarts reportés au prompt B2 (n° 21, 22)

---

## Écarts relevés en écrivant le rattachement à une organisation (A2, 17/08)

Deux d'entre eux étaient des **défauts du modèle** et ont été corrigés dans le SQL, base rechargée — c'est la règle du projet : on modifie le SQL d'abord, on recharge, puis on écrit le code. Les deux autres sont des obligations d'API, inscrites au prompt **B2**.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **21** | **RÉGLÉ le 17/08 — la recherche ne trouvait pas un début de nom complet.** `similarity()` compare deux chaînes ENTIÈRES : « institut » face au nom de l'IFDD donne 0,17, sous le seuil de 0,3 de `pg_trgm` | `040` § 5 | Tout. L'écran affichait « aucun résultat » et proposait de créer la fiche qui existait déjà — le défaut n° 1 de la v1, produit par le mécanisme censé l'empêcher. Le sigle fonctionnait, le nom complet non : exactement la moitié des utilisateurs de la citation du cadrage | **Corrigé dans le SQL** (voir « Modifications du modèle »). Vérifié après rechargement : « institut » → 60,0, « franco » → 51,4, « in » → 55,0, « IEPF » → 100,0, un texte sans rapport → 0 résultat |
| **22** | **RÉGLÉ le 17/08 — la fonction ne rendait pas de quoi reconnaître une fiche** : ni sceau de vérification, ni nombre de membres, ni ville, ni type | `040` § 5 | Le prompt exige ces informations sur chaque résultat, et elles ne sont pas décoratives : ce sont elles qui distinguent les deux fiches OSED l'une de l'autre. Sans elles, deux requêtes de plus **par frappe**, ou un écran qui laisse choisir à l'aveugle | **Corrigé dans le SQL.** `types/org.ts` répercuté |
| **23** | **La fonction fait remonter les fiches partageant le DOMAINE de l'adresse, quel que soit le nom cherché** — bonus de 40 points, sans condition de ressemblance | `040` § 5 | Constaté dans le navigateur : chercher « Agence spatiale du Sahel » renvoyait l'organisation du domaine de la personne, sans aucun rapport avec la question posée, alors qu'un bandeau la propose déjà nommément au-dessus du champ | **Contourné côté écran, à dessein** : la page écarte les résultats sans `name_similarity`. Le comportement de la fonction reste JUSTE pour le back-office (A11), qui cherche « tout ce qui pourrait être la même entité ». Deux usages, deux lectures — à ne pas « corriger » dans le SQL |
| **24** | **Rien ne dit quel RÔLE prend une personne qui rejoint une organisation.** `memberships.role` a pour défaut `member`, et aucune règle n'attribue `manager` | `040` § 4 | L'interface a tranché seule : qui **crée** une fiche en devient `manager` — personne d'autre ne peut l'approuver, et quelqu'un doit pouvoir accepter les adhésions suivantes —, qui **rejoint** reste `member`. C'est défendable et ce n'est écrit nulle part | **Reporté au prompt B2**, à confirmer. La question qui suit — qu'advient-il quand le référent d'une fiche part ? — appartient au même arbitrage |

---

## Ce qui a été vérifié le 17/08 sur le rattachement à une organisation, et comment

Un écran de recherche ne se prouve pas au rendu statique. Tout ce qui suit a été exercé **dans un navigateur réel**, connecté tour à tour sous deux comptes différents.

| Contrôle | Résultat |
|---|---|
| **La correction du modèle tient-elle sur une base vierge ?** | `make check-db` (`down -v` puis rechargement complet) **vert**, zéro erreur d'initialisation, frontières de modules conformes. La fonction rend bien ses quatre colonnes nouvelles |
| **Les mocks rendent-ils les MÊMES scores que PostgreSQL ?** | Comparés terme par terme sur l'IFDD, seule organisation semée : « institut » 60,0 / « IFDD » 125,0 / « IEPF » 100,0 / « franco » 51,4 / « in » 55,0 — **identiques des deux côtés**. Un écart initial sur « franco » (0 contre 51,4) a révélé que `word_similarity` ne divise pas par la réunion des trigrammes mais par ceux de la requête seule : trouvé par la comparaison, pas par relecture |
| **Le parcours d'inscription mène-t-il vraiment à cet écran ?** | Il n'y menait PAS — trouvé en jouant le parcours de bout en bout, pas en relisant le code : après vérification d'adresse on arrivait à la connexion, puis à l'accueil, sans organisation et sans que rien le dise. Branché : le bouton « Se connecter » de l'écran de vérification porte `?redirect=`, le mécanisme du middleware `auth` déjà écrit et déjà validé côté page de connexion. **Vérifié dans les deux langues** : `/connexion?redirect=%2Frattachement-organisation` et `/en/login?redirect=%2Fen%2Fjoin-organization`, connexion suivie → on atterrit sur l'écran de rattachement |
| **Le rattachement peut-il être refusé ?** | Oui, et c'est écrit : « Je n'ai pas d'organisation pour l'instant » ramène à l'accueil, avec la phrase qui dit que le compte reste pleinement utilisable et que le rattachement s'ajoutera depuis le profil. En étape imposée, le même bouton devient « Renoncer pour l'instant » — partir signifie alors renoncer à l'action, et il faut le dire |
| **La garde `requires-organization` fonctionne-t-elle ?** | Éprouvée en la posant TEMPORAIREMENT sur `/style-guide`, puis retirée — aucune page gardée n'existe encore, la première sera A4. Les trois cas : sans rattachement → renvoi vers `/rattachement-organisation?redirect=/style-guide&reason=proposal` avec le bandeau qui nomme l'action ; **demande en attente → bloquée elle aussi**, l'écran montrant « Vos rattachements — En attente » et l'explication ; rattachement obtenu → « Reprendre où j'en étais » ramène à la page, **qui s'ouvre** cette fois. Vérifié que `/style-guide` est redevenu public après retrait |
| L'écran reste-t-il atteignable ensuite ? | Entrée « Mon organisation » dans la barre publique dès qu'une personne est connectée (`nav.account.myOrganization`, clé qui existait déjà), sur écran large comme dans le menu mobile. Elle disparaîtra avec la vraie page de profil (A5) |
| **Le doublon volontaire remonte-t-il ?** | « OSED » ramène **les deux fiches**, 125,0 et 100,0, toutes deux marquées « C'est probablement la vôtre », distinguables par le sceau, le type, la ville et le nombre de membres. « observatoire du sahel » ramène les deux également |
| Recherche par le début du nom complet | « institut » ramène l'IFDD et l'IMRE. C'est le cas qui ne fonctionnait pas avant la correction du modèle |
| Le bouton de création n'apparaît **qu'après** une recherche | Vérifié : rien avant la première frappe ; état vide + bouton **secondaire** quand la recherche ne ramène rien ; lien discret « Aucune de ces organisations n'est la vôtre ? » sous une liste de résultats |
| Rattachement automatique par domaine | Connecté sous `fatoumata.sy@ujfc.org` : bandeau « Le domaine @ujfc.org appartient à Union des jeunes francophones pour le climat », rattachement **immédiat** annoncé puis effectué → « Vous êtes rattaché à… ». Sous `karim.ilboudo@example.org`, aucun bandeau — le domaine ne prouve rien |
| Demande soumise à approbation | Rejoindre le ROAC depuis le même compte → « Votre demande sera transmise à un référent », puis « Demande envoyée à… ». Les deux libellés du bouton diffèrent selon l'issue attendue |
| Adhésion déjà en attente | Karim Ilboudo voit « Vos rattachements — UJFC, En attente » et la phrase « Inutile de la renouveler » |
| **L'avertissement avant création** | Saisir « OSED » dans le formulaire fait apparaître l'alerte **pendant la frappe** ; à l'envoi, comparaison ligne à ligne, valeurs identiques surlignées, trois issues — rejoindre (primaire), créer quand même (secondaire), modifier sa saisie (discret). **« Créer quand même » aboutit vraiment** : la fiche est créée |
| Doublon EXACT, celui que la base refuse | Créer « Réseau ouest-africain pour l'adaptation côtière » au Sénégal → « Une organisation porte déjà ce nom dans ce pays. La voici », avec la fiche et de quoi la rejoindre. Une information, pas un mur |
| Validation du formulaire | Nom vide → « Saisissez le nom complet » ; sigle de plus de 32 signes → « Un sigle compte entre 2 et 32 caractères » (le `ck_` du § 1) ; type et pays obligatoires |
| 375 px, thème sombre, anglais | `scrollWidth == clientWidth == 375`, fond `rgb(35,31,32)` — le noir de charte —, page entièrement traduite. **Zéro clé brute** (balayage sur `organization.join.*`, `common.*`, `validation.*`) |
| Non-régression du guide de style | `/style-guide` répond **200**, ses modales restent fermées au montage et s'ouvrent au clic, « facultatif » s'affiche toujours sur les 14 champs de formulaire ordinaires, zéro clé brute |
| `make check-front` | Vert — `nuxt typecheck` à 0 erreur, construction complète |
| Aucun fichier de code applicatif > 1000 lignes | Le plus long des fichiers créés est `app/pages/organization/join.vue` (630 lignes) |

**Un manque de chaînage, trouvé en jouant le parcours plutôt qu'en relisant le code.** Le prompt A2 dit « présenté après la création de compte et accessible depuis le profil » : l'écran existait, complet et vérifié, mais **rien n'y menait**. Deux liens l'ont réglé — le bouton de l'écran de vérification d'adresse, et une entrée « Mon organisation » dans la barre publique. La leçon vaut pour les prompts suivants : un écran qui répond à toutes ses exigences internes peut rester inatteignable, et cela ne se voit qu'en refaisant le chemin depuis le début.

**Cinq défauts trouvés en chemin, corrigés là où ils étaient** — aucun n'appartenait à cet écran, tous se seraient payés ailleurs :

| Défaut | Où | Correction |
|---|---|---|
| **`I18nText` promettait une valeur pour TOUTE locale** (`[locale: string]: string`), alors que seul le français est obligatoire en base. Conséquence : une donnée n'ayant qu'un français ne s'assignait plus à son propre type dès qu'on l'annotait — l'inférence produit `{ fr: string; en?: undefined }` | `types/shared.ts` | Index passé à `string \| undefined`. `resolveI18nText()` traitait l'absence depuis toujours (`isFilled`) : c'est le type qui promettait plus que la donnée |
| **`UiModal` montée DÉJÀ ouverte ne s'ouvrait jamais** : le watcher n'avait rien vu changer. Motif naturel dès qu'un dialogue porte sur un élément choisi dans une liste ; invisible dans le guide de style, où toutes les modales sont montées fermées | `components/ui/Modal.vue` | Synchronisation extraite et appelée aussi `onMounted` |
| **« facultatif » sur une barre de recherche** — « Nom ou sigle de l'organisation facultatif ». Un champ de recherche n'appartient à aucune soumission : il n'est ni obligatoire ni facultatif | `Ui FormField`, `UiSearchInput` | Propriété `hideOptional`, posée par `UiSearchInput`. Même famille que la correction d'A1 sur les champs en lecture seule |
| **Le layout public affichait « Se connecter » à une personne connectée** — sur le premier écran du jalon qui exige de l'être | `layouts/public.vue` | La barre connaît la session : nom de la personne et bouton de déconnexion. Chargement non bloquant, pour ne pas retarder les pages publiques |
| **`ACCOUNT(1)` valait exactement `DUPLICATE.osed`** : les deux familles d'identifiants simulés partageaient le préfixe `7007` | `mocks/ids.ts` | Comptes déplacés en `7009`. Aucune donnée n'en était fausse, mais le fichier promet qu'un identifiant croisé dans une console se retrouve par simple recherche |
