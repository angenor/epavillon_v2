# Research — Guide Négo, sessions de négociation (étape 3a)

Chaque décision : ce qui est retenu, pourquoi, ce qui est écarté.

---

## R1 — Une session est une ligne de `negotiation.meetings`

**Décision** : `kind = 'negotiation_session'`, colonnes ajoutées pour ce que l'import porte
([data-model § 3](data-model.md)), toutes nulles pour les réunions de l'étape 4.

**Pourquoi** : ADR-008 et `02-domaine.md` le disent ; rien ne lit encore la table, l'étendre ne casse
rien. L'étape 4 (réunions de la Francophonie) et 3b (signalements) visent la même table.

**Écarté** : une table `official_sessions` à part — deux tables pour « une réunion », exactement la
correction D2 de la v1.

## R2 — Le format de la source officielle

**Constaté le 25/09** sur les données réelles de la COP29 et de la COP30 (échantillons pris au
navigateur).

- **La source** est le calendrier de conférence du site de la CCNUCC :
  `https://unfccc.int/unfccc-conference-calendar/{applicationID}` rend **toute la COP en un JSON**
  (COP29 = `11685`, 1 423 entrées ; COP30 = `12286`, 1 529 entrées). La page
  `https://unfccc.int/cop30/schedule` porte l'identifiant (`drupalSettings.applicationID`) et le fuseau.
- **Une entrée** : `id`, `title` (anglais), `date: [{value, end_value}]`, `room: [{value}]`,
  `typeofevent: [str]`, `access: [{value: "0" | "1"}]` (« Open meeting » / « Limited access »),
  `body`, `topic`, `keywords`, `url` (la fiche, l'original), `time`, `linktowebcast`, `haswebcast`,
  `maintextbody`.
- **Pas de champ d'état** : l'annulation et le report se lisent en préfixe du titre (« CANCELLED - »,
  « ** POSTPONED ** ») ; un déplacement ne se voit qu'en comparant deux lectures — c'est exactement ce
  que fait le comparateur.
- **Le point de l'ordre du jour et la nature de la réunion sont dans le titre** : « CMA 8 (a) Global
  goal on adaptation - Informal consultation » (COP30), « … - Informal consultation: CMA 9 (a) »
  (COP29), « HoDs draftng group CMA 11 (a) -NCQG ». Un analyseur tolérant les deux ordres et les
  fautes extrait le code (`(COP|CMP|CMA|SBI|SBSTA|APA) \d+(\s*\([a-z]\))?`) et l'intitulé.
- **Aucun document lié** à une session, ni dans le JSON ni sur la fiche : la section de la fiche
  disparaît à cette étape (FR-029).
- **Les heures sont naïves, et décalées** : `value` vaut l'heure locale **moins une heure** (vérifié
  sur trois réunions, COP29 et COP30 ; le site lui-même corrige en dur). Le lecteur applique une
  **correction réglable** (`time_correction_minutes`, 60 par défaut) puis interprète l'heure dans le
  fuseau de l'édition.
- **Les « Coordination meetings »** mêlent les groupes de Parties (G77 et Chine, LMDC, EIG, Groupe
  africain, PMA…) et les observateurs (YOUNGO, BINGO, ENGO…) : seuls les premiers, résolus dans
  `negotiation_group`, sont importés (FR-012a).
- **Une protection anti-robot** (Imperva) refuse un client HTTP sans navigateur : `curl` reçoit une
  page vide. **Le lecteur réel ne peut pas tourner tel quel** : il lui faudra l'accès ouvert par
  l'accord du secrétariat (liste d'autorisation, flux dédié), et non un navigateur sans interface
  à entretenir sur le serveur.
- **Conditions d'utilisation** : toute reproduction demande l'accord écrit du secrétariat — le jeu
  archivé du dépôt est borné à ce que la recette exige (voir R3), et son `LISEZMOI.md` cite la source.

**Décision de structure** : un trait `SourceOfficielle` — `async fn lire(&self) ->
Result<Vec<SessionLue>, EchecLecture>` — rend une **forme pivot** (`SessionLue` : clé, titre brut,
catégorie, début, fin, salle, accès, URL). L'analyse du format CCNUCC est **commune aux deux
lecteurs** (`import/ccnucc.rs`) : `LecteurArchive` lit un jeu embarqué, `LecteurReel` le même JSON
par HTTP, écrit mais branché sur rien tant que l'accès manque. Le comparateur ne connaît que la
forme pivot, enrichie par la résolution (R4).

## R3 — Le jeu archivé

**Décision** : `backend/crates/modules/negotiation/src/import/archives/cop30/` — le JSON réel de la
COP30, **réduit aux réunions que l'import retient** (négociations, plénières, événements mandatés,
consultations de la présidence, coordinations de Parties) et **aux 17 et 18 novembre 2025** — une
réduit à **quelques dizaines de réunions** (dont un cas « POSTPONED », une coordination de groupe reconnu et une non reconnue), structure et clés intactes —, tel quel dans sa forme : `lecture-1.json`. `lecture-2.json` en
diffère de quatre écarts (une session avancée, une changée de salle, une disparue, une nouvelle,
reconstituée et dite telle). Un fichier `injoignable` fait échouer la lecture. Le back-office choisit
le fichier (`archive_name`) ; `archive_first_day` **translate les jours** de l'archive, en gardant
l'heure murale, dans le fuseau de l'édition — l'archive se rejoue sur la COP31, ou sur aujourd'hui
pour la recette.

**Pourquoi** : SC-001 à SC-003 se prouvent en changeant de fichier ; le lecteur est éprouvé sur le
format réel, fautes et décalage compris.

**Le dépôt est public** (go du 25/09) : le jeu versionné se borne à ce que les tests exigent, son
`LISEZMOI.md` dit la source et l'accord du secrétariat en cours ; les captures brutes vivent dans
`.sources-ccnucc/`, **ignoré par Git**.

## R4 — Trier et rattacher : les données des vocabulaires

**Décision** : `reference.taxonomy_terms.metadata` porte, pour `negotiation_meeting_type`,
`source_categories` (les `typeofevent` qui l'admettent) et `denominations` (fragments de titre) ; pour
`negotiation_group`, `denominations` (noms et sigles : « G77 & China », « LMDC », « AGN »…). Chaînes
comparées après normalisation (minuscules, sans accents ni ponctuation), **en mots entiers** : « EIG » est dans « Sovereign », « LDC » dans « LLDCs ».

Résolution d'une entrée :
1. sa catégorie n'est admise par aucun type → **écartée** ;
2. le premier type dont une dénomination figure dans le titre (« informal consultation », « contact
   group », « hods drafting group », « presidency consultation »…) ; sinon le type par défaut de la
   catégorie (`plenary` pour « Plenary », `mandated_event`, `negotiation_other` pour « Negotiations ») ;
3. un type marqué `requires_title_match` (la consultation de la présidence, dans une catégorie de
   338 événements dont la plupart ne négocient rien) n'admet que les titres qui le nomment ;
4. une coordination est toujours retenue ; son groupe est le terme de `negotiation_group` que son titre nomme, sinon nul — **re-résolu à chaque lecture**, sans compter comme écart ni s'afficher comme changement : une dénomination ajoutée au vocabulaire rattache les coordinations déjà importées (retouche du go, 25/09).

Vocabulaire des types : `plenary`, `contact_group`, `informal_consultations`, `informal_informals`,
`heads_of_delegation`, `presidency_consultation`, `mandated_event`, `group_coordination`,
`negotiation_other`. `label.en` est le terme passé au lexique.

**Pourquoi** : tranché le 25/09 — jamais une liste dans le code. `metadata` existe pour cela ; une
colonne `aliases` toucherait le noyau partagé pour un seul usage.

## R5 — La règle de coupure, en un seul endroit

**Décision** : `negotiation.import_is_serving(event_id)` en SQL, lue par la route publique et par le
back-office. Servi = allumé **et** au moins une réussite **et** `missed_reads < missed_threshold`
**et** `now() − last_success_at < missed_threshold × interval_seconds + 60 s`.

**Pourquoi** : la dernière clause couvre ce que le compteur ne voit pas — un worker arrêté ne manque
aucune lecture. Sans elle, une panne du worker laisserait servir une liste de la veille comme fraîche
(principe XII).

**Lecture manquée** : source injoignable, délai (15 s), contenu illisible, **ou aucune réunion retenue** — une
source qui rend zéro session pendant une COP est en panne, pas vide ([spec, cas limites](spec.md)).
Une lecture manquée n'écrit aucune session.

## R6 — « Déplacée » se déduit, l'ENUM ne bouge pas

**Décision** : « Déplacée » = il existe un changement `start`, `end` ou `venue` ; la valeur barrée est
l'`old_value` du dernier. « En cours » / « Terminée » se déduisent de l'heure, côté client, dans le
fuseau de la COP. `status` ne porte que ce que la source affirme : `scheduled` ou `cancelled`.

**Pourquoi** : un état « déplacé » stocké se désynchroniserait de l'heure ; et `ALTER TYPE … ADD VALUE`
ne se rejoue pas dans une transaction de migration.

## R7 — Le travail d'import

**Décision** : `negotiation.import_official_sessions`, un `JobHandler` qui se **replanifie lui-même**
(patron de `jobs/purge.rs`), clé d'idempotence `import:<edition>:<créneau>`, créneau = `floor(epoch / interval_seconds)`. **« Lire maintenant »** pose un travail à la clé `import:<edition>:manuel:<request_id>`, porteur de `manuel: true`, **qui ne se replanifie pas** — la chaîne reste unique, et deux lectures manuelles de suite ont bien lieu. Il **réussit toujours**
côté file : une lecture manquée est une donnée (`import_runs`, `missed_reads`), pas un échec du
travail — sinon l'attente exponentielle de `fail_job` espacerait les lectures précisément pendant la
panne. Éteint, il ne se replanifie pas ; allumer pose la première lecture. `armer_les_recurrents()`
du worker réarme la chaîne au démarrage pour chaque import allumé.

**Écriture** : une transaction par lecture — sessions, points d'ordre du jour,
changements, compteurs, journal. Comparaison champ par champ sur la forme pivot ; une ligne sans
écart n'est pas touchée, seul `last_read_at` avance (en une requête pour toutes).

**Disparition** : `absent_reads + 1` à chaque lecture réussie où la session manque ; à 2, `cancelled`
avec `cancellation_reason = 'removed'` (tranché le 25/09). Reparue, le compteur retombe à 0 ;
annulée par disparition puis reparue, elle redevient `scheduled`, avec un changement `status`.
Un titre préfixé « CANCELLED » → `cancelled`/`source` ; « POSTPONED » → `cancelled`/`postponed`, affiché
« Annulée — reportée par la source » ; le préfixe est retiré du titre gardé.

## R8 — « Mon agenda » n'est pas une inscription

**Décision** : table `negotiation.agenda_entries`, PK `(person_id, meeting_id)`.

**Écarté** : `negotiation.meeting_registrations` — capacité, liste d'attente, compteur tenu par
déclencheur, présence : tout ce qu'une session officielle n'a pas. Y écrire ferait monter
`registered_count` de réunions que l'IFDD n'organise pas.

## R9 — Une réponse pour toute la COP

**Décision** : `GET /negotiation/sessions?edition=` rend toutes les sessions de l'édition, avec
`ETag`/`304`. Pas de route de détail.

**Pourquoi** : la COP30 compte environ 450 réunions retenues sur deux semaines, 65 au plus un jour ; la réponse
comprimée reste sous 150 Ko. Un seul aller en ligne rend **toutes** les fiches lisibles hors
connexion — mieux que FR-038, qui n'exige que les fiches ouvertes. Une route « modifié depuis » par
ligne n'apporterait rien que l'`ETag` ne donne déjà, et doublerait la règle de coupure.

## R10 — Le lexique, sans dépendre de l'étape 2

**Décision** : contrat transmis par l'orchestrateur le 25/09 — le terme anglais du type ouvre
`/guide-nego/lexique?terme=<texte anglais>` ; l'étape 2 résout ce texte sur le téléphone
(`resoudreLeTerme`, `utils/guide-nego/lexique.ts`). 3a **n'écrit ni la fonction ni la table** ; avant
la fusion de l'étape 2, l'adresse mène à un lexique vide, ce qui est admis.

**Écarté** : ouvrir la feuille basse du lecteur — ce serait réécrire en parallèle ce que dev1 remplace.

## R11 — La traduction des titres

**Décision** : un second travail, `negotiation.translate_session_titles`, posé par l'import quand une
lecture laisse des titres sans traduction. Il prend jusqu'à 40 titres, les envoie **en un appel** à
OpenRouter (`POST https://openrouter.ai/api/v1/chat/completions`, modèle lu dans le réglage
`ai.drafting_model`, réponse JSON exigée : un tableau aligné sur l'entrée), et écrit
`negotiation.title_translations`. Clé lue par `std::env::var("OPENROUTER_API_KEY")` au démarrage du
worker, dans la configuration du module, **jamais journalisée** ; absente, le travail n'est pas posé
et les titres restent en anglais seul (FR-024). Échec d'un appel : le travail finit sans écrire,
l'import suivant le reposera — pas de boucle d'essais.

Un trait `Traducteur` isole l'appel ; les tests emploient un traducteur fixe. **Aucun test n'appelle
OpenRouter** (SC-010).

**Écart à `03-api.md` l. 37**, inscrit comme exception datée du 25/09 dans ADR-004 : qui prévoyait un appel de Rust vers le service d'IA en Python : ce
service n'existe pas encore (étape 7). L'appel direct depuis le worker ne crée aucune route ni
service (principe XIV tenu) ; il se déplacera derrière le service d'IA quand celui-ci naîtra.

**Écart au principe XII** : décidé le 25/09, inscrit dans `COMMENT ON TABLE title_translations`, dans
`05-design.md` et dans le *Complexity Tracking* du plan.

## R12 — Le rappel dans l'application ouverte

**Décision** : FR-034, tranché le 25/09. Le choix vit dans `agenda_entries.remind_before` (il suit le
compte, 3b l'emploiera pour la notification). Le bandeau est calculé côté client depuis la garde ;
il marche hors connexion.

**Écarté** : `engagement.scheduled_reminders` — il vise `programme.sessions` (le Pavillon) et
enverrait par courriel ou notification, hors périmètre.

## R13 — L'écran « Mes thématiques » porte les groupes

**Décision** : un second bloc « Mon groupe de négociation » sous les thématiques, mêmes cases, même
file ; route sœur `/negotiation/me/groups`, table sœur `group_subscriptions`. La garde de première
entrée de 0c ne change pas : les groupes sont facultatifs.

**Écarté** : étendre `PUT /negotiation/me/themes` à deux listes — l'empreinte et le `412` de 0c
couvriraient deux choix indépendants, et un groupe changé sur un téléphone rendrait périmé un choix
de thématiques fait sur un autre.

## R14 — Le fuseau

**Décision** : l'édition servie à la coquille gagne `timezone` ; toute heure de session passe par
`formatTime`/`timeRangeParts` de `utils/datetime.ts` avec ce fuseau et « heure de <ville> »
(`zoneElides()` pour l'élision). « lu à » reste à l'heure du téléphone sans fuseau (écart 32).
