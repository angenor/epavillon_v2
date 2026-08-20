# Démarrage rapide — Événements (B3)

**Fonctionnalité** : Événements (B3) · **Date** : 2026-08-20

Comment lancer, **éprouver à la main**, et vérifier. Ce qui suit se joue dans un terminal et dans un navigateur, jamais dans un test seul : B1 a trouvé six défauts en jouant les parcours et zéro en les relisant, B2 en a trouvé deux que seule une requête HTTP réelle pouvait montrer.

---

## Préalables

Ceux de B1 et B2, inchangés — `cp .env.example .env`, `make up`, les trois interfaces (Mailpit, Jaeger, documentation de l'API). Voir [`../001-socle-identite/quickstart.md`](../001-socle-identite/quickstart.md).

**Une seule clé nouvelle** :

```
EVENT_CALL_AUTOCLOSE_INTERVAL=1h      # cadence de la clôture automatique des appels (R15)
```

Comme les autres, **le démarrage échoue si elle est mal écrite** : une durée illisible arrête le service, jamais une requête.

**Aucun courriel n'est envoyé par ce module.** Mailpit reste utile pour vérifier que rien n'en part.

---

## Lancer

```bash
cd backend && cargo run -p api        # l'API
cd backend && cargo run -p worker     # relais d'outbox, file de travaux, clôture des appels
```

Le site n'est pas nécessaire ici : ce module n'envoie rien. Le lancer sert seulement à vérifier que rien n'a régressé côté écrans.

---

## Éprouver les parcours à la main

### D'abord, de quoi parler — et le semis ne donne pas d'édition

`900_seed.sql` sème **quatre séries** (climat, biodiversité, désertification, rendez-vous de l'IFDD) et **un canal général de la plateforme**, sans édition, actif et par défaut. **Aucune édition n'est semée.**

Deux conséquences, et les ignorer coûte une demi-heure (c'est l'écart n° 86 de B2 en miroir) :

- il faut **créer une édition** avant de pouvoir éprouver quoi que ce soit ;
- le canal général est déjà le défaut de son propre groupe : poser un canal par défaut sur une édition **ne le déloge pas**, et c'est le comportement attendu (R6).

Se connecter comme super-administrateur, puis créer l'édition de référence : série climat, millésime 2027, pavillon **tenu**, fuseau `America/Belem`, du 9 au 20 novembre 2027, pays Brésil, ville Belém.

### Le sigle — l'écart n° 9, et le cœur du prompt

À éprouver dans l'ordre, parce que chaque cas ferme une porte différente :

1. Créer l'édition ci-dessus **sans sigle** → refus, champ `acronym`, et la réponse porte **une valeur proposée** dérivée du libellé. Vérifier qu'elle est utilisable telle quelle.
2. La créer avec `COP31` → acceptée.
3. Créer une seconde édition **sans pavillon** et sans sigle → **acceptée**. C'est le cas PACO : la règle ne doit pas casser un usage existant.
4. Sur cette seconde édition, basculer « pavillon tenu » sans fournir de sigle → **refus**. La règle porte sur l'état résultant, pas sur l'état antérieur.
5. Sur la COP31, **retirer** le sigle → **refus**, le même. C'est le cas que l'on oublie le plus souvent.
6. Essayer `A`, treize caractères, `COP 31`, `COP31é` → chacun refusé sur son champ.

**Puis vérifier ce que tout cela sert** : déposer un dossier sur la COP31 (ou l'insérer à la main) et lire son numéro. Il doit commencer par `COP31-`, pas par les huit premières lettres de l'adresse d'URL.

### Le périmètre, et l'URL forgée

Attribuer à un second compte le rôle d'administrateur sur la **seconde** édition seulement, puis, connecté avec lui :

- lister les éditions → **une seule** remonte, et `is_global_scope` vaut faux ;
- ouvrir le détail de la COP31 en forgeant son identifiant → **refus**, indiscernable d'un identifiant inexistant ;
- forger l'identifiant d'un fil, d'un lieu, d'une salle, d'un canal et de l'appel de la COP31 sur les **six** routes qui remontent par un enfant → refus sur chacune (R2) ;
- **tenter de créer une édition** → refus explicite : la création exige la portée globale.

Puis, avec un compte **sans aucun** droit d'administration : lister → **refus explicite**, jamais une liste vide. C'est le défaut silencieux que le modèle décrit et qu'un garde testant « pas global » reproduirait.

### Le calendrier, et le jour qui se décale

Sur la COP31 (fuseau de Belém, trois heures derrière l'UTC) :

- demander le **plan** de génération → il annonce douze journées à créer, aucune hors période, rien d'inchangé. **Relire la base : rien n'a été écrit.**
- générer → douze journées, **datées du 9 au 20 novembre**. Si la première tombe le 8 ou le 10, le fuseau n'a pas été appliqué (R5) — c'est le défaut le plus discret du module.
- habiller la journée du 12 (titre, adresse de page, couleur), puis **relancer la génération** → l'habillage est intact.
- resserrer la période au 10–18, demander le plan → il annonce **quatre journées hors période** avec le nombre de séances qu'elles portent.
- générer **sans** cocher le retrait → rien n'est supprimé.
- générer **en** cochant → les quatre partent, et `sessions_detached` est **exact au chiffre près**. Le vérifier en comptant avant.

Enfin, sur une édition d'un an — le cas du cycle de webinaires : le plan doit annoncer **plus de trois cents journées**, et **ne rien écrire**. C'est ce qui rend l'arbitrage possible plutôt que de le devancer.

### Le canal par défaut, et celui qu'on ne déloge pas

- Créer un canal sur la COP31, le marquer par défaut → il l'est.
- En créer un second, le marquer par défaut → **le premier ne l'est plus**, sans qu'on ait rien décoché.
- Vérifier que le **canal général de la plateforme** est toujours actif et par défaut de son propre groupe.
- Essayer de modifier ce canal général depuis l'édition → refus `platform_channel`.
- Retirer un canal jamais utilisé → supprimé.
- Rattacher une séance à un canal, puis le retirer → **désactivé, pas supprimé**, réponse `ok: true` avec `deactivated`. C'est un succès, et l'écran doit le lire comme tel.

### L'appel, sa grille, et le critère qu'on ne peut pas retirer

- Demander la **grille par défaut** → six critères, dont un éliminatoire, avec leurs poids. Les comparer à `event.seed_default_criteria()` : les valeurs doivent être identiques, pas « équivalentes ».
- Créer l'appel de la COP31 avec cette grille → accepté.
- En créer un **second** sur la même édition → refus `already_exists`.
- Annuler le premier, en créer un nouveau → **accepté**. L'index exclut les annulés.
- Envoyer une grille vide → refus `criteria_empty`. Deux critères de même code → refus **avec le rang** de la ligne.
- Poser une clôture avant l'ouverture, une prolongation avant la clôture, une durée par défaut hors bornes, une fermeture de pavillon avant son ouverture → quatre refus distincts, chacun sur son champ.
- Prolonger l'appel → l'échéance effective bouge, **l'échéance initiale est toujours là**.
- Poser des notes sur un critère (ou les insérer), puis **retirer ce critère de la grille** → refus 422 nommant le critère et son nombre de notes. **Puis relire `programme.review_scores` : les notes doivent être toutes là.** C'est le seul endroit du module où la base aurait détruit sans rien dire (R9).
- Modifier le barème d'un critère qui porte des notes → accepté, et `scores_affected` vaut vrai.

### Le comité, et ce qu'il n'accorde pas

- Composer le comité en un geste : deux ajouts, un pilote, un plafond.
- Retirer un membre qui porte des dossiers → accepté, et la réponse **le nomme** avec son nombre de dossiers.
- Ajouter quelqu'un qui ne détient pas la permission d'évaluer sur l'édition → ajouté, et `has_review_permission` vaut faux. **Vérifier en base qu'aucun rôle ne lui a été attribué** : siéger n'accorde rien.
- Envoyer deux fois la même personne dans la charge utile → accepté, une seule ligne.
- Avec un compte détenant `event.event.manage` **sans** `event.call.manage` : écrire un fil → accepté ; écrire l'appel ou le comité → **refusé**. Puis l'inverse.

### La publication, et le seul contrôle bloquant du module

Il faut des séances : les insérer à la main tant que B5 n'existe pas.

- Poser deux séances qui se recouvrent dans la **même salle physique** → demander le contrôle préalable : un point `blocking` nommant la salle.
- Publier → **rien n'est publié**, `blocked` vaut vrai, et la liste dit quoi régler. Relire `event.events.programme_published_at` : toujours nul.
- Décaler l'une des deux → le contrôle ne rend plus que des avertissements.
- Publier → accepté. Vérifier **trois** choses : la date posée sur l'édition, `published_count` égal au nombre de séances désignées, et **exactement un** événement `event.programme.published` dans la file de sortie.
- **Republier** → la date d'origine ne bouge pas et **aucun second événement** n'apparaît.
- Publier une édition **sans aucune séance** → accepté, zéro séance, liste vide. Ce n'est pas un conflit.
- Avec un compte détenant `event.event.manage` mais pas la permission de planifier → **refus** (R12).

### Ce que le worker tient à jour

- Poser un appel `open` dont l'échéance effective est passée.
- Attendre une occurrence de la clôture automatique → l'appel passe `closed` et un événement `event.call.closed` est écrit.
- **Arrêter le worker, provoquer un second appel échu, redémarrer** → la chaîne se réarme et l'appel est clos. C'est le point de contrôle de B1, rejoué ici.

---

## Les tests

```bash
cd backend && cargo test -p event          # ce module seul
cd backend && cargo test --workspace       # tout, y compris ce que B1 et B2 tiennent déjà
```

### Les quatre obligations de la constitution, et les tests qui les tiennent

| Obligation (principe X) | Test |
|---|---|
| chemin nominal de chaque route | `toutes_les_routes_repondent.rs` — **les 37 sur la vraie application** |
| refus par périmètre, URL forgée comprise | `perimetre_edition_url_forgee.rs`, `perimetre_vide_refuse.rs` |
| traduction d'au moins un invariant de la base | `contraintes_edition_traduites.rs`, `contraintes_appel_traduites.rs` |
| écriture des événements attendus | `outbox_evenements_du_module.rs` — **et ce qui n'émet rien** |

### Les vérifications propres à ce module

| Test | Ce qu'il tient, et ce qui tomberait sans lui |
|---|---|
| `sigle_obligatoire_avec_pavillon.rs` | les quatre chemins de R1. Sans lui, la bascule en pavillon passerait |
| `critere_porteur_de_notes.rs` | le refus **et** les notes toujours là après. Sans lui, une cascade détruirait un argumentaire opposable |
| `canal_par_defaut_unique.rs` | l'ordre retirer-puis-poser, y compris concurrent. Sans lui, l'index refuse à l'exécution |
| `detachement_compte_avant.rs` | le chiffre annoncé égale le chiffre réel, pour la journée, le fil, la salle et le lieu |
| `jours_civils_dans_le_fuseau.rs` | une édition à Belém commence le bon jour |
| `publication_bloquee_puis_publiee.rs` | le refus, la publication, l'unique événement, la republication inoffensive |
| `detail_en_une_reponse.rs` | les six onglets et leurs décomptes justes après une écriture dans l'un d'eux |
| `editions_publiques_sans_session.rs` | le brouillon et l'annulée absents, l'annoncée présente, l'édition hors série présente, les trois images embarquées |

---

## Les portes à passer avant de livrer

```bash
make check-db      # détruit le volume, recharge le schéma, vérifie les 16 schémas
                   # et que cross_module_fk_report ne contient aucune ligne non conforme
make check-back    # fmt --check · clippy -D warnings · cargo test --workspace
make check         # les trois, depuis la racine
```

**Trois vérifications sont mécaniques et bloquantes** :

```bash
cd backend && cargo tree -p event | grep -E 'identity|org'   # doit ne RIEN rendre
psql -c 'SELECT * FROM platform.cross_module_fk_report WHERE NOT is_compliant;'   # zéro ligne
find backend -name '*.rs' | xargs wc -l | sort -rn | head -5  # aucun fichier au-dessus de 1000
```

**Et une quatrième, propre à ce jalon** : vérifier qu'aucune ligne du module n'écrit hors du schéma `event`.

```bash
grep -rnE 'INSERT INTO (programme|media|identity|org|reference)\.|UPDATE (programme|media|identity|org|reference)\.' \
     backend/crates/modules/event/src
# doit ne RIEN rendre
```

`repo/cross.rs` réunit toutes les lectures hors schéma (R14). Ce contrôle dit qu'il n'y a rien d'autre, et qu'aucune n'est devenue une écriture.

---

## Une fois que tout passe

Éprouver au **navigateur**, sur les données simulées, que rien n'a régressé côté écrans — `NUXT_PUBLIC_API_BASE` reste vide jusqu'à B7. Puis mettre à jour la progression : journal du jour, `docs/progression/ecrans/b3-evenements.md`, décisions prises en chemin, et la ligne de suivi dans `docs/PROGRESSION.md`.
