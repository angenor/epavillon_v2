# Recherche — 0b, compte et admission

> Ce que la lecture du dépôt a tranché avant d'écrire une ligne de code. Chaque point donne la
> décision, sa raison, et ce qui a été écarté.

---

## R1 — Le crate `negotiation` naît à cette étape

**Constat** : `backend/crates/modules/` porte `analytics`, `content`, `engagement`, `event`,
`identity`, `live`, `media`, `org`, `programme`. **Il n'y a pas de `negotiation`.** Le socle l'attend
pourtant déjà : `ScopeType::NegotiationSpace` existe dans `kernel/src/auth.rs`, `identity` lit
`negotiation.spaces` dans `repo/rbac.rs`, `media` a une garde `("negotiation","documents")`, et la
ligne `('negotiation','negotiation',…)` est au registre `platform.modules` de `010_platform.sql`.

**Décision** : 0b crée le crate, calqué sur `identity`, avec **pour seules dépendances `kernel` et
`contracts`** (principe II). Fichiers à toucher, tous connus :

| Fichier | Ce qu'on y fait |
|---|---|
| `backend/Cargo.toml` | membre `crates/modules/negotiation` + alias dans `[workspace.dependencies]` |
| `backend/crates/modules/negotiation/Cargo.toml` | nouveau, calqué sur celui d'`identity` |
| `backend/crates/modules/negotiation/src/lib.rs` | `routes(cfg)`, `admin_routes(cfg)`, `job_handlers(...)`, `NegotiationState` |
| `backend/crates/api/Cargo.toml`, `crates/worker/Cargo.toml` | dépendance vers le crate |
| `backend/crates/api/src/state.rs` | champ `negotiation` et sa construction |
| `backend/crates/api/src/lib.rs` | `if etat.modules.is_mounted("negotiation") { … }` et `app_data` |
| `backend/crates/api/src/modules.rs` | le code dans la liste figée de l'export OpenAPI |
| `backend/crates/api/src/openapi.rs` | `doc.merge(negotiation::routes::openapi::NegotiationApi::openapi())` |
| `backend/crates/worker/src/main.rs` | `negotiation::job_handlers(...)` et la purge récurrente |
| `backend/crates/contracts/src/negotiation.rs` + `lib.rs` | les noms d'événements et leurs charges |

**Écarté** : loger ces routes dans `identity`. L'admission est du métier de négociation — ses tables
vivent dans le schéma `negotiation`, et un crate ne lit que son schéma.

---

## R2 — Comment la session sait qu'elle vient de l'application

**Constat** : la session passe par **deux cookies `HttpOnly`** posés par l'API — `epavillon_at`
(accès, `Path=/`) et `epavillon_rt` (rafraîchissement, `Path=/api/auth`, `SameSite=Strict`). L'API
**ne lit aucun en-tête `Authorization`**. À la connexion, `routes/auth.rs` extrait le `user_agent` et
l'adresse, les passe en `Device` à `session::open`, qui insère dans `identity.sessions`.

**Décision** : le client envoie un objet `client` **facultatif** dans le corps de
`POST /api/auth/login` et `POST /api/auth/register` :

```json
{ "client": { "kind": "app", "device_id": "…", "label": "Android · Chrome", "platform": "android" } }
```

Absent, il vaut `web` — le site n'est pas touché. À la **rotation** (`POST /api/auth/refresh`), le
client ne le redit pas : `session::open` recopie le `client_kind` et l'appareil de la session qu'elle
remplace. Sans cela, toute session de l'application redeviendrait « site » au premier renouvellement,
et les chiffres d'usage seraient faux au bout d'une heure.

Le `device_id` est engendré **une fois** par le client et gardé dans le stockage local de Guide Négo
(`utils/guide-nego/stockage.ts`, nouvelle clé `gn.appareil`) — jamais un cookie, que le garde-fou
interdit à Guide Négo.

**Il n'accorde aucun droit.** Un client peut le forger, comme il forge son `user_agent` : il sert à
nommer une session dans une liste et à compter les essais de code, rien d'autre.

**Écarté** : déduire le client de l'origine ou du chemin (`/guide-nego/…`) — la connexion est un appel
d'API, elle ne connaît pas la page appelante de façon fiable. Écarté aussi : une session par jeton
dès maintenant — ADR-002 la réserve à la coquille Capacitor, et rien ne l'exige avant.

---

## R3 — Les refus d'un code sortent en 200, avec leur message

**Constat** : `routes/auth.rs` porte déjà la règle en tête de fichier — « un refus prévu par le
contrat du site sort en 200 avec son discriminant ». La connexion le fait pour `mfa_required`.
Le catalogue d'erreurs (`kernel/src/error.rs`, macro `codes!`) porte code stable + statut + message
français.

**Décision** : `POST /api/negotiation/invitation-codes/redeem` répond **200** avec une union :

```
issue = "accepted" | "pending_approval" | "unknown" | "revoked" | "exhausted"
      | "expired" | "not_yet_valid" | "throttled" | "already_granted"
```

et, **dans la même réponse, le message français composé par l'API** — c'est elle qui connaît la date
de révocation et le temps d'attente restant. Le client l'affiche **tel quel** (FR-020) et n'écrit
aucun second texte ; les titres, les aides et les boutons autour restent de l'i18n.

Les champs utiles accompagnent le message : `revoked_at`, `retry_after_seconds`, `granted.scope`,
`granted.network`.

Restent de vraies erreurs, avec leur code stable : non connecté, adresse non vérifiée, compte
suspendu, corps malformé.

**Raison** : six refus prévus par le parcours ne sont pas six pannes. En 4xx, le transport du front
les traiterait comme des erreurs et l'écran perdrait ses deux sorties — or la spécification exige
qu'**aucune issue ne laisse l'écran sans suite** (FR-015).

**Nouveaux codes d'erreur** à ajouter à la macro `codes!` : `NEGOTIATION_ACCESS_REQUEST_PENDING`,
`NEGOTIATION_ACCESS_REQUEST_DECIDED`, `NEGOTIATION_INVITATION_CODE_DUPLICATE`,
`NEGOTIATION_ADMISSION_MODE_INVALID`, `NEGOTIATION_SPACE_UNKNOWN`. Chacun traduit un invariant de la
base (§ 7 du modèle de données), aucun ne redouble une vérification déjà faite par PostgreSQL.

---

## R4 — Le mode d'admission se lit à chaque tentative

**Constat** : `platform.settings` est une table clé/valeur `jsonb` lue par l'API ;
`platform.feature_flags` n'ouvre et ne ferme qu'en binaire.

**Décision** : le mode vit dans `platform.settings`, clé `negotiation.admission_mode`, et il est
**relu à chaque tentative d'entrée** — aucun cache en mémoire de processus. C'est ce qui rend vrai
SC-002 : l'administrateur bascule, et la personne suivante le voit, sans mise en ligne ni
redémarrage. Le coût est une lecture indexée par clé primaire sur un geste rare.

**Écarté** : un drapeau de fonctionnalité — il n'a que deux états, et il en faut trois. Écarté aussi
un cache de quelques secondes : il ferait mentir le critère de sortie pour gagner une lecture.

---

## R5 — « Mon accès » se lit hors connexion sans rien inventer

**Constat** : l'étape 0a a livré `useGnLecture` — lecture réseau, repli sur la garde IndexedDB,
`luA`, `source`, `pret` — et `useGnConnexion` pour l'état du réseau.

**Décision** : `GET /api/negotiation/me/access` rend, en une fois, l'état d'accès, sa portée, sa date,
l'appartenance au réseau, la demande en attente s'il y en a une, et le mode d'admission courant.
L'écran « Mon accès », le verrou et le parcours d'entrée lisent **cette seule route**, à travers
`useGnLecture` : hors connexion, ils affichent ce qui a été lu avec son heure (FR-034, principe XI),
sans qu'aucun écran n'ait à gérer le hors-connexion lui-même.

La réponse porte un `ETag` et accepte « modifié depuis », comme toutes les listes de Guide Négo.

---

## R6 — Le courriel de décision ne traverse aucun module

**Constat** : `negotiation` ne peut pas dépendre d'`identity` (principe II), et les modèles de
courriel d'`identity` vivent dans son crate (`mail.rs`, `jobs/emails.rs`, montés par
`identity::job_handlers`). `registration.rs` montre le chemin exact : dans **une même transaction**,
il émet son événement de domaine **et** met le courriel en file par `jobs::enqueue`.

**Décision** : le crate `negotiation` porte ses propres `src/mail.rs` et `src/jobs/emails.rs`, avec
deux modèles — demande admise, demande refusée —, montés par `negotiation::job_handlers(...)` dans le
worker. La décision écrit l'état, appelle `platform.emit_event()` et met le courriel en file **dans la
transaction**. Si elle échoue, rien ne part ; si elle réussit, le courriel part — FR-028 et SC-011
sont tenus par la transaction, pas par du code défensif.

**Écarté** : appeler `identity` pour envoyer. C'est exactement la dépendance croisée que le principe
II interdit, et elle rendrait `negotiation` intestable seul.

---

## R7 — La limitation d'essais se fait en base, pas dans Valkey

**Constat** : `01-stack.md` annonçait Valkey pour « les essais de code d'invitation ». **Valkey n'est
câblé nulle part dans `backend/`** — aucune dépendance `redis` dans aucun `Cargo.toml`, aucun
`redis::` dans le code. L'y introduire est une dépendance d'ampleur, qui exige une décision écrite
dans `docs/progression/decisions/`.

**Décision** : une table, `negotiation.invitation_code_attempts`, avec une fonction de comptage sur
fenêtre glissante et une purge par chaîne récurrente du worker. Le seuil et la fenêtre sont des
réglages de `platform.settings`, pas des constantes.

**Le compte se fait par personne, et par personne seulement.** `device_id` vient du corps de la
requête : il se forge. Compter « par personne **et** par appareil » suffirait à changer d'identifiant
à chaque essai pour que la limite ne limite rien. La fonction `invitation_attempts_recent(person,
window)` ne prend **délibérément pas** d'appareil en argument : ce qui n'est pas passé ne peut pas
être contourné. L'appareil reste enregistré comme information, utile à qui lit une série d'échecs.

**Durée de garde : 90 jours**, inscrite au commentaire de la table. Assez pour enquêter sur une série
d'échecs, trop court pour que la table devienne un journal de fréquentation.

**Ce qu'on y gagne** : aucune dépendance nouvelle, un compteur qui survit à un redémarrage, et la
trace d'un essai de force brute visible par un administrateur. **Ce qu'on y perd** : une écriture par
essai raté — sur un geste rare, fait par une personne connectée, c'est sans effet.

**Le code essayé n'est jamais stocké**, ni en clair ni en empreinte : un essai raté peut être le vrai
code d'un autre espace, et cette table est lisible au back-office.

À noter dans `docs/AppNego/01-stack.md` : la ligne Valkey ne vaut plus que pour la fraîcheur de
l'import.

---

## R8 — Le verrou est le seul composant qui manque

**Constat** : les vingt-deux composants de la coquille sont livrés. La planche de design déclare
elle-même ses absents, et l'un d'eux concerne 0b : **le verrou d'un module réservé**. Le pictogramme
`lock` existe déjà dans le sprite, employé par `pages/guide-nego/fermee.vue`.

**Décision** : 0b crée `GnVerrou` — titre du module, mention « Réservé aux négociatrices et
négociateurs », liste de ce qui s'y trouve, rappel de ce qui reste ouvert, deux sorties. Il s'ajoute à
la planche et **se retire de la liste des absents**, comme chaque étape doit le faire.

Tout le reste se compose avec l'existant : `GnEcran`, `GnEntete`, `GnChamp`, `GnBouton`,
`GnLigneReglage`, `GnEtatErreur`, `GnChargement`, `GnMarqueEtat`, `GnLigneInformation`,
`GnMessageEphemere`. **Rien n'est redessiné.**

---

## R9 — L'état de session : un seul, celui du site

**Constat** : `check-guide-nego.mjs` interdit, dans les fichiers de Guide Négo, tout cookie nommé
`epavillon_*` et tout composant du site. Or la session **est** portée par des cookies du site, et le
store `stores/auth.ts` lit un cookie témoin.

**Décision** : Guide Négo **réutilise le store `auth` du site**. Il n'est ni un composant ni un jeton
de design : la session et les droits sont explicitement ce qui se réutilise de l'ePavillon
(`01-stack.md`), et deux états de session donneraient deux vérités sur une même personne. Un
composable mince, `useGnSession()`, l'enveloppe pour ce dont l'application a besoin — un état lisible
hors connexion, sans jamais nommer un cookie.

Le garde-fou reste vert : aucun fichier de Guide Négo ne nomme de cookie ni n'importe de composant.
La borne du principe XIII porte sur le **design**, pas sur la session — et le script vérifie
exactement cela.

**À poser dans le stockage local** : une marque non sensible (`gn.session-connue`) qui dit
« quelqu'un était connecté ici », pour choisir le bon écran à l'ouverture hors connexion. **Elle
n'accorde rien** : tout droit se revérifie auprès de l'API dès le réseau revenu.

---

## R10 — Le préfixe `/v2`, encore

`APP_PUBLIC_URL` commande tout. Deux points à éprouver, pas à supposer :

- le cookie de rafraîchissement porte `Path=/api/auth` — sous préfixe, le chemin doit suivre. Le test
  `identity/tests/cohabitation_sous_prefixe.rs` existe déjà : l'étendre plutôt qu'en écrire un autre ;
- les nouveaux écrans sont sous `pages/guide-nego/`, donc **gardés automatiquement** par le service
  worker : `modules/guide-nego-garde.ts` suit les imports depuis la mise en page et n'a aucune liste à
  tenir à jour. Rien à déclarer — mais `npm run verifier-garde:guide-nego` doit repasser au vert, les
  nouvelles adresses comprises.

---

## R12 — Une session d'application dure une COP, pas une nuit

**Constat** : `AUTH_SESSION_TTL` vaut **12 h** ; `AUTH_SESSION_TTL_REMEMBERED`, **30 j**, et seulement
si la personne coche « se souvenir de moi ». **L'écran « 03b Connexion » de la maquette n'a pas cette
case** — et l'ajouter serait la mauvaise réponse : personne ne devrait avoir à cocher pour que son
application de terrain tienne la semaine.

**Décision** : `client.kind = "app"` ouvre une session de **90 jours, glissants** — la durée repart à
chaque rotation. Un réglage la porte (`AUTH_SESSION_TTL_APP`), jamais une constante.

**Pourquoi glissante** : une durée fixe posée à la connexion ferait expirer le 14 novembre un compte
ouvert le 15 octobre — au milieu de la COP, après des semaines de préparation. C'est exactement le
moment où l'application doit tenir, et exactement celui où il n'y a pas de réseau pour se reconnecter.

**Et quand elle expire quand même**, hors connexion : l'application **garde ce qui a été lu**, avec
l'heure de sa lecture, et ne réclame la reconnexion qu'au retour du réseau. Le réflexe inverse —
vider l'écran et renvoyer à la connexion dès que le jeton est périmé — transforme une salle sans
réseau en application inutilisable.

**Écarté** : ajouter la case « se souvenir de moi » à l'écran de l'application. Elle fait porter à la
personne une décision d'ingénierie, et une personne qui ne coche pas est punie trois semaines plus
tard, sans comprendre pourquoi.

---

## R13 — Les liens des courriels ramènent dans l'application

**Constat** : `identity/src/mail.rs` compose ses liens vers les écrans du **site**, chemins traduits
(`/verification-adresse`, `/en/verify-email`…). Une personne qui crée son compte dans l'application
atterrit sur l'ePavillon, à ses couleurs, **sans chemin de retour** — et son parcours est perdu.

**Décision** : `identity` choisit le lien d'après le **client de la demande**, retenu avec le jeton
(`one_time_tokens.payload` existe déjà). Demande venue de l'application → `/guide-nego/…` ; venue du
site → inchangé. `negotiation` n'appelle jamais `identity` : le courriel appartient au module qui
l'envoie (principe II).

**Le piège d'iPhone** : l'application installée n'y partage pas le stockage de Safari. Le lien ouvert
dans le navigateur est dans une autre session — la page **ne peut pas** continuer le parcours. Elle
confirme et renvoie : « Adresse confirmée — retournez dans Guide Négo ». L'application, **à son retour
au premier plan** (`visibilitychange`, déjà employé en 0a pour relire au retour du réseau), relit
l'état du compte et enchaîne **sans rien demander**. Sur Android, l'application capture le lien et
enchaîne directement.

Trois écrans à créer, absents de la maquette, composés avec les composants livrés — à inscrire comme
écart dans `05-design.md`. Le détail est dans
[contracts/courriels-retour-app.md](contracts/courriels-retour-app.md).

---

## R14 — On migre, on ne détruit pas

**Constat** : `docs/database/` n'est chargé qu'à la création du volume, et **il n'existe pas d'outil
de migration** (§ 13 de `DEPLOIEMENT.md`). 0b touche `identity.sessions`, une table **en service**.
Traiter `down -v` comme la procédure normale — ce que faisait la première version de ce plan —
effacerait la base locale, qui n'a pas de sauvegarde ; c'est ce qui est arrivé le 16/09.

**Décision** : un livrable de plus, [migration.sql](migration.sql), qui porte l'écart complet et se
rejoue **sans dégât** — chaque objet créé sous condition. En local comme en production : sauvegarder,
jouer, rejouer pour le prouver, puis `make check-db-safe`. **SQLx compile contre la base migrée.**

**Le contrôle, et il a déjà servi** : comparer le `pg_dump --schema-only` de la base migrée à celui
d'une base chargée depuis `docs/database/`, après tri des lignes. La **base modèle du harnais de test**
fait cette référence sans rien installer : elle porte l'empreinte du SQL et se reconstruit seule dès
qu'il change (`kernel/src/testing.rs`). Le 16/09, ce contrôle a rattrapé le seul oubli de la
migration précédente.

**Conséquence sur les tests** : ils n'ont besoin d'aucune manœuvre. Le harnais recopie sa base modèle
par `CREATE DATABASE … TEMPLATE` à chaque test.

---

## R11 — Ce que 0b ne fait pas

- **Aucune double authentification.** Le parcours de connexion reprend l'étape `mfa_required`
  existante telle quelle, sans l'étendre.
- **Aucune notification poussée ni centre de notifications** : étape 3b. Le courriel est le seul canal
  sortant, et l'écran d'attente le dit.
- **Aucun contenu de module réservé** : le verrou est livré, ce qu'il garde vient à son étape.
- **Aucun nouveau drapeau.** `guide_nego.enabled` continue de commander l'ouverture de l'application.
