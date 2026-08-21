# Quickstart — Propositions (B4)

**Fonctionnalité** : [spec.md](spec.md) · **Plan** : [plan.md](plan.md) · **Routes** : [contracts/routes.md](contracts/routes.md)

Comment lancer ce module, l'éprouver à la main, et savoir qu'il tient. Ce fichier ne contient aucun code d'implémentation : c'est un guide de mise en route et de vérification.

---

## Préalables

```bash
cp .env.example .env                                   # si ce n'est pas déjà fait
docker compose -f ops/docker-compose.dev.yml up -d     # Postgres, Valkey, Mailpit, Jaeger, Garage
make garage-init
```

**Le schéma n'est chargé qu'au premier démarrage du conteneur.** Il n'a pas bougé depuis B3 : **B4 ne modifie aucun fichier de `docs/database/`**. Si la base a été rechargée entre-temps, rien à refaire.

`DATABASE_URL` doit être renseignée et la base démarrée pour **compiler** : SQLx vérifie les requêtes à la compilation, un nom de colonne inventé échoue au `cargo build` (principe VI).

---

## Lancer

```bash
cd backend && cargo run -p api        # l'API, http://localhost:8080
cd backend && cargo run -p worker     # le worker — inchangé par B4, mais il doit tourner
```

`GET /api/docs` rend l'OpenAPI engendrée, fermée en production. Les 37 routes de ce module y figurent avec leurs codes d'erreur.

**Le worker n'est pas modifié** (contrats/events.md) : ce module ne déclare aucun travail différé. C'est un fait à vérifier, pas un oubli — le test de montage compte les gestionnaires.

---

## Éprouver les parcours à la main

### D'abord, de quoi parler — et le semis ne donne aucun dossier

`900_seed.sql` ne pose rien dans `programme` hormis un drapeau de fonctionnalité. Il faut donc, dans l'ordre : une **édition** avec son fuseau (B3), un **appel ouvert** avec sa grille par défaut (B3), une **organisation vérifiée** (B2), une **personne** membre active de cette organisation (B1 et B2). Les quatre routes existent depuis les modules précédents ; les enchaîner est le premier test de bout en bout du jalon.

### Le brouillon, son numéro, et l'adresse d'URL que personne n'envoie

Enregistrer un dossier **titre vide**. Il doit être créé — c'est le premier enregistrement automatique, celui de la première frappe — et rendre son numéro (« COP31-00001 »). Vérifier en base que `slug` n'est pas nul.

Puis enregistrer un second dossier portant **exactement le même titre** que le premier, dans la même édition. Il doit être accepté (écart n° 95, R5) : le service suffixe. Si le second échoue, la dérivation ne réessaie pas.

**Le numéro ne doit jamais changer** : le relever au premier enregistrement, le comparer à celui de la confirmation de dépôt.

### Le créneau, et les trois heures qui disparaissent

Saisir un créneau au 12 novembre à **14:30** sur une édition dont le fuseau est celui de Belém. Rouvrir le dossier par la route de recomposition **depuis un autre fuseau**. Il doit rendre **14:30** (R6). Un écart de trois heures ici est le défaut le plus discret du module : rien ne le signale.

### Le dépôt, et ses deux refus qui portent des valeurs

Déposer sur un appel ouvert : la réponse porte le nombre de revues attendues et la date d'annonce des résultats, **lus sur l'appel**.

Fermer l'appel en base, redéposer : la réponse est **200** avec `call_closed` et **l'échéance**. Poser un plafond d'un dossier, en déposer un second : **200** avec `quota_reached` et **le plafond**. Si l'un des deux sort en 500 ou sans sa valeur, le classement préalable (R9) n'a pas eu lieu.

### La machine à états, vue par trois personnes différentes

Sur un même dossier en évaluation, demander les transitions offertes successivement comme **déposant**, comme membre du comité détenant le droit de noter, et comme administrateur détenant le droit de décider. Trois réponses différentes, chacune conforme à la table des règles :

- le déposant : « retirer », **motif exigé** ;
- le noteur : « demander des corrections », **motif exigé** ;
- le décideur : « retenir » sans motif, « rejeter » avec motif.

Puis tenter une transition **non offerte**. Le refus doit être un **200** avec `transition_not_allowed`, et le message français doit être **celui du déclencheur**, repris mot pour mot.

Enfin, tenter une transition à motif **sans motif** : **200** avec `reason_required`, et un code distinct du précédent (R8).

### Le retrait, et le motif qui écrase la décision

Retirer un dossier en évaluation avec le motif « erreur de dépôt ». Relire la colonne `decision_reason` de la ligne : elle porte ce texte. Relire le **journal** : chaque transition y garde son propre motif (écart n° 97). C'est le journal qu'un écran doit lire — et le vérifier ici évite qu'un écran affiche « motif de la décision : erreur de dépôt » sur un dossier remis en course.

### Le voile de l'aveugle, et ce qui ne part pas

Sur un appel en aveugle, avec deux revues déjà déposées par des pairs : ouvrir la fiche comme membre affecté **n'ayant pas noté**. Inspecter la **charge utile entière** — pas l'écran : elle ne doit contenir aucune note, aucune recommandation, aucun nom de pair. Le nombre de revues masquées, lui, est là.

Déposer sa propre revue, rouvrir : les revues des pairs apparaissent.

Ouvrir la même fiche comme administrateur **qui décide sans noter** : il les reçoit. Masquer les notes à qui doit trancher rendrait la décision impossible.

### La consolidation, et le classement qui ne bougeait pas

Déposer une revue avec ses notes par critère. La réponse rend la moyenne, la moyenne pondérée, le nombre de revues et l'élimination. **Relire ces quatre valeurs en base immédiatement après** : elles doivent être identiques (R10, écart n° 98). Si elles ne le sont pas, la consolidation n'a pas été appelée — et rien d'autre ne le dirait.

Poser un zéro sur un critère éliminatoire, déposer : le dossier est marqué éliminé.

Poser une note supérieure au maximum d'un critère : refus **nommant le critère et sa borne**.

### Ce que l'organisation ne voit pas

Charger l'espace d'une organisation ayant deux dossiers dont un retenu et noté. **Balayer la réponse entière** à la recherche de : une note, un rang, un nom de membre du comité, un nom d'inscrit. Aucun ne doit s'y trouver (FR-077, SC-018).

Écrire trois messages sur son dossier — un « comité », un « privé », un « partagé ». Recharger côté organisation : **un seul** message. Le filtre est à la source, pas dans l'écran.

### La demande de correction, et le compteur honnête

Écrire une demande de correction **en visibilité « comité »**. Elle doit ressortir **partagée** (écart n° 99) : le compteur « 1 point à corriger » que le déposant voit désigne alors un message qu'il peut lire.

La marquer résolue **comme déposant**, vérifier que le compteur retombe à zéro ; la retirer, vérifier qu'il remonte.

### Le renvoi après clôture

Passer un dossier en « corrections demandées », **fermer l'appel**, puis renvoyer par la route de renvoi. Il doit aboutir (écart n° 38). Le tenter par la route de **dépôt** : il doit être refusé pour appel clos. Les confondre réintroduirait par le contrat le contrôle qu'on vient de retirer du déclencheur.

### La séance qui ne bouge pas

Sur un dossier retenu ayant une séance programmée (B5 ne l'a pas encore livrée : la poser à la main en base), corriger le titre et le créneau souhaité. Relire la séance : **créneau, salle, inscrits et rappels inchangés** (SC-024). C'est la décision structurante n° 1 du modèle, et le seul moyen de la vérifier est de la vérifier.

### Le périmètre, et l'URL forgée

Se connecter comme administrateur détaché sur une seule édition. Demander la liste d'une **autre** édition, puis la fiche, l'historique, les pièces, le journal et les commentaires d'un dossier de cette autre édition en **forgeant l'identifiant**. Chaque route refuse, et le refus **ne se distingue pas** de celui d'un identifiant inexistant.

Recommencer avec un compte **sans aucun** droit d'administration : **refus explicite**, jamais une liste vide.

### La déduction des transitions v1

Créer un dossier, **vider son journal en base**, poser une date de décision. Charger la frise côté organisation : l'étape d'évaluation ment. Lancer l'opération de déduction : trois lignes semées, la frise redevient exacte. **La relancer** : zéro ligne de plus (R20).

Vérifier enfin que la déduction **n'a émis aucun événement** — c'est ce qui évite qu'une reprise déclenche huit mille courriels de décision.

---

## Les tests

```bash
cd backend
cargo test -p programme              # les tests du module
cargo test --workspace               # tout, y compris les modules précédents
```

Base **réelle et jetable**, harnais `kernel::testing` de B1. **Aucun mock de base** (principe X).

### Les quatre obligations de la constitution, et les tests qui les tiennent

| Obligation | Test |
|---|---|
| Le chemin nominal de **chaque** route | un test qui frappe les **37 routes** sur la vraie application — leçon de B2, où trois routes sur vingt et une étaient muettes, et de R18, où deux préfixes sont partagés |
| Un refus par périmètre, **URL forgée comprise** | six identifiants forgés sur six routes de dossier, même refus qu'un inexistant |
| La traduction d'au moins un invariant de la base | la transition interdite, le motif manquant, et la note au-dessus du plafond de son critère |
| Les événements attendus dans l'outbox | **une** ligne par transition, pas deux (le déclencheur émet déjà) ; les trois événements du service ; et **zéro** après une déduction v1 |

### Les vérifications propres à ce module

| Ce qui est vérifié | Pourquoi ce test-là |
|---|---|
| Le voile, par **inspection de la charge utile** | un test qui lit l'écran ne prouve rien : ce qui compte est ce qui sort de l'API |
| L'espace organisation, par **balayage de la charge utile** | même raison, et c'est la seule façon de tenir « ni note, ni rang, ni nom » |
| Le premier enregistrement à titre vide | il échouerait sans la dérivation d'adresse (écart n° 95) |
| Deux dossiers homonymes dans une édition | il échouerait sans le suffixe |
| La consolidation, par **relecture en base** | la valeur rendue et la valeur stockée doivent coïncider |
| Le renvoi sur appel clos | l'écart n° 38, celui qui a déjà bloqué un dossier |
| L'heure murale, aller-retour | l'écart de trois heures que rien ne signale |
| Le triplet d'entité des thématiques | un client qui l'enverrait ne doit pas le voir honoré |
| La purge à l'effacement | aucune contrainte ne la fait |
| La création en brouillon malgré un état demandé | l'insertion échappe au garde (écart n° 96) |
| Noter sans affectation | refusé ; lire sans affectation, permis |
| `cargo tree -p programme` | **aucune arête** vers `identity`, `org` ni `event` |

---

## Les portes à passer avant de livrer

```bash
make check-back      # fmt, clippy -D warnings, cargo test --workspace
make check-front     # inchangé par B4, mais la porte est globale
make check           # + check-db : DÉTRUIT LE VOLUME et recharge le schéma de zéro
```

**`make check-db` détruit la base de développement.** Le lancer en dernier, une fois le reste au vert — c'est ce que B2 avait laissé pour la fin, et la leçon tient.

Trois vérifications sont mécaniques et bloquantes (gouvernance) : `cross_module_fk_report` vide — **rien n'y change ici, ce module ne crée aucune clé** —, `make check` au vert, et le graphe des crates sans arête entre deux modules.

**Aucun fichier de `backend/` au-dessus de 1000 lignes**, et c'est le module où la limite se rapproche le plus : la fiche d'évaluation compose onze tables. Le découpage par agrégat du plan est fait pour cela.

---

## Une fois que tout passe

Mettre à jour la progression : le journal du jour, `docs/progression/ecrans/b4-propositions.md`, les décisions prises en chemin, et la ligne de suivi de `docs/PROGRESSION.md`. C'est une obligation de la même force que les portes techniques.
