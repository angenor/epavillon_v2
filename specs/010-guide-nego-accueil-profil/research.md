# Recherche — étape 0c : thématiques, « Ma journée » et profil

**Date** : 2026-09-22 · **Spécification** : [spec.md](spec.md)

Ce document tranche ce que la spécification a laissé ouvert, et consigne ce qui a été vérifié
dans le code et dans le modèle avant d'écrire une ligne. Chaque décision porte son *pourquoi*,
et ce qui a été écarté.

---

## R1 — Où vit le lien entre une personne et ses thématiques

**Décision : une table propre, `negotiation.theme_subscriptions`, sur le patron exact de
`negotiation.network_memberships`.** Ni `identity.negotiator_profiles.specializations`, ni
`reference.entity_terms`.

**Pourquoi.**

`identity.negotiator_profiles` (`030_identity.sql:447`) n'a **de ligne pour personne** : ni le semis,
ni le backend — zéro occurrence de `negotiator_profiles` dans `backend/crates/` —, ni aucun trigger
n'en crée. Une personne admise par code d'invitation, avec le rôle `negotiator`, n'y figure pas.
Y ranger « mes thématiques » fabriquerait donc **un profil de négociateur désigné à chaque premier
clic sur une case à cocher**, et `negotiation.space_members.negotiator_profile_id` se mettrait à
pointer vers des profils vides nés d'un geste d'écran.

Les deux notions sont d'ailleurs inverses : `specializations` décrit une **compétence attestée**,
renseignée par un administrateur à côté des désignations annuelles (`negotiator_designations`, un acte
administratif : « untel est désigné pour la filière climat en 2026, par untel ») ; 0c demande une
**préférence de lecture**, choisie et défaite librement. Une vérité, pas deux : ce sont deux notions,
pas deux rangements d'une même notion.

Enfin `specializations` est un `text[]` sans `COMMENT ON COLUMN`, sans index, sans contrainte de
vocabulaire — c'est **exactement le défaut que la v2 a corrigé**, et le modèle le dit deux fois :
« ni à empiler des colonnes tableau — `main_themes activity_theme[]` en v1, qui n'offrait aucune
intégrité référentielle » (`020_reference.sql:172`), et le `jsonb` informe des préférences de
notification de la v1, « non requêtable » (`110_engagement.sql:134`). Rien n'y empêcherait d'écrire un
code d'`activity_theme` ou une faute de frappe, et rien ne dirait depuis quand une thématique est suivie.

**`reference.entity_terms` écarté aussi**, bien que tentant — zéro DDL, `terms_of` et `term_badges`
gratuits. Quatre raisons : `reference` est le noyau partagé, et partout ailleurs `entity_terms` classe
un **contenu** (proposition, article, session), jamais une personne ; `entity_id` n'a aucune intégrité
référentielle et `platform.purge_term_links()`, promise en commentaire (`020_reference.sql:177`),
**n'existe pas** — la suppression d'une personne laisserait des lignes orphelines que l'anonymisation
RGPD ne nettoierait pas davantage ; la table n'est **pas auditée** (aucun `tg_audit` dans `020_reference.sql`),
on ne saurait jamais qui a retiré un suivi ; et sa clé primaire impose une colonne `role` sans objet ici,
sans laisser de place pour la fin d'un suivi ni pour ce dont 3a et 3b auront besoin.

**Ce que la table propre apporte** : une ligne = un suivi, indexable et joignable ; l'index inverse
`(theme_term_id, …)` répond à « qui suit la finance ? », dont 3b a besoin pour ses alertes ; le trigger
de garde de vocabulaire **existe déjà et vit dans le bon schéma** — `negotiation.tg_check_term_taxonomy`
(`100_negotiations.sql:63`), employé six fois dans le module ; `ON DELETE RESTRICT` vers le terme
protège la dépréciation ; et l'absence de ligne vaut « aucune thématique suivie », repli naturel et
silencieux, comme pour `engagement.notification_preferences`.

**Conséquence de frontière** : la table vit dans le schéma `negotiation`, donc la clé étrangère vers
`identity.people` porte le préfixe obligatoire `xmod_fk_`. La clé vers `reference.taxonomy_terms` n'en
porte pas : `reference` est noyau partagé et le CTE `shared_kernel` de `platform.cross_module_fk_report`
l'exempte (`000_bootstrap.sql:342`).

---

## R2 — Comment la liste des thématiques est lue

**Décision : aucune route nouvelle. La route publique des termes de taxonomie existe déjà.**

`GET /reference/taxonomies/{code}/terms` (`backend/crates/api/src/routes/reference.rs:154`,
`operation_id = "reference_termes"`) rend les termes **actifs** d'un vocabulaire, triés par `sort_order`
puis `code`, libellés multilingues compris, sans session — et « une taxonomie inconnue rend une liste
vide, pas 404 ». Semer `negotiation_theme` suffit à la servir.

Côté client, `api.reference.terms(...)` existe déjà dans `useApi.ts`. La règle « réutiliser avant
d'écrire » s'applique ici à la lettre : une seconde fabrique qui rendrait les mêmes termes serait un
contrat de trop. Et la contrainte XIII ne l'interdit pas — elle borne les **composants et les jetons**
du site, pas les méthodes d'API partagées.

**Écarté** : une route `/negotiation/themes` propre au module. Elle n'aurait rien ajouté, et aurait
figé dans `negotiation` un vocabulaire qui appartient au référentiel.

---

## R3 — La forme des routes d'écriture

**Décision : deux routes, et un remplacement en bloc.**

- `GET /negotiation/me/themes` — ce que je suis, avec l'empreinte du contenu.
- `PUT /negotiation/me/themes` — **la liste entière**, jamais un ajout ou un retrait unitaire.

**Pourquoi le remplacement en bloc.** L'écran valide une liste, pas une suite de gestes : « Suivre ces
thématiques » est un seul acte. Surtout, un `PUT` de la liste complète est **idempotent** — le rejouer
donne le même état —, et c'est précisément ce qui rend sûre l'écriture différée de R5 : une file qui
repart au retour du réseau ne peut pas produire un double effet. Un `POST /themes/{id}` suivi d'un
`DELETE /themes/{id}` rejoués dans le désordre, si.

Le patron d'écriture existe : `programme/src/repo/themes.rs` retire puis réinsère dans une même
transaction, et **nomme le code refusé** plutôt que de dire « une thématique est inconnue ». On le suit.

**Le suivi se ferme, il ne se supprime pas** : retirer une thématique pose `left_at`, comme une
appartenance de réseau. L'index unique partiel `WHERE left_at IS NULL` tient l'unicité du suivi vivant,
et l'audit garde la trace de ce qui a été retiré.

---

## R4 — « Modifié depuis » ou empreinte ?

**Décision : une empreinte de contenu (`ETag` fort) et `If-None-Match`, comme à l'étape 0b. Pas de
paramètre « modifié depuis » à cette étape.**

`03-api.md:9` demande que « chaque liste accepte "modifié depuis", réponde avec une empreinte et
l'heure du serveur ». Vérification faite : **rien ne l'implémente aujourd'hui** — aucun paramètre de ce
nom dans aucune route du dépôt, et aucune réponse ne porte l'heure du serveur. Le seul précédent
exécutable est l'`ETag` de `GET /negotiation/me/access` (`routes/acces.rs:61`), dont le commentaire
tranche déjà le débat : l'état vient de plusieurs tables et de `now()`, aucune colonne ne porte l'instant
où il a changé, et **un 304 fautif laisserait des données périmées sur le téléphone**.

Les listes de 0c sont minuscules — dix thématiques, quelques suivis : elles se lisent d'un bloc, comme
le lexique et la FAQ que l'ADR-003 fait garder en entier. Le « modifié depuis » se justifiera à l'étape 1,
sur les documents, où le volume le demande ; l'inventer ici, sans le besoin qui le valide, serait une
forme vide.

L'empreinte se calcule exactement comme en 0b : `kernel::crypto::token_hash` du corps sérialisé,
16 octets en hexadécimal, entre guillemets.

---

## R5 — L'écriture différée hors connexion

**Décision : elle n'existe pas, et 0c la construit — bornée à ce qu'elle sert.**

Constat vérifié : `utils/guide-nego/garde.ts` n'a qu'un magasin, `lectures` ; `useGnLecture` ne traite
que la lecture ; `useGnConnexion` n'écoute que `offline`, jamais `online` ; `api/http.ts` pose
`retry: 0` sur toute écriture, délibérément. La seule reprise existante est
`relireAuRetourAuPremierPlan()`, qui **relit** et ne rejoue rien.

Or la spécification l'exige deux fois (FR-009, FR-039), la constitution aussi (principe XI : « une
écriture faite sans réseau n'est ni refusée ni perdue »), et `01-stack.md:41` range les thématiques
suivies parmi ce qui est « toujours » disponible hors connexion et « envoyé au retour du réseau ».

**Ce qui se construit** : un second magasin IndexedDB, `ecritures`, une file à **une seule entrée par
clé** — une nouvelle intention de la même clé remplace la précédente, puisque le `PUT` porte l'état
entier —, un départ déclenché par l'événement `online` et par le retour au premier plan, et un retrait
de la file uniquement après succès. L'idempotence vient de la forme de la route (R3), pas d'un jeton.

**Ce qui ne s'y met pas** : la doctrine de 0b reste entière — un accès ne se met pas en file, on ne peut
pas annoncer un droit avant de l'avoir obtenu (FR-019 de 0b). La file est réservée à ce dont la personne
est elle-même l'autorité : ses thématiques. Un accord de consentement l'y rejoindra à l'étape qui le
livrera.

---

## R6 — Les textes qui engagent

**Décision : les fichiers vivent dans `kernel`, la route publique est servie par `api`, et le réglage
d'environnement disparaît.**

Le commanditaire a tranché la forme : un fichier Markdown `fr`/`en` par texte, portant sa version en
tête, embarqué dans l'API et servi par une route publique, la version servie remplaçant
`PRIVACY_POLICY_VERSION`. Restait à dire **où**, et la frontière de modules l'impose :

- `programme` consomme la version à l'inscription (`service/registration.rs:122`) et **ne peut dépendre
  d'aucun autre crate de module** (principe II). Si les textes vivaient dans un module, la version lui
  serait inatteignable.
- `kernel` est le seul crate dont tous les modules dépendent. Les fichiers y sont donc embarqués par
  `include_str!`, et `kernel` expose le texte, sa langue et sa version.
- La **route publique** ne relève d'aucun module métier : elle se pose dans `crates/api/src/routes/`,
  à côté de `reference.rs`, qui est le précédent exact d'une route publique n'appartenant à personne.

**Ce sera la première route du dépôt à servir un contenu embarqué** — vérifié : aucun `include_str!` de
contenu, aucune dépendance d'embarquement, et aucun `text/*` servi nulle part. Le patron se compose de
deux précédents : le corps précalculé au montage de `/docs` (`api/src/openapi.rs`) et l'empreinte de
contenu de `routes/acces.rs`.

**Le contrôle qui refuse un texte modifié sans nouvelle version** : un test garde, à côté des fichiers,
l'empreinte attendue de chaque texte pour sa version déclarée. Modifier le texte sans toucher la version
fait diverger l'empreinte et **échoue à la compilation des tests**. C'est volontairement pénible : une
preuve de consentement qui nomme une version dont le contenu a bougé n'oppose plus rien.

**Ce qui change côté configuration** : `PRIVACY_POLICY_VERSION` sort de `.env.example`, le champ brut,
son défaut et sa validation sortent de `kernel/src/config.rs`, et `ProgrammeConfig.privacy_policy_version`
disparaît. `registration.rs:122` lit désormais la version depuis `kernel`. **Une ligne changée chez
l'appelant**, et la doctrine inscrite en commentaire de `programme/src/repo/consents.rs:26` est réécrite
— elle disait que la version vient de la configuration, ce qui cesse d'être vrai.

**Ce qui ne bouge pas** : la signature de `repo::consents::accorder(conn, person_id, policy_version, ip)`,
celle d'`exiger_le_consentement`, la constante `FINALITE`, la source `registration_form`, le code
d'erreur `REGISTRATION_CONSENT_REQUIRED`, et le fait que la preuve s'écrive dans la transaction de
l'inscription. Les lignes de consentement déjà écrites sous `2026-01` **ne sont pas réécrites** :
`identity.consents` est un historique.

**Les licences ne sont pas un texte qui engage** : elles disent la police et les bibliothèques
embarquées, n'appellent aucun accord et n'ont pas de version opposable. Elles se tiennent avec la
construction, dans les traductions de l'écran.

---

## R7 — Le rendu d'un texte long

**Décision : la route sert le Markdown, le client le rend avec une grammaire close, sans dépendance
nouvelle.**

Le texte vient du binaire de l'API, écrit par l'IFDD : ce n'est pas un contenu hostile, et la grammaire
employée est connue. Le composant de rendu couvre ce que les textes emploient — titres de niveau 2 et 3,
paragraphes, listes, liens, emphase — échappe tout le reste, et se teste sans navigateur. Ajouter une
bibliothèque de rendu Markdown au client pour trois textes serait une dépendance d'ampleur au sens de la
constitution, pour un gain nul.

**Et un test rend les fichiers réels avec cette grammaire, et échoue sur ce qu'elle ne couvre pas.**
Les textes seront écrits par quelqu'un qui n'a aucune raison d'en connaître les bornes : un tableau ou
une note de bas de page rendraient du charabia sans prévenir. Le test échoue à la place. C'est ce qui
rend le rendu maison tenable : il n'a pas à tout couvrir, il doit dire ce qu'il ne couvre pas.

---

## R8 — La place occupée sur l'appareil

**Décision : `navigator.storage.estimate()`, avec un repli qui le dit.**

Rien ne mesure quoi que ce soit aujourd'hui : aucune occurrence de `navigator.storage` dans le front, et
le seul parcours possible du stockage est `lireToutesLesGardes()`, qui ne pèse rien. L'estimation du
navigateur est la seule mesure qui compte l'ensemble — coquille, données lues, documents à venir. Elle
n'est pas disponible partout : dans ce cas l'écran dit qu'il ne peut pas la mesurer sur cet appareil,
plutôt que d'annoncer un zéro faux.

**« Libérer la place » ne vide que ce qui se relit** : les données lues et, à partir de l'étape 1, les
documents téléchargés. **La coquille reste** — c'est elle qui permet d'ouvrir l'application sans réseau,
et la vider retirerait à la personne exactement ce qu'elle est venue chercher en salle. La confirmation
le dit en toutes lettres.

---

## R9 — L'avatar dans l'en-tête

**Décision : une prop sur `GnEcran`, relayée à `GnEntete`, et un composant `GnAvatar` nouveau.**

Aucune page n'instancie `GnEntete` directement : toutes passent par `GnEcran`. Un slot seul ne suffirait
donc pas — il faudrait le relayer, ce qui touche les deux fichiers de toute façon. Une prop est plus
simple à lire et se teste mieux.

L'avatar et le bouton retour occupent **la même place** à gauche de la barre : ils sont exclusifs, et le
retour l'emporte quand il est présent. « Ma journée » n'a pas de retour — c'est un écran racine —, donc
aucun conflit en pratique.

`GnAvatar` suit `composants.md` : 40 px, rayon 24, initiales en 15/700. L'image du compte de l'ePavillon
quand elle existe, les initiales sinon.

---

## R10 — La sortie du bloc `auth` de `useApi.ts`

**Décision : oui, en préalable, et avec `deps`.**

`useApi.ts` est à **973 lignes** pour un garde-fou à 1000. Le bloc `auth` est déjà une constante
(lignes 272 à 322, plus son commentaire depuis la ligne 253) : environ 70 lignes déplaçables, soit près
de cent lignes de marge retrouvées. Aucun des **14 appelants répartis dans 7 fichiers** ne change :
`auth` reste exposé sous `api.auth.*`, il suffit que la constante soit construite avant le littéral de
retour, puisque `createGuideNegoApi({ auth, … })` en a besoin.

La fabrique reçoit **`deps`**, comme les treize autres blocs, et non le motif à primitives restreintes de
`guide-nego.ts` : `checkPasswordResetToken` passe un troisième argument à `call` que ce motif ne déclare
pas. Deux conventions coexistent déjà dans `composables/api/` ; on prend la majoritaire, qui couvre le cas.

Les nouvelles méthodes de 0c, elles, vont dans `composables/api/guide-nego.ts`, jamais dans `useApi.ts`.

---

## R11 — Le fuseau du sous-titre de « Ma journée »

**Décision, révisée par le commanditaire le 22/09 : à cette étape, le sous-titre ne porte que le jour —
« Jeudi 12 novembre » —, sans fuseau nommé. L'étape 3a apportera celui du lieu de l'édition.**

*Pourquoi pas le fuseau de l'appareil, d'abord retenu* : nommé, il trompe. Son identifiant dit
« Istanbul » à Antalya, « Lome » et « Ndjamena » sans accent, « Douala » pour Yaoundé ; le nom long
du moteur dit « heure moyenne de Greenwich » à Dakar. En 3a, le nom viendra du lieu de l'édition,
selon la règle du site : `edition.city` d'abord (`composables/useDateTime.ts`).

**« Aujourd'hui » se recalcule au retour au premier plan** : une application restée en mémoire ne doit
pas ouvrir le lendemain sur la journée de la veille.

La spécification prévoit le fuseau de l'édition en cours « quand il y en a une ». Vérification faite,
il n'existe **ni fonction ni vue d'édition courante** : `event.v_public_editions` porte un
`temporal_state` calculé (`upcoming` / `ongoing` / `past`) et rien ne garantit l'unicité d'une édition
en cours. Or 0c n'affiche **aucune heure d'événement** — tous les blocs sont vides. Lire une édition
pour n'en tirer qu'un mot de sous-titre ajouterait une route et une ambiguïté sans rien servir.

L'étape 3a, qui apporte les heures de sessions, apportera le fuseau qui va avec. Les heures de lecture,
elles, n'en portent jamais (écart 32).

---

## R12 — La première entrée, sans enfermer personne

**Décision : l'écran des thématiques est proposé une fois, et n'enferme pas.**

`pages/guide-nego/index.vue` redirige déjà vers l'ouverture tant qu'elle n'a pas été vue, par une clé
locale. 0c ajoute la même mécanique pour les thématiques : proposées une fois à une personne connectée
qui n'en suit aucune, puis jamais reproposées d'elles-mêmes — la ligne du profil reste le chemin.

Sans cela, une personne qui ne veut pas choisir tout de suite serait renvoyée à cet écran à chaque
ouverture, alors que la règle « au moins une thématique » ne porte que sur **l'enregistrement d'un
choix**, pas sur le droit d'entrer.

---

## R13 — Les dix thématiques, et leurs codes

**Décision : codes naturels, distincts par leur vocabulaire et non par leur orthographe.**

Cinq des dix codes existent déjà sous `activity_theme` — `adaptation`, `gender`, `transparency`,
`loss_and_damage`, `climate_finance`. La clé est `(taxonomy_code, code)` : la coexistence est légale, et
tout diagnostic passe de toute façon par le vocabulaire. Déformer les codes pour éviter une homonymie
rendrait la correspondance future plus difficile, pas plus claire.

| Code | Français | Anglais | Ordre |
|---|---|---|---|
| `adaptation` | Adaptation | Adaptation | 10 |
| `mitigation` | Atténuation | Mitigation | 20 |
| `finance` | Finance | Finance | 30 |
| `loss_and_damage` | Pertes et préjudices | Loss and damage | 40 |
| `article_6` | Article 6 | Article 6 | 50 |
| `transparency` | Transparence | Transparency | 60 |
| `gender` | Genre | Gender | 70 |
| `just_transition` | Transition juste | Just transition | 80 |
| `agriculture` | Agriculture | Agriculture | 90 |
| `technology` | Technologie | Technology | 100 |

L'ordre est celui de la maquette, par pas de dix, comme tous les vocabulaires du modèle. Le vocabulaire
est déclaré `is_multi_select = true` — on en suit plusieurs —, `is_hierarchical = false` et
`is_system = true` : il commande une règle métier, les alertes de 3b, et aucun écran d'administration ne
le modifie.

---

## R14 — Ce que cette étape ne met pas au back-office

**Décision : aucune route `/admin` n'est livrée.**

Le prompt de plan commun prévoit l'administration sous `/admin` avec filtrage par périmètre. Ici, il n'y
a rien à administrer : le vocabulaire se sème par le SQL, comme tous les vocabulaires du site — le
commanditaire l'a dit —, les suivis appartiennent à la personne, et les textes se changent par un
fichier. Le premier écran d'administration viendra quand l'IFDD voudra ajouter une thématique sans
passer par le SQL.

---

## R15 — Recharger la base sans la détruire

**Décision : un script de migration rejouable, comme à l'étape 0b.**

`docs/database/` n'est chargé qu'à la création du volume. Toute table nouvelle se double donc d'un
script rejouable — `CREATE TABLE IF NOT EXISTS`, `INSERT … ON CONFLICT DO NOTHING`, gardes sur les
triggers — vérifié en comparant les schémas après coup, exactement comme `specs/009-…/migration.sql`.
La modification de `docs/database/` se consigne dans `docs/progression/modele.md` : le modèle est commun
aux deux applications, c'est la seule exception à la règle de suivi de Guide Négo.

`make check` et `make check-db` détruisent la base locale et ne se lancent pas.

---

## R16 — Deux appareils, et un choix qui arrive en retard

**Décision : une intention porte l'empreinte de l'état sur lequel elle a été prise, l'envoie en
`If-Match`, et l'API rend `412` si l'état a changé.**

L'idempotence de R3 protège du **rejeu** — envoyer deux fois la même chose ne fait pas deux fois l'effet.
Elle ne protège pas de l'**ancienneté**, et le scénario tient en trois lignes :

> Téléphone sans réseau à 10:00, la personne choisit *adaptation* et *genre* : l'intention part en file.
> Tablette en ligne à 11:00, elle choisit *finance*. Téléphone de retour à 12:00 : l'intention de 10:00
> part et **efface le choix de 11:00**, sans que personne ne le voie.

`If-Match` ferme cela. Un `PUT` **sans** `If-Match` reste accepté : l'écran en ligne vient de lire
l'état, il n'a rien à opposer. Sur `412`, l'application **abandonne** l'intention, relit, et le dit —
« Vos thématiques ont changé sur un autre appareil ». Jamais de fusion : la liste qu'une fusion
produirait n'aurait été choisie par personne.

**L'empreinte se calcule sur les codes suivis, triés** — jamais sur le corps rendu. Deux appareils de la
même personne, l'un en français l'autre en anglais, auraient sinon deux empreintes pour un même état.

**Conséquence non prévue, et heureuse** : le corps rendu **ne peut donc pas porter de libellés**. Si
l'empreinte les ignorait mais que le corps les portait, un appareil qui change de langue recevrait un
`304` et garderait les anciens libellés. Le corps ne rend donc que des codes — le client a déjà le
vocabulaire par la route publique des termes, qu'il lit de toute façon. Une source de libellés au lieu
de deux, et un contrat plus court.

**Trois règles de file qui en découlent**, écrites pour tenir au-delà des thématiques, puisque le
parcours (étape 2) et les signalements (3b) emploieront la même file :

1. Une entrée porte **l'identifiant de la personne** qui l'a prise, et ne part jamais sous un autre
   compte — un téléphone se prête, au stand.
2. La file **se vide à la déconnexion**.
3. Elle part **aussi à l'ouverture de l'application**, pas seulement sur `online` et au retour au
   premier plan : un téléphone rouvert le lendemain n'émet pas d'`online`.

---

## Deux dettes trouvées en chemin, hors périmètre

Elles ne bloquent pas 0c et ne s'y corrigent pas, mais elles sont notées pour ne pas se reperdre :

1. **`platform.purge_term_links()` n'existe pas**, alors que `020_reference.sql:177` la donne comme le
   mécanisme de nettoyage des rattachements orphelins de `reference.entity_terms`.
2. **`reference.terms_of` ne filtre pas `is_active`** quand `reference.term_badges` le fait : un terme
   déprécié continue de sortir dans les codes mais disparaît des pastilles.
