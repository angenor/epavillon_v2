# Pièges rencontrés et ce qu'ils ont coûté

> Extrait de la [progression](../PROGRESSION.md). Transverse aux écrans : à relire avant d'en commencer un.

| Symptôme | Cause | Correction |
|---|---|---|
| **`nuxt typecheck` ne vérifiait plus rien depuis un moment, sans que rien ne le dise** (20/08) | `vue-router` était épinglé `^4.5.0` dans `frontend/package.json`, alors que Nuxt 4.5.2 dépend de `^5.2.0`. La dépendance explicite gagne à la résolution, et Nuxt pousse inconditionnellement le greffon `vue-router/volar/sfc-route-blocks` dans le `tsconfig` qu'il génère — un sous-chemin que la branche 4 n'exporte pas. `vue-tsc` échouait **à la lecture de sa configuration**, donc AVANT d'avoir vérifié le moindre type, et sortait en erreur sans jamais nommer un fichier du projet | Aligné sur `^5.2.0`. **Le vrai enseignement n'est pas la version** : une commande de vérification qui échoue toujours de la même façon finit par se lire comme du bruit. Un `make check` qui ne passe pas depuis longtemps ne protège plus de rien |
| **Une décision remise à nul dans les mocks retombait sur celle du jeu de données** (20/08) | `settledDecisionOf(id)?.reviewed_at ?? candidate.reviewed_at` : `??` ne distingue pas « la session n'a rien écrit » de « la session a écrit un nul ». Remettre une paire de doublons dans la file écrit précisément des nuls | Ternaire sur l'EXISTENCE de l'entrée, pas sur la valeur du champ. Le piège attend tout mock qui simule un effacement plutôt qu'une écriture |
| **Un changement de thème laissait les graphiques d'un thème en retard** : encre claire sur fond blanc, formats d'axe perdus (« 0.00, 1.00 ») (18/08) | Deux causes superposées. La palette était relue en réaction au magasin de préférences, donc AVANT que `app.vue` n'ait posé `data-theme` sur le `<html>` ; et la fusion d'options d'ApexCharts **perd les fonctions**, donc les formats d'axe passés à la mise à jour | `MutationObserver` sur l'attribut de thème — il se déclenche après la mutation, jamais avant. Et le tracé est **remonté** par une clé qui porte le thème et la langue, plutôt que mis à jour |
| **ApexCharts était téléchargé sur la page publique d'un événement**, qui n'affiche aucun graphique (18/08) | Un greffon `.client` appartient au paquet d'ENTRÉE : `app.use(VueApexCharts)` depuis un import de tête embarque les ~500 ko partout | `defineAsyncComponent(() => import('vue3-apexcharts'))` — la bibliothèque ne part qu'au premier `<apexchart>` rendu. Vérifié dans une session de navigateur neuve |
| **« griculture et alimentation »** : le libellé d'axe le plus long coupé au commencement (18/08) | La bibliothèque réserve la largeur du texte TRONQUÉ, puis lui ajoute ses points de suspension au moment de l'écrire : le rendu dépasse toujours la réserve | Plafond de largeur abaissé pour qu'elle tronque elle-même, et 12 px de marge à gauche pour rendre le dépassement inoffensif |
| **Un axe de comptes graduait en « 1, 2, 2 »** et perdait sa graduation la plus haute (18/08) | `forceNiceScale` découpe un maximum de 3 en 0,75 — 1,5 — 2,25 ; notre format, qui n'affiche pas de décimale, arrondissait deux graduations au même nombre | Pas calculé pour tomber sur des entiers, avec une graduation de respiration au-dessus du pic |
| **La grille du planificateur ne bougeait pas** : aucun bloc déplaçable, aucune poignée active, et pas le moindre message (18/08) | vue-cal charge son module de glisser-déposer dans `created()`, et **seulement si l'édition est déjà ouverte**. Or le droit d'arbitrer et la détection du pointeur fin arrivent APRÈS le premier rendu : le composant naissait en lecture seule et le restait. Trouvé en inspectant `modules.dnd` dans l'instance, pas en lisant le code | `editable` fait partie de la CLÉ du composant, ce qui force un remontage quand le droit apparaît. Règle générale : une propriété qui commande une CAPACITÉ, et non un simple affichage, appartient à la clé |
| **Le bouton « Enregistrer » d'un panneau était actif à l'ouverture**, sans qu'on ait rien touché (18/08) | `2027-11-12T14:00:00-03:00` et `2027-11-12T17:00:00.000Z` désignent la même seconde et s'écrivent différemment : la détection de changement comparait des CHAÎNES | Comparaison par valeur (`Date.parse`). Le piège revient partout où le front reconstruit une date avant de la comparer à celle du serveur |
| **Le récapitulatif de publication affichait « A ↔ B (["2027-11-12T14:00:00-03:00",…) »** au milieu d'une phrase française (18/08) | `publication_readiness()` collait le `tstzrange` du chevauchement dans sa colonne `detail`. La valeur était juste, lisible par personne, intraduisible et impossible à situer dans un fuseau | La fonction rend `occurs_at`, un instant ; l'interface le met en mots. La base rend des DONNÉES, l'interface les rédige — sans quoi la règle « toute date affichée porte son fuseau » ne peut pas s'appliquer |
| **Les cinq pages d'authentification répondent 500**, toutes, dès le premier chargement — « Must be called at the top of a `setup` function ». Les autres pages fonctionnent | `useApi()` appelait `useI18n()`, qui exige un contexte de `setup`. Or le nouveau store de session appelle `useApi()` **depuis un middleware de navigation**, donc hors de tout composant. Aucun écran ne l'avait fait avant : le guide de style et la page d'accueil consomment `useApi()` depuis un `setup` | `useApi()` lit désormais la locale sur `useNuxtApp().$i18n`, disponible partout où le contexte Nuxt l'est. Le motif vaudra pour tous les écrans à middleware — A2, A5 et le back-office |
| **500 sur la seule page de connexion** : « Message compilation error: Invalid linked format » | Le `@` d'une adresse donnée en exemple (`prenom.nom@organisation.org`) est le marqueur de **message lié** de vue-i18n (`@:clé`). Une adresse en dur dans un fichier de traduction ne se compile donc pas | Échappé en `{'@'}` dans les deux locales. Le piège attend tout futur fichier de traduction contenant une adresse électronique |
| La page se rend **avec ses clés brutes** (« nav.site.name ») en production, alors que le mode développement est correct | `import.meta.glob` dans un fichier de locale : Nitro compile ces fichiers hors du pipeline Vite et remplace `import.meta` par `globalThis._importMeta_`, qui n'a pas de `glob`. Aucun avertissement à la construction, et les alertes de clé manquante de vue-i18n sont muettes en production | `modules/i18n-messages.ts` génère des imports statiques. Le piège est raconté dans son en-tête pour qu'il ne soit pas réintroduit |
| Deuxième tentative : template dans `.nuxt/` importé par `#build/…` | « Vue app aliases are not allowed in server runtime » — Nitro refuse les alias applicatifs | Fichier généré dans `i18n/locales/.generated/`, importé par chemin relatif. Ni alias, ni chemin absolu, ni `import.meta` |
| Le module ne voyait pas l'ajout d'un fichier de traduction | Le hook `builder:watch` de Nuxt ne surveille que `app/` et quelques fichiers de la racine, jamais `i18n/` | Observateur chokidar dédié, actif en développement seulement. **Vérifié** : ajout puis suppression d'un fichier régénèrent l'agrégation sans redémarrage |
| `npm install` échoue sur un conflit de pairs | `esbuild@0.25` hissé par `unplugin` contre `esbuild ^0.27 \|\| ^0.28` exigé par Vite 8 | `overrides: { "esbuild": "^0.28.0" }` dans `frontend/package.json` |
| `nuxt typecheck` : `node:fs`, `process` introuvables | `@types/node` absent | Ajouté en dépendance de développement |
| `lazy: true` refusé par la configuration i18n | L'option a disparu en @nuxtjs/i18n v10 : le chargement à la demande est le comportement par défaut dès qu'une locale a un `file` | Option retirée ; le découpage par locale est vérifié dans la sortie de construction (un fragment par langue) |

## Socle et identité (B1, 20/08)

Les trois premiers sont les pièges du module `identity` : aucun ne casse quoi que ce soit, aucun ne produit d'erreur, et c'est précisément ce qui les rend chers. Un défaut qui échoue se corrige ; un défaut qui réussit se découvre six mois plus tard.

| Symptôme | Cause | Correction |
|---|---|---|
| **Deux lignes `identity.person.anonymized` pour un seul effacement** — sans erreur, sans avertissement (20/08) | `identity.anonymize_person()` appelle **elle-même** `platform.emit_event()`. Le service qui l'invoque croit devoir émettre l'événement, comme il le fait pour toutes ses autres écritures. Rien ne proteste : l'outbox accepte les deux, et un consommateur idempotent traite la première puis ignore la mauvaise. Le défaut ne se voit qu'en relisant l'outbox d'un agrégat qui aurait deux fois la même histoire | `service/privacy.rs` n'émet rien après l'appel, et **dit pourquoi** à l'endroit où l'on serait tenté d'ajouter la ligne. `effacement.rs` compte les événements. La règle générale : avant d'émettre après un appel de fonction SQL, lire la fonction |
| **Une trace d'audit anonyme**, pour une écriture qui avait pourtant un auteur (20/08) | Une écriture sans `app.actor_id` **n'échoue pas** : elle écrit `actor_id = NULL` et rien ne le signale. Le cas piège est la réinitialisation de mot de passe — la personne n'a pas de session, et son identifiant sort du jeton consommé, donc de l'intérieur de la transaction déjà ouverte | `kernel::db::write()` est la seule porte d'écriture et pose le contexte elle-même ; `kernel::db::set_actor()` le repose en cours de transaction quand l'acteur ne se connaît qu'après la première lecture. `toute_ecriture_laisse_son_auteur.rs` joue le cycle complet et échoue s'il reste une ligne sans acteur. **Une seule trace anonyme est légitime** : l'inscription de soi-même, où la personne n'a pas encore d'identifiant |
| **Le formulaire de connexion redevenait l'annuaire des comptes** — même message, mais dix fois plus vite sur une adresse inconnue (20/08) | Ne hacher le mot de passe que lorsque l'adresse existe. Le message ne dit rien, le **temps** dit tout : Argon2id coûte des dizaines de millisecondes, une adresse absente n'en coûte aucune | Une empreinte factice est calculée sur adresse inconnue, avec les mêmes paramètres. `discretion_temps_de_reponse.rs` mesure cent tentatives de chaque sorte et exige moins de 10 % d'écart entre les médianes. Le piège vaut pour toute réponse invariable — inscription, renvoi de lien, réinitialisation |
| **Trois exemplaires du même courriel** partis pour une seule inscription (20/08) | Le relais du site retenait l'identifiant du message **après** l'envoi. Or le doublon réel est concurrent : le client de l'API abandonne au bout de quinze secondes et réessaie deux secondes plus tard, pendant que le premier envoi est encore en cours. La garde ne protégeait que d'un doublon séquentiel, qui n'arrive jamais | L'identifiant est **réservé avant** l'envoi, et rendu si l'envoi échoue — perdre un message vaut pire que le tenter deux fois. Node exécute ce test-et-pose sans interruption : la réservation est atomique |
| **`platform.audit_log.actor_label` restait vide**, alors que la colonne existe pour « rester lisible après anonymisation RGPD » (20/08) | `platform.tg_audit()` ne la renseignait pas. Sans elle, le nom de l'auteur se lit par jointure — et une personne qui exerce son droit à l'effacement fait devenir « Utilisateur anonymisé » toutes ses décisions passées | Le trigger lit `display_name` à l'écriture, seul instant où le nom existe encore. Une lecture par clé primaire sur une écriture déjà auditée |
| **`cargo build` échouait hors ligne** après l'ajout de requêtes dans les tests (20/08) | `cargo sqlx prepare --workspace` sans `--all-targets` n'enregistre pas les requêtes des tests d'intégration : `.sqlx/` paraît complet et ne l'est pas | `cargo sqlx prepare --workspace -- --all-targets --all-features`, toujours |
| **`make check` échouait sur la mise en forme**, deux fois (20/08) | `cargo fmt --all --check` est la première commande de `check-back` : un fichier écrit à la main échoue avant que quoi que ce soit ne compile | `cargo fmt --all` avant tout `make check`. Ce n'est pas une option de confort, c'est un portail |

## Organisations (B2, 20/08)

Trois pièges du modèle, et trois de l'implémentation. Les trois premiers ont été **prévus** par le plan et n'ont donc rien cassé — c'est tout l'intérêt de les avoir cherchés avant. Les trois suivants n'ont été trouvés qu'en faisant tourner le code.

| Symptôme | Cause | Correction |
|---|---|---|
| **La fusion écrirait deux fois `org.organization.merged`, et marquerait deux fois la paire** — sans erreur, sans avertissement | `org.merge_organizations()` appelle **elle-même** `platform.emit_event()` et met **elle-même** la paire à « fusionnée » avant de rendre la main. C'est le piège n° 1 d'`identity` — `anonymize_person()` — répété à l'identique | `service/merge.rs` n'émet rien et ne marque rien, et **le dit à l'endroit où l'on serait tenté d'ajouter la ligne**. `outbox_une_seule_fusion.rs` **compte** les événements. Règle générale, apprise deux fois : avant d'émettre après un appel de fonction SQL, **lire la fonction** |
| **Une adhésion refusée ne pouvait plus jamais être redemandée** | `ux_memberships (organization_id, person_id)` **ne connaît pas le statut** : une ligne révoquée occupe la place. Une lecture suivie d'une écriture perd aussi la course entre deux demandes simultanées | Un **unique ordre** `INSERT … ON CONFLICT DO UPDATE … WHERE status = 'revoked'` : la base tranche. `adhesion_revoquee_puis_redemandee.rs` joue trois allers-retours et cent demandes concurrentes |
| **Toute fusion arbitrant le nom légal aurait échoué** | `ux_organizations_name_country` ne porte que sur les fiches **vivantes** : tant que la fiche absorbée l'est, la survivante ne peut pas reprendre son nom. Or le nom légal est le champ le plus souvent arbitré, et `docs/progression/api.md` comme l'en-tête du type du front disaient « **avant** l'appel » | Les arbitrages viennent **après** `org.merge_organizations()`, dans la même transaction. La garantie recherchée est intacte — si l'arbitrage échoue, la fusion est annulée avec lui. Les deux documents sont corrigés |
| **Cent créations concurrentes du même nom sortaient en « service indisponible »**, pas en refus (20/08) | La transaction perdante lisait la fiche en conflit **avant** d'être rendue : elle retenait donc **deux** connexions du pool à la fois. Avec cinq connexions et cinq transactions en vol, le pool s'épuise et les dernières expirent | La transaction est **rendue avant** la lecture du refus. Le même piège s'est reproduit à la vérification d'un domaine, où la lecture suivait une violation de contrainte : une transaction abandonnée refuse toute commande suivante — « current transaction is aborted » — et l'on rendait une erreur interne à la place du refus attendu |
| **La liste du back-office était vide sur une base neuve**, sans que rien ne le dise (20/08) | Elle joignait `analytics.mv_organization_scorecard` **par l'intérieur**. La projection n'est rafraîchie que par un travail différé : une fiche créée il y a dix secondes n'y figure pas — c'est-à-dire exactement celle que l'équipe vient traiter | Jointure **par la gauche**, à partir de la table vivante. Les compteurs manquants valent zéro, ce qui est vrai : une fiche que rien ne référence n'a rien à compter |
| **Une faute d'orthographe connue se faisait passer pour une dénomination posée par la base** (20/08) | `is_derived` comparait le seul nom normalisé. Or « Institut … pour le **Developpement** Durable », enregistré comme faute de frappe, se normalise **exactement** comme le nom légal — les accents disparaissent. L'API refusait donc de retirer une ligne que rien ne régénère | La comparaison porte sur le nom normalisé **et le genre** : `legal` avec `legal`, `acronym` avec `acronym` |

| **Trois routes sur vingt et une étaient muettes**, et rien ne le signalait (20/08) | Deux `web::scope("/organizations")` enregistrés séparément — l'un pour les lectures ouvertes, l'autre pour les adhésions. **Actix retient le premier scope dont le préfixe correspond et rend 404 si la route n'y figure pas** : il n'essaie pas le suivant. Ni la compilation, ni les tests des services — qui les appellent directement —, ni la documentation OpenAPI — qui décrit ce qu'on **annote**, pas ce qui est **monté** — ne pouvaient le voir. Le même piège s'est révélé deux fois de plus dans la même heure : sur `/admin/organizations`, puis sur `/people`, que le module `identity` monte déjà | **Un seul `web::scope` par préfixe**, composé dans `lib.rs` et dans l'API pour `/people`. Et surtout `crates/api/tests/routes_org.rs`, qui frappe **les vingt et une routes sur la vraie application** et n'admet aucun 404. C'est la leçon de B1 répétée : **un test qui appelle un service n'éprouve pas le montage** |
| **`POST /organizations/{id}/members` refusait un corps sans `organization_id`** — un champ que le chemin porte déjà (20/08) | La charge utile reprenait le contrat du front, où le champ existe parce que les données simulées n'ont pas de chemin. Le client devait donc l'envoyer **en double**, sous peine d'un « le champ organization_id est absent ou mal formé » qui ne désigne rien de ce qu'il a fait | Le champ devient facultatif dans le corps et **le chemin fait foi**. Trouvé en frappant la route pour de vrai, jamais par un test de service |

**Un écart mesuré, et assumé** : le décompte de transfert d'une fusion est exact au chiffre près sur **dix-sept lignes du registre sur dix-huit**. La dernière — `identity.people.primary_organization_id` — est déplacée par `tg_memberships_sync_primary` **avant** que la boucle du registre n'y arrive, si bien que le journal en compte moins que l'aperçu n'en annonçait. Les lignes ont bien été déplacées, simplement pas par l'ordre qui les comptait. Le corriger demanderait de reproduire l'effet d'un trigger dans un décompte, ce que le principe VIII interdit. Consigné dans l'en-tête de `repo/merge_counts.rs`.


---

## Le raccordement à l'API (B7, 22/08)

**Une instance de classe ne se met JAMAIS dans l'état d'un store.**
Le payload de Nuxt est composé par `devalue`, qui refuse tout ce qui n'est pas un objet simple. Une
erreur — `ApiUnreachableError`, `FetchError`, n'importe laquelle — posée dans un `ref` de store fait
échouer la sérialisation de la page **entière** : le visiteur reçoit un **500**. Le piège est vicieux
parce qu'il ne se déclenche **qu'en cas de panne d'API**, c'est-à-dire au seul moment où ce chemin
sert : tous les écrans passent en développement, et l'écran dégradé qu'on a pris soin d'écrire ne
s'affiche jamais. Retenir un objet plat — `LoadFailure` : message, code, référence d'incident.

**Le site et l'API doivent partager le même HÔTE, pas seulement le même site.**
La portée d'un cookie ignore le port mais pas l'hôte : un site sur `localhost:3000` qui appelle
`127.0.0.1:8080` n'enverra jamais sa session. Rien ne le signale — les appels aboutissent, et tout
ressort simplement déconnecté.

**Le site doit être ouvert sur l'adresse EXACTE d'`APP_PUBLIC_URL`.**
C'est la seule origine que l'API autorise. Ouvrir le site ailleurs fait refuser toutes les écritures,
et le refus arrive **sans en-têtes CORS** : le navigateur le masque au code du site, qui affiche une
panne réseau au lieu du message français que l'API a composé.

**Le rendu serveur ne peut pas renouveler un jeton, et ce n'est pas un manque.**
Le cookie de rafraîchissement est limité au chemin `/api/auth` : il n'atteint jamais le serveur Nuxt.
C'est ce qui borne le dégât d'une fuite. Toute logique de rotation écrite pour le rendu serveur est du
code mort — la rotation est un geste du navigateur, exclusivement.

**Un bandeau posé dans la mise en page ne voit pas ce que la page charge.**
Vue rend l'arbre en une passe : un composant placé au-dessus du contenu est évalué **avant** que la
page n'ait chargé quoi que ce soit. L'information arrive à l'hydratation, pas au rendu serveur. À
savoir avant d'y passer une demi-heure.

**Deux `operation_id` égaux produisent un TypeScript qui ne compile pas.**
Et le message ne nomme ni la route ni le module : il dit « Duplicate identifier » sur un fichier
engendré de onze mille lignes. Le binaire d'export refuse maintenant le document.

**`#[serde(flatten)]` sur une struct qui partage un nom de champ émet la clé DEUX FOIS.**
Le JSON reste lisible — la dernière valeur écrite l'emporte — donc cela « marche », par accident. Un
changement d'ordre de champs suffit à inverser le résultat, en silence.

---

## Le registre de modules du générateur d'OpenAPI est écrit à la main (27/08)

`cargo run -p api --bin openapi` **ne consulte pas la base** — c'est voulu : engendrer le client depuis
une base de développement retirerait du contrat les chemins d'un module éteint ce jour-là. Il se sert
donc de `ModuleRegistry::complet()`, une liste **écrite à la main** dans `crates/api/src/modules.rs`.

**Un module créé sans l'y inscrire ne figure pas au contrat**, et le symptôme accuse le mauvais côté :
`check-api-contract` reproche au site d'appeler des chemins absents du contrat, alors que le défaut est
dans le générateur. Constaté en livrant B9, sur les neuf routes des deux nouveaux crates.

C'est le **seul endroit du dépôt** où la liste des modules est écrite à la main : partout ailleurs elle
vient de `platform.modules`.

---

## Une écriture concurrente sur une fonction qui lève invalide la transaction (27/08)

`live.unpublish_incident()` lève `no_data_found` sur un message jamais publié, et le service traduit
cette levée en issue de contrat plutôt que de rejouer la condition en amont — la règle vit à un seul
endroit.

**Mais la transaction est alors invalidée**, et tout ce qu'on y tenterait ensuite échouerait sur
« current transaction is aborted ». Elle est donc abandonnée avant de composer la réponse. C'est le
même piège que partout où l'on traduit une levée de fonction : le refus se lit **après** avoir renoncé
à la transaction, jamais dedans.
