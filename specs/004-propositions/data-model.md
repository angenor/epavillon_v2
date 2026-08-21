# Phase 1 — Modèle de données : Propositions (B4)

**Date** : 2026-08-20 · **Source de vérité** : `docs/database/070_programme_proposals.sql` et les sections citées des autres fichiers.

**Ce document ne définit aucune table.** Le modèle existe, il fait autorité, et **B4 ne le modifie pas**. Ce fichier dit ce que le code lit, ce qu'il écrit, ce que la base tient déjà, et les quatre endroits où le service tient ce que la base ne tient pas.

---

## 1. Ce que le module écrit, et ce qu'il ne fait que lire

### Écrit — schéma `programme` (fichier `070`)

| Table | Ce que le module y fait |
|---|---|
| `proposals` | crée en brouillon, enregistre, dépose, renvoie, décide, efface logiquement |
| `proposal_organizations` | ajoute et retire les co-organisations. **La ligne du porteur est posée par déclencheur** — jamais par le service |
| `proposal_speakers` | ajoute, modifie, réordonne, retire |
| `proposal_documents` | rattache et détache un objet déjà stocké |
| `review_assignments` | confie, retire, enregistre un déport motivé |
| `reviews` | brouillon et dépôt d'une revue |
| `review_scores` | notes par critère et leur commentaire |
| `proposal_comments` | messages, demandes de correction, résolutions |
| `proposal_reads` | accusé de lecture, par la fonction du modèle |
| `proposal_transitions` | **jamais directement** — écrit par déclencheur. Sauf la déduction v1 (§ 5) |

### Écrit — hors schéma, deux dérogations bornées

| Table | Pourquoi elle n'a pas d'autre porte | Où c'est isolé |
|---|---|---|
| `reference.entity_terms` | polymorphe, sans clé étrangère vers les propositions : aucun autre module ne peut poser les thématiques d'un dossier (R11) | `repo/themes.rs` |
| `identity.people` | l'intervenant **est** une personne, la clé est obligatoire, et le contrat exige une réponse synchrone (R12). Précédent livré : le module Organisations le fait déjà pour une invitation | `repo/people.rs` |

### Lu seulement — hors schéma

| Table ou fonction | Question de **ce** module |
|---|---|
| `event.events` | « dans quel fuseau ce créneau se lit-il ? », « cette édition est-elle terminée ? » |
| `event.calls_for_proposals` | « ce dépôt est-il recevable ? », « combien de revues, quel aveugle, quelles bornes ? » |
| `event.review_criteria`, `event.max_weighted_score()` | « quelle grille note ce dossier, et sur combien ? » |
| `event.call_reviewers` | « qui siège au comité de cet appel, et quelle est sa charge ? » |
| `event.effective_deadline()` | « jusqu'à quand peut-on déposer ? » |
| `org.organizations`, `org.memberships` | « qui porte ce dossier, et qui peut écrire en son nom ? » |
| `identity.people` | les intervenants, les auteurs de messages, les membres du comité |
| `identity.has_permission()`, `identity.administered_events()` | l'autorisation et le périmètre |
| `reference.taxonomy_terms`, `terms_of()`, `term_badges()` | les thématiques, pour filtrer et pour afficher |
| `media.assets`, `media.object_url()` | les pièces du dossier et leur adresse |
| `analytics.mv_organization_scorecard` | l'historique de participation de l'organisation porteuse |
| `platform.entity_history()` | l'historique champ par champ, par la fonction du module |

**Toutes ces lectures sont réunies dans `repo/cross.rs`** — patron de B3. Dispersées, la frontière devient invisible ; réunies, un ajout se discute.

---

## 2. La machine à états, telle qu'elle est — et ce que le code en fait

Huit états, **quatorze chemins déclarés en données**. Le code n'en recopie aucun.

| Depuis | Vers | Permission requise | Porteur | Motif |
|---|---|---|---|---|
| brouillon | déposé | soumettre | oui | non |
| brouillon | retiré | — | oui | non |
| déposé | en évaluation | décider | non | non |
| déposé | corrections demandées | noter | non | **oui** |
| déposé | retiré | — | oui | **oui** |
| en évaluation | corrections demandées | noter | non | **oui** |
| en évaluation | retenu | décider | non | non |
| en évaluation | non retenu | décider | non | **oui** |
| en évaluation | retiré | — | oui | **oui** |
| corrections demandées | déposé | soumettre | oui | non |
| corrections demandées | retiré | — | oui | non |
| corrections demandées | non retenu | décider | non | **oui** |
| retenu | annulé | décider | non | **oui** |
| non retenu | en évaluation | décider | non | **oui** |

**Ce que le déclencheur fait à chaque transition acceptée** : il refuse ce qui n'est pas déclaré (`restrict_violation`), exige le motif quand la règle le dit (`not_null_violation`), **date le dépôt** et **date la décision** avec son auteur, écrit la ligne de journal, et **émet l'événement de domaine** (R2).

**Ce que le service fait** : il tente. Il ne rejoue pas le graphe, sauf pour **offrir** les transitions (R7) et pour **classer** les trois refus de recevabilité avant l'écriture (R9).

**Trois observations qui commandent du code** :

- **Le garde n'est posé que sur la mise à jour de l'état.** Une insertion échappe à la machine (écart n° 96) : le service crée donc toujours en brouillon.
- **Le motif s'écrit dans la colonne de la décision, et l'écrase** (écart n° 97) : la lecture du motif passe par le journal, jamais par la ligne.
- **Le journal est vide pour les dossiers repris de la v1** (écart n° 37) : d'où la déduction du § 5.

---

## 3. Les invariants — qui tient quoi

### Tenus par la base, jamais redoublés

| Invariant | Où | Ce que le service en fait |
|---|---|---|
| Transitions déclarées | déclencheur d'état | traduit `restrict_violation` |
| Motif exigé | déclencheur d'état | traduit `not_null_violation` |
| Fenêtre de l'appel au **premier** dépôt | déclencheur de recevabilité | classe avant (R9), le déclencheur reste le dernier mot |
| Plafond par organisation, **dossiers portés uniquement** | déclencheur de recevabilité | idem |
| Organisation vérifiée si l'appel l'exige | déclencheur de recevabilité | idem |
| Un seul porteur par dossier | index unique partiel | n'écrit jamais la ligne du porteur |
| Cohérence porteur ↔ ligne de rôle | déclencheur | n'écrit jamais la ligne du porteur |
| Numéro de dossier attribué à l'insertion | déclencheur | ne le calcule ni ne le remplace |
| Adresse d'URL unique par édition | index unique | **dérive et suffixe** (R5) — la contrainte reste l'arbitre |
| Note ≤ maximum du critère | déclencheur | traduit `check_violation`, en nommant le critère |
| Un intervenant une fois par rôle | contrainte unique | traduit en réponse nommée |
| Période souhaitée cohérente | vérification | traduit |
| Durée entre 15 et 600 minutes | vérification | traduit — les bornes **de l'appel** sont plus serrées et vérifiées par le service |
| Corps de message non vide | vérification | traduit |
| Français exigé sur un texte multilingue | domaine | traduit, en nommant le champ par le nom du domaine |

### Tenus par le service, parce que la base ne les tient pas

| Règle | Pourquoi elle n'est pas en base | Écart |
|---|---|---|
| Bornes d'intervenants de l'appel | aucun déclencheur ne les vérifie | n° 27 |
| Longueurs maximales des textes | la base n'a pas à trancher ce qu'est un résumé lisible | n° 28 |
| Assainissement du HTML | la colonne accepte un fragment libre | n° 32 |
| Purge des thématiques à l'effacement | table polymorphe, et la fonction annoncée n'existe pas | n° 3, n° 94 |
| Création en brouillon | le garde ne couvre pas l'insertion | n° 96 |
| Consolidation des notes après un dépôt de revue | aucun déclencheur ne l'appelle | n° 98 |
| Demande de correction forcée en visibilité partagée | les deux colonnes sont indépendantes | n° 99 |
| Filtrage des trois visibilités | aucune règle en base | — |
| Voile de l'évaluation en aveugle | dépend du lecteur, aucune vue ne peut le porter | n° 53 |
| Noter exige une affectation | rien ne le lie en base | R21 |
| Bornes de durée et plage horaire de l'appel | portées par l'appel, appliquées nulle part | — |
| Contact du dossier par défaut | colonne nullable que rien ne remplit | n° 30 |

---

## 4. Les entités, et ce que le code en manipule

**Proposition** — le dossier. Trente-six colonnes, dont sept recalculées (agrégats de notes, vecteur de recherche, compteur de vues) que le service **ne touche jamais**. La colonne de recherche est engendrée et exclue du code : aucune route de recherche plein texte n'est au contrat.

**Règle de transition** — quatorze lignes, lues, jamais écrites.

**Transition** — le journal. Écrit par déclencheur, lu par l'espace organisation et par la fiche. Semé une seule fois, par la déduction du § 5.

**Organisation associée** — clé composite, quatre rôles. Le porteur y est posé par déclencheur ; le service n'écrit que les trois autres rôles, et **refuse d'y ajouter le porteur** : l'accepter le ferait basculer en silence par le `ON CONFLICT` du déclencheur.

**Intervenant** — rattaché à une personne. Deux instantanés — fonction et organisation **au moment de l'activité** — que le modèle distingue explicitement de la fiche de la personne, et que le déposant peut modifier même quand l'identité est verrouillée.

**Pièce** — rattachée à un objet stocké. Le service ne pose ni ne détruit l'objet.

**Affectation de revue** — qui évalue quoi, pour quand. Le déport y est une date et un motif, jamais une suppression.

**Revue** — une par personne et par dossier. Brouillon tant que la date de dépôt est nulle : elle ne compte dans aucun agrégat et n'est visible d'aucun pair.

**Note par critère** — clé composite. Une note **absente** n'est pas une note à zéro : zéro sur un critère éliminatoire disqualifie le dossier.

**Échange** — trois visibilités, un fil hiérarchique, une demande de correction et sa résolution.

**Accusé de lecture** — collectif. « Ce dossier, moi, l'ai-je ouvert ? » passe par la fonction qui prend le lecteur en paramètre.

---

## 5. La déduction des transitions d'un dossier repris de la v1

**Ce que la reprise laisse** : des dossiers dans leur état final, sans aucune ligne de journal — l'insertion échappe au garde (§ 2), et le déclencheur d'insertion ne journalise que l'état de départ, qui est ici l'état d'arrivée.

**Ce que la déduction sème**, et seulement sur un dossier dont le journal est vide :

| Ligne | Instant | Condition |
|---|---|---|
| → brouillon | date de création | toujours |
| brouillon → déposé | date de dépôt | si elle existe |
| déposé → état final | date de décision | si elle existe **et** que l'état est retenu, non retenu, annulé ou retiré |

**Ce qu'elle ne fait pas** : deviner un passage par l'évaluation, ni une demande de correction. Ce qui n'est pas dans les dates du dossier n'est pas déductible, et l'inventer serait pire qu'un trou — c'est ce que le front a évité en franchissant l'étape d'évaluation dès qu'une décision existe.

**Elle est rejouable** : la condition « journal vide » est dans la même requête que l'insertion.

---

## 6. Les quatre traversées de type, héritées de B1 à B3

| Type PostgreSQL | Traversée | Pourquoi |
|---|---|---|
| `programme.proposal_status` et les autres énumérations | `text`, avec double transtypage à l'écriture | patron des trois modules livrés |
| `platform.i18n_text` | `jsonb` | document multilingue rendu tel quel au front, qui le résout |
| `numeric(n,2)` | `float8`, dans les deux sens | aucune caractéristique décimale au workspace ; l'autorité du calcul reste en base (R24) |
| `platform.email`, `platform.slug`, `platform.url` | `text` puis transtypage vers le domaine | le refus porte alors le **nom du domaine**, que le noyau sait transformer en champ fautif |

---

## 7. Volumétrie de référence

Quarante dossiers par édition dans le jeu de données ; quelques centaines attendues sur une COP. Trois revues par dossier, six critères par appel, une dizaine de membres de comité, un à cinq intervenants et zéro à trois co-organisations par dossier. Les dossiers repris de la v1 se comptent en milliers, et c'est le seul endroit où la volumétrie compte — la déduction du § 5 tourne une fois, sur tout le corpus.
