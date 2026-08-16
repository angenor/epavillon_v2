# Audit de la documentation — ePavillon v2

**Date** : 16 août 2026
**Périmètre** : les 10 fichiers Markdown du dépôt — 2 592 lignes.
`docs/database/*.sql` n'a pas été audité ; il a seulement été consulté pour vérifier que les objets cités par la documentation existent réellement.
**État** : aucun fichier n'a été modifié. Ce document est un constat, pas une correction.

**Méthode** : trois relectures à périmètre exclusif, puis contre-vérification de chaque affirmation forte — recomptage du modèle, résolution de tous les liens, existence de chaque objet SQL cité.

---

## Sommaire

- [1. Les trois problèmes les plus graves](#1-les-trois-problèmes-les-plus-graves)
- [2. Incohérences et erreurs](#2-incohérences-et-erreurs)
- [3. Ce qui manque](#3-ce-qui-manque)
- [4. Ce qui est de trop](#4-ce-qui-est-de-trop)
- [5. Avis d'ensemble](#5-avis-densemble)
- [Annexe — ce qui a été vérifié et tient](#annexe--ce-qui-a-été-vérifié-et-tient)

---

## 1. Les trois problèmes les plus graves

### ① La parole du commanditaire a disparu du disque

**Où** : `docs/historique/` — les deux fichiers `note-intention.md` (84 lignes) et `retours-cadrage.md` (18 lignes) sont **supprimés du répertoire de travail**. Ils ne subsistent que dans `HEAD` : `git status` affiche ` D `, la suppression n'est pas commitée.

Trois liens pointent dans le vide — et ce sont **les trois seuls liens cassés de tout le dépôt** :

| Lien | Fichier |
|---|---|
| `docs/historique/` | [`../CLAUDE.md:44`](../CLAUDE.md#L44) |
| `historique/` | [`README.md:14`](README.md#L14) |
| `historique/note-intention.md` | [`CADRAGE.md:544`](CADRAGE.md#L544) |

**Pourquoi ça compte.** Ce dossier est désigné comme l'arbitre du projet : si un document le contredit, c'est le document qui a tort. Or c'est précisément lui qui a retourné cinq décisions structurantes — co-organisation, chevauchements non bloqués, appel unique par édition, administrateur à périmètre limité, direct unique. Une session qui rencontre une contradiction — et il y en a plusieurs, listées ci-dessous — n'a plus rien pour trancher. Elle tranchera au jugé.

À noter : `retours-cadrage.md` n'était référencé nulle part, même lorsqu'il existait. C'est pourtant le document le plus décisif des deux.

**Correction proposée.**

```bash
git checkout HEAD -- docs/historique/
```

Puis référencer explicitement `retours-cadrage.md` dans [`CADRAGE.md:540`](CADRAGE.md#L540) §13. Si la suppression était volontaire, alors le statut d'arbitre doit être retiré des trois documents qui l'invoquent — mais ce serait perdre la seule trace du « pourquoi » de la moitié des décisions.

---

### ② Les décisions retournées ont laissé leurs vestiges dans les documents mêmes qui les corrigent

Le cas le plus net tient en neuf lignes de [`CADRAGE.md`](CADRAGE.md) :

> **Ligne 130** — « un **appel à propositions** comme entité de plein droit : un événement peut en avoir zéro (COP sans pavillon…), un, **ou plusieurs** (une journée thématique ouvre sa propre fenêtre) »

> **Ligne 139** — « **Un appel à propositions par édition, au plus.** […] Une version antérieure de ce document en autorisait plusieurs, **en imaginant qu'une journée thématique puisse ouvrir sa propre fenêtre de soumission** ; l'IFDD a tranché »

La ligne 139 décrit mot pour mot l'erreur de la ligne 130, sans que la 130 ait été corrigée. Le commanditaire avait pourtant écrit :

> *« un événement peut avoir zero ou un appel à propositions et non plusieurs (je ne sais pas pourquoi tu as pensé qu'il peut avoir plusieur appel) »*

La ligne 130 est dans la liste « Ce que la v2 ajoute » — la partie qu'on lit en diagonale.

**Les autres vestiges, tous vérifiés :**

| Vestige | Emplacement | Ce qu'il contredit |
|---|---|---|
| « **Conflits durs impossibles**, conflits souples signalés automatiquement » — indicateur de succès mesuré à six mois | [`CADRAGE.md:531`](CADRAGE.md#L531) | ADR-13. Le vocabulaire « dur / souple » n'est défini nulle part ailleurs |
| « Sessions programmées, **non-chevauchement**, formulaires d'inscription… » | [`README.md:112`](README.md#L112) | Le même fichier dit l'inverse lignes 27 et 173 |
| « une frontière violée fait échouer **la CI** » | [`CADRAGE.md:198`](CADRAGE.md#L198) | Ligne 220 : « pas de chaîne d'intégration continue » — réponse explicite à la question du commanditaire |
| Valkey : « **Pas en phase 1** » | [`CADRAGE.md:212`](CADRAGE.md#L212) | Ligne 218 : Valkey dans le `docker compose` local, comme demandé |
| ADR-01 : « **Treize schémas** » avec liste nominative | [`CADRAGE.md:318`](CADRAGE.md#L318) | 15 schémas réels — manquent `training` et `legacy` |
| « les **douze** décisions d'architecture » | [`README.md:201`](README.md#L201) | 14 ADR ; la ligne 11 du même fichier dit « 14 » |

**Pourquoi ça compte.** Ces six lignes ne sont pas des coquilles : ce sont des instructions actives. La plus grave est celle d'ADR-01 — le principe fondateur du projet est « un module = un schéma PostgreSQL = un crate Rust », et la liste censée l'illustrer omet le module Formations, celui que le commanditaire déclare *« important, il doit etre construit au MVP »*.

**Correction proposée.** Une passe ciblée sur ces six lignes. Puis, pour chaque décision retournée, garder **un seul** énoncé de référence et faire renvoyer les autres vers lui (voir §4).

---

### ③ Les trois garde-fous censés remplacer la mémoire entre sessions sont chacun percés

Le dispositif est bien conçu : préambule obligatoire en tête de chaque prompt, journal de progression, `make check` avant tout commit. Aucun des trois ne fonctionne.

**a. Le préambule ne couvre que 5 prompts sur 26.**

[`PROMPTS_DEVELOPPEMENT.md:17`](PROMPTS_DEVELOPPEMENT.md#L17) exige de le coller « en tête de **chaque** prompt de ce fichier, **sans exception** ». Le marqueur `[PRÉAMBULE]` n'apparaît qu'aux lignes **42, 102, 152, 209 et 703** — soit A0.1 à A0.4 et B7.

Les quatorze écrans **A1 → A14** et les prompts **B0 → B6** ne le portent pas. Une session à qui l'on colle le bloc A8 ne reçoit ni l'ordre de lire `MODELE_INDEX.md` et les fichiers SQL concernés, ni celui de mettre à jour `PROGRESSION.md` — c'est-à-dire exactement les deux mécanismes censés remplacer la mémoire.

**b. `PROGRESSION.md` est vide de ce qui compte.**

Il s'annonce « la mémoire du projet entre deux sessions » ([`PROGRESSION.md:3`](PROGRESSION.md#L3)), mais ses trois tables porteuses sont vides (`— | — | —`) :

- modifications du modèle de données (ligne 80)
- écarts constatés entre le modèle et l'interface (ligne 90)
- **décisions prises en cours de route** (ligne 100)

Aucune des cinq décisions retournées pendant la rédaction n'y figure : chevauchements non bloqués, appel unique par édition, ajout de la co-organisation, déplacement du workspace Cargo dans `backend/`, double refonte du découpage i18n. Elles ne sont retrouvables qu'en relisant les 557 lignes du cadrage.

*À sa décharge* : il ne surestime rien. Tout ce qu'il déclare non commencé l'est effectivement, et les 18 lignes front + 8 lignes API correspondent exactement aux prompts existants.

**c. `make check` ne peut pas échouer, et détruit la base.**

[`ENVIRONNEMENT_LOCAL.md:132-145`](ENVIRONNEMENT_LOCAL.md#L132) : les deux `SELECT count(*)` sont **affichés**, jamais testés — aucun `|| exit 1`. Le document précise « les deux compteurs doivent afficher **0** » : c'est un contrôle visuel humain. Une clé étrangère inter-modules non conforme passe le portail silencieusement.

De plus, `check-db` commence par `down -v` : **chaque `make check` détruit le volume PostgreSQL** et toutes les données saisies à la main. Ce n'est signalé nulle part. Or [`../CLAUDE.md:227`](../CLAUDE.md#L227) en fait une condition de commit.

**Correction proposée.**

1. Insérer `[PRÉAMBULE]` en tête des 21 blocs qui en manquent.
2. Remplir la table « Décisions prises en cours de route » avec les cinq décisions retournées, datées et motivées.
3. Transformer les deux compteurs en assertions et avertir du `down -v` :

```makefile
@test "$$($(PSQL) -t -A -c 'SELECT count(*) FROM platform.cross_module_fk_report WHERE NOT is_compliant;')" = "0" \
  || (echo "Clés étrangères inter-modules non conformes"; exit 1)
```

---

## 2. Incohérences et erreurs

### 2.1 Chiffres du modèle — cinq annonces sur six sont fausses

Tout a été recompté sur les 18 fichiers de `docs/database/`.

| Annoncé | Réel (recompté) | Où l'annonce apparaît |
|---|---|---|
| 149 tables | **146** (142 hors partitions `_default`) | [`../README.md:55`](../README.md#L55) · [`README.md:16`](README.md#L16) · [`PROGRESSION.md:15`](PROGRESSION.md#L15) · [`PROMPTS_DEVELOPPEMENT.md:107`](PROMPTS_DEVELOPPEMENT.md#L107) |
| 12 vues | **14** | [`../README.md:55`](../README.md#L55) · [`README.md:16`](README.md#L16) |
| 145 fonctions | **152** distinctes (153 déclarations) | idem |
| 14 schémas | **15** (`legacy` inclus) | [`../README.md:55`](../README.md#L55) |
| 167 clés étrangères inter-modules | **157** contraintes `xmod_fk_` déclarées (166 occurrences) — aucun décompte ne donne 167 | [`../README.md:55`](../README.md#L55) · [`README.md:16`](README.md#L16) · [`README.md:143`](README.md#L143) |
| 7 vues matérialisées · 18 fichiers SQL · 14 ADR · 8 règles métier | exacts | — |

Le plus coûteux est le dernier : [`README.md:143`](README.md#L143) présente « les 167 clés étrangères inter-modules sont conformes » comme un **résultat de vérification**, donc comme une preuve. C'est une preuve fausse de dix unités.

Recommandation : remplacer ces constantes recopiées par une commande qui les recompte.

### 2.2 Erreurs factuelles

- [`README.md:40`](README.md#L40) — les journées thématiques renvoient à `event.event_days` ; le SQL dit explicitement qu'elles vivent dans `event.programme_tracks`, `event_days` n'étant que le calendrier. Rend la règle métier n° 7 inapplicable.
- [`../CLAUDE.md:82`](../CLAUDE.md#L82) — « aucun fichier **du dépôt** ne dépasse 1000 lignes » : déjà faux de quatre fichiers SQL (1001, 1061, 1416, 1829 lignes). La commande de contrôle en [`PROMPTS_DEVELOPPEMENT.md:754`](PROMPTS_DEVELOPPEMENT.md#L754) ne balaie que `frontend backend` — c'est la règle qui doit être bornée au code applicatif.
- [`../CLAUDE.md:46`](../CLAUDE.md#L46) et [`MODELE_INDEX.md:3`](MODELE_INDEX.md#L3) — « plus de onze mille lignes » de SQL : **14 143** en réalité, soit 28 % de plus.
- [`CADRAGE.md:384`](CADRAGE.md#L384) — ADR-12 annonce trois tables partitionnées ; il y en a **quatre** (`analytics.page_views` manque).
- [`../CLAUDE.md:235`](../CLAUDE.md#L235) et [`../README.md:77`](../README.md#L77) — cinq modules « commandés par un drapeau `platform.feature_flags` » : **Formations et Messagerie n'ont aucun drapeau semé** dans `900_seed.sql`, et `negotiation.channels` ne couvre que les canaux. L'écran A14 ne pourra pas les piloter.
- [`CADRAGE.md:552`](CADRAGE.md#L552) — la messagerie v1 citée comme « `messages`, `connections`, `appointments` […] le schéma les prévoit » : **aucune table `appointments` n'existe** en v2.
- [`../CLAUDE.md:235`](../CLAUDE.md#L235) — « Messagerie » listée comme module au même rang que les autres : elle n'a ni schéma, ni crate, ni section §4.x. Ses tables vivent dans `engagement`.
- [`README.md:210`](README.md#L210) — renvoi vers « `PROMPTS_DEVELOPPEMENT.md` **§2** — monter l'environnement local » : ce fichier n'a aucune section numérotée, et l'environnement local est le sujet d'un autre document. Renvoi doublement faux.
- [`README.md:212`](README.md#L212) — renvoie à un « prompt A0 » qui n'existe pas (A0.1 à A0.4).
- [`../README.md:78`](../README.md#L78) — un `# epavillon_v2` orphelin en fin de fichier, résidu du README auto-généré.
- [`README.md:51-90`](README.md#L51) — la carte des modules omet `training` (1829 lignes, le plus gros fichier du modèle) et `legacy`.
- [`PROMPTS_DEVELOPPEMENT.md:745`](PROMPTS_DEVELOPPEMENT.md#L745) — « les deux écrans à traiter en premier après le socle : A2 et A8 » contredit le diagramme des lignes 724-743, qui place A1 avant A2 et A6-A7 avant A8.
- [`PROMPTS_DEVELOPPEMENT.md:650`](PROMPTS_DEVELOPPEMENT.md#L650) — `identity.administered_events` présenté comme joignable ; c'est une **fonction** retournant une table, pas une vue.
- [`PROMPTS_DEVELOPPEMENT.md:646`](PROMPTS_DEVELOPPEMENT.md#L646) contre [`../CLAUDE.md:182`](../CLAUDE.md#L182) — deux formulations de la même règle : « passent par `platform.outbox_events` » contre « par `platform.emit_event()` ». Citer la fonction, pas la table.
- [`PROMPT_STYLE_GUIDE.md:18`](PROMPT_STYLE_GUIDE.md#L18) contre [`:162`](PROMPT_STYLE_GUIDE.md#L162) — « artefact HTML autonome, aucune ressource externe » et « TailwindCSS v4 » sont incompatibles : le rendu sera soit non autonome, soit en syntaxe v3.
- [`PROMPT_STYLE_GUIDE.md:80`](PROMPT_STYLE_GUIDE.md#L80) contre [`CHARTE_GRAPHIQUE.md:77`](CHARTE_GRAPHIQUE.md#L77) — deux conventions incompatibles pour **le même** `design-tokens.css` : `--color-surface` contre `--ifdd-cyan`. A0.1 génère les seconds, le guide de style produira les premiers.
- [`PROMPT_STYLE_GUIDE.md:5`](PROMPT_STYLE_GUIDE.md#L5) et `:199` contre [`PROMPTS_DEVELOPPEMENT.md:227`](PROMPTS_DEVELOPPEMENT.md#L227) — `style-guide.vue` a deux origines contradictoires : artefact HTML conservé, ou page Vue créée par A0.4. Ce ne sont pas le même fichier.
- **Chemins front incohérents** : `frontend/types/` et `frontend/mocks/` ([`PROMPTS_DEVELOPPEMENT.md:112`](PROMPTS_DEVELOPPEMENT.md#L112), `:154`) hors de `app/`, mais `frontend/app/components/ui/` (`:211`) dedans, et `composables/useApi.ts` ([`../CLAUDE.md:74`](../CLAUDE.md#L74)) jamais qualifié. En Nuxt 4 (`srcDir: app/`), deux de ces trois emplacements cassent l'auto-import.
- [`ENVIRONNEMENT_LOCAL.md:63`](ENVIRONNEMENT_LOCAL.md#L63) — ports Garage 3901 et 3903 déclarés mais non publiés ; aucune procédure `garage layout assign` ni création de bucket : le service démarre et refuse toute écriture.
- [`ENVIRONNEMENT_LOCAL.md:22`](ENVIRONNEMENT_LOCAL.md#L22) — `pg_stat_statements` est créé par `000_bootstrap.sql` mais `shared_preload_libraries` n'est pas configuré : extension présente, inexploitable.
- [`CADRAGE.md:540`](CADRAGE.md#L540) — le §13 « Documents liés » ne cite que trois cibles, dont une cassée, et **omet les six autres documents du dossier**. Il est resté à l'état du premier jour.

### 2.3 Une règle affichée que la base ne garantit pas

Hors périmètre Markdown, mais elle invalide une des huit règles métier.

La règle n° 4 — « un seul direct à la fois, **tous événements confondus** » ([`../CLAUDE.md:57`](../CLAUDE.md#L57)) — n'est pas tenue. L'index `ux_streams_single_live_per_channel` porte sur `broadcast_channel_id`, et `event.broadcast_channels` porte un `event_id` : deux événements ayant chacun leur canal peuvent diffuser simultanément. [`CADRAGE.md:147`](CADRAGE.md#L147) présente même l'ouverture d'un second canal comme un bénéfice — c'est la levée de la règle.

Le commanditaire est explicite : *« 2 activité de 2 evenements differents peuvent avoir lieu simultanément mais 1 seul en direct »*.

À signaler également : deux commentaires SQL référencent encore `programme.ex_sessions_no_broadcast_overlap` (`060_events.sql:308`, `080_live.sql:595`) — **cette contrainte n'existe dans aucun fichier**. C'est le fossile de la contrainte d'exclusion retirée par ADR-13.

### 2.4 Passages à double lecture

- [`../CLAUDE.md:56`](../CLAUDE.md#L56) — règle n° 3 « Deux activités d'une même édition **ne peuvent pas** se tenir en même temps », juste sous la règle n° 2 qui interdit de bloquer. Se lit comme un blocage à implémenter.
- [`PROMPTS_DEVELOPPEMENT.md:477`](PROMPTS_DEVELOPPEMENT.md#L477) — niveau de gravité nommé « **BLOQUANT** (rouge) » six lignes après « les chevauchements NE SONT PAS BLOQUÉS ». Le sens voulu est une gravité d'affichage ; le mot dit l'inverse. Il inclut « une salle physique réservée deux fois », précisément la formulation que le commanditaire a corrigée.
- [`CADRAGE.md:131`](CADRAGE.md#L131) — l'« évaluation en aveugle » **inverse** une demande explicite du commanditaire (*« voir les notes attribué par les autres révisionniste, la moyenne des notes »*). Dite « optionnelle », sans valeur par défaut ni niveau de réglage.
- [`CADRAGE.md:493`](CADRAGE.md#L493) — Formations en **jalon 3**, mais « livré dès que le jalon 1 est stabilisé ». Le terme « produit minimum viable », employé deux fois, n'est défini nulle part.
- [`CADRAGE.md:139`](CADRAGE.md#L139) — « par édition » puis « un **événement** porte zéro appel », l'index réel s'appelant `ux_calls_one_per_event` : série / édition / événement ne sont jamais définis.
- [`CADRAGE.md:483`](CADRAGE.md#L483) — « soumettre une proposition **à** plusieurs organisations » ; le sens visé est « au nom de ». C'est un critère de recette.
- **La date de la COP31 n'est fixée nulle part**, et les deux seuls repères datés se contredisent : novembre **2026** ([`CADRAGE.md:466`](CADRAGE.md#L466) et `:514`) contre « 9–20 novembre **2027** » dans le mock de référence ([`PROMPTS_DEVELOPPEMENT.md:180`](PROMPTS_DEVELOPPEMENT.md#L180)) — lequel situe par ailleurs la COP31 à Belém, ville de la COP30.

---

## 3. Ce qui manque

Par ordre d'importance pour la suite du projet.

1. **La méthode de travail du commanditaire n'est écrite nulle part dans le cadrage.** *« developper d'abord le front-end avec des donners mocks en tenant compte du modele de données […] ensuite developper le backend et l'interconnecter au front. Je developpe le backend avec gitHub speckit »* — zéro occurrence de `mock` ou `speckit` dans `CADRAGE.md`. C'est la seule instruction de *comment travailler* du dossier. Elle survit dans `PROMPTS_DEVELOPPEMENT.md`, mais le document qui fixe le cap l'ignore.

2. **Un prompt « A0.0 — initialisation du dépôt »** : `ops/docker-compose.dev.yml`, `ops/garage.toml`, `Makefile`, `.gitignore`, `.env.example`. C'est la toute première action attendue ([`PROGRESSION.md:30`](PROGRESSION.md#L30)) et la seule sans prompt. Conséquence déjà visible : **4 fichiers `.DS_Store` sont versionnés** faute de `.gitignore`.

3. **La chaîne de connexion PostgreSQL**, jamais donnée en un seul morceau — alors que SQLx à vérification compilée en a besoin *pour compiler* (`check-back`). Ni le contenu de `.env.example`, ni le nom de la variable pointant l'API côté front.

4. **Où vivent les crates `api`, `worker` et `kernel`.** `CLAUDE.md` lance `cargo run -p api` ; les prompts ne définissent que `backend/crates/modules/`. B1 les inventera, B2 en héritera.

5. **Google Agenda / Apple, « phase 2+ »** — la seule exigence que le commanditaire ait lui-même datée d'une phase : **zéro occurrence dans les dix fichiers Markdown**. Elle ne survit que dans un drapeau du seed SQL (`calendar.external_sync`).

6. **Toutes les exigences d'interface du commanditaire** : `vue-cal` (nommé deux fois, dont une comme l'outil même de l'arbitrage des créneaux), vue grille, couleurs selon l'état, sélecteur d'année, « section autre » de la page programmation, typologie événements spontanés / périodiques. Le cadrage ne porte aucune exigence d'IHM. Une partie est rattrapée par les prompts, pas toutes.

7. **Le nommage des jetons de design**, à trancher une fois pour toutes (`--ifdd-*` ou `--color-*`), et la décision sur la bibliothèque de calendrier (A9) et de graphiques (A6) — avant que deux écrans ne fassent deux choix.

8. **`CHARTE_GRAPHIQUE.md` ne contient pas sa source.** Le commanditaire fournit `@nouvelle_version/Charte_graphique_Ifdd_couleurs.md` ; ce fichier n'est pas dans le dépôt, et la charte dit dériver d'un `.ai` également absent. Les 15 couleurs ne sont pas retraçables.

9. **Les valeurs de rappel demandées** — 2 jours, 1 jour, 1 heure, 30 minutes, cumulées. Le principe « règle de rappel administrable » est présent, les valeurs nulle part.

10. **Bilinguisme, accessibilité et responsive sont absents du cadrage.** `CLAUDE.md` les impose comme règles de code, mais aucune exigence produit ne les porte, y compris dans les non-objectifs. On ne sait donc pas si l'accessibilité est écartée ou oubliée — pour une plateforme francophone à audience largement mobile, la question mérite d'être posée.

11. **La reprise des données v1 est placée au jalon 2**, alors que le critère de sortie du jalon 1 exige « le parcours complet jouable sur des **données réelles importées** ». La reprise est donc requise avant le jalon dont elle dépend.

12. **Une réponse à « qui sert la COP31 »** — v1 ou v2, et comment les propositions collectées d'un côté rejoignent l'exécution de l'autre. Ni tranché, ni consigné parmi les questions ouvertes.

---

## 4. Ce qui est de trop

- **La doctrine « tout ce qui est vrai n'a pas vocation à être bloqué » est écrite trois fois** ([`CADRAGE.md:282`](CADRAGE.md#L282), [`../README.md:59`](../README.md#L59), [`README.md:173`](README.md#L173)) et la règle des chevauchements **cinq fois**. C'est la décision la plus récemment retournée, donc celle où la divergence coûte le plus — et [`README.md:112`](README.md#L112) diverge déjà.
  → Garder [`../CLAUDE.md:55`](../CLAUDE.md#L55) comme énoncé de référence ; les autres y renvoient.

- **Le tableau « Où trouver quoi » existe en deux versions** ([`../CLAUDE.md:33-44`](../CLAUDE.md#L33) et [`README.md:5-14`](README.md#L5)), avec sept lignes communes et un désaccord déjà installé (14 ADR contre douze).
  → Garder celui de `CLAUDE.md`, réduire l'autre à un renvoi.

- **Les chiffres du modèle sont énoncés trois fois** et sont faux aux trois endroits.
  → Un seul énoncé, ou mieux : une commande qui les recompte.

- **La liste des cinq modules « En cours de maintenance » apparaît quatre fois** à l'identique ([`../CLAUDE.md:235`](../CLAUDE.md#L235), [`../README.md:77`](../README.md#L77), [`CADRAGE.md:107`](CADRAGE.md#L107), [`CADRAGE.md:481`](CADRAGE.md#L481)) — et elle est fausse aux quatre.

- **Le catalogue des composants est dupliqué et diverge déjà** : [`PROMPT_STYLE_GUIDE.md:117-154`](PROMPT_STYLE_GUIDE.md#L117) exige un fil d'Ariane, un menu contextuel, un champ de recherche et une barre de navigation qu'A0.4 ne crée pas.
  → Garder A0.4 comme liste faisant foi, puisque c'est elle qui devient du code.

- **Le découpage i18n est reproduit quasi mot pour mot** dans [`../CLAUDE.md:87-112`](../CLAUDE.md#L87) et [`PROMPTS_DEVELOPPEMENT.md:57-89`](PROMPTS_DEVELOPPEMENT.md#L57). Il a déjà été revu deux fois ; deux copies garantissent une troisième divergence.
  *À leur crédit : les deux sont parfaitement conformes au découpage final. Aucun reste d'un état antérieur n'a été trouvé.*

- **[`CHARTE_GRAPHIQUE.md:106-133`](CHARTE_GRAPHIQUE.md#L106)** — 28 lignes sur 132 décrivent le fichier Illustrator source, ses plans de travail et un message d'avertissement Adobe. Aucune valeur pour le développement, et le fichier n'est pas dans le dépôt. La ligne `[Registration]` (couleur d'impression, HEX « — ») n'a rien à faire dans un document servant à dériver des jetons web.

- **La liste ordonnée des 18 fichiers SQL existe en deux mises en forme** ([`README.md:102-121`](README.md#L102) en tableau, [`MODELE_INDEX.md:73-92`](MODELE_INDEX.md#L73) en bloc ASCII) — deux maintenances pour un contenu identique.

---

## 5. Avis d'ensemble

Le dossier est très au-dessus de la moyenne : la doctrine est explicite, les décisions sont motivées et pas seulement énoncées, le découpage anti-contexte est réellement pensé, et `MODELE_INDEX.md` est irréprochable — ses 160 objets SQL ont été vérifiés un par un, aucun n'est faux.

Une session peut travailler, mais elle se trompera sur des points précis et prévisibles : elle modélisera plusieurs appels à propositions si elle s'arrête à `CADRAGE.md:130`, cherchera les journées spéciales dans la mauvaise table, croira que `make check` la protège, et produira 146 types en croyant en avoir oublié trois.

Ce qui l'en empêche le plus n'est pas l'erreur isolée : c'est que **le dossier ne dit jamais lequel de deux passages contradictoires est le plus récent** — et que l'arbitre désigné, `docs/historique/`, a été effacé du disque.

Le second frein est que le dispositif de mémoire est percé aux trois endroits qui devaient le tenir : le préambule ne couvre que 5 prompts sur 26, `PROGRESSION.md` ne consigne aucune des cinq décisions retournées, et `make check` ne peut pas échouer.

Trois gestes changeraient l'essentiel : restaurer `docs/historique/`, nettoyer les six lignes vestiges du §1②, et remplir la table « Décisions prises en cours de route ». Une demi-journée de travail, et le dossier passe de « bon mais piégé » à « fiable ».

---

## Annexe — ce qui a été vérifié et tient

Un audit qui ne signale que les défauts donne une image fausse. Ces points ont été contrôlés et sont corrects :

| Contrôle | Résultat |
|---|---|
| Les 160 objets SQL cités par `MODELE_INDEX.md` | **tous existent**, dans le bon schéma |
| Les objets SQL cités par les autres documents (`merge_organizations`, `detect_conflicts`, `publication_readiness`, `administered_events`, `has_permission`, `emit_event`, `uuid_v7`, `cross_module_fk_report`, `usage_quotas`, `i18n_text`, `taxonomy_terms`, `ux_calls_one_per_event`…) | **tous existent** |
| Liens Markdown du dépôt, résolus un par un | **3 cassés sur l'ensemble**, tous vers `docs/historique/` |
| Renvois de section internes à `CADRAGE.md` (§2.1, §2.2, §2.4, §2.5, §5.6, §10) | **tous corrects** |
| Renvois de `CLAUDE.md` vers `CADRAGE.md` (§2, §5, §6, §10) | **tous corrects** |
| Numérotation de `CADRAGE.md` (1→13 + annexe, sous-sections 2.x à 8.x) | **aucun saut, aucun doublon** |
| Découpage i18n dans `CLAUDE.md` et les prompts | **conforme à l'état final**, aucun reste |
| Emplacement du workspace Cargo (`backend/`) | **cohérent partout**, aucun reste d'un `Cargo.toml` racine |
| Les huit règles métier de `CLAUDE.md` face à la parole du commanditaire | **fidèles** (une réserve de formulation sur la règle n° 3) |
| Terminologie des rôles (« révisionniste » = `reviewer`) | **conforme au modèle et au commanditaire** |
| Ports et services de `ENVIRONNEMENT_LOCAL.md` face à `CLAUDE.md` | **concordants** (Mailpit 8025, Jaeger 16686, Postgres 5432, Valkey 6379) |
| Décompte des prompts (18 front, 8 API) face à `PROGRESSION.md` | **exact** |
| Format des numéros de dossier (`COP31-00147`) face au trigger SQL | **conforme** |
| Les 15 couleurs de `CHARTE_GRAPHIQUE.md` face à `PROMPT_STYLE_GUIDE.md` | **identiques**, valeur par valeur |
| Exigences du commanditaire sur Zoom, YouTube, quotas éditoriaux, historique des modifications, ratio d'acceptation, admin scopé, co-organisation | **toutes reprises** |
