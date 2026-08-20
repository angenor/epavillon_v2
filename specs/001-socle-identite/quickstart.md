# Démarrage et vérification

**Fonctionnalité** : Socle technique et Identité (B1) · **Date** : 2026-08-20

Comment lancer l'API, éprouver les parcours à la main, et vérifier que ce que la spécification exige est réellement tenu. Ce fichier est un **guide d'exécution** : il ne contient pas de code d'implémentation.

---

## Préalables

```bash
cp .env.example .env          # une seule fois
make up                       # postgres, valkey, jaeger, mailpit, garage
```

`make up` attend que le schéma soit chargé — les dix-neuf fichiers de `docs/database/`, dans l'ordre de leur numérotation. Ne pas se fier au seul état de santé du conteneur : il passe au vert avant la fin du chargement, et `make up` le sait.

Trois interfaces servent aux vérifications :

| Interface | Adresse | Ce qu'on y regarde |
|---|---|---|
| Mailpit | `http://localhost:8025` | les courriels partis, avec leurs liens |
| Jaeger | `http://localhost:16686` | la trace d'une requête, par son identifiant |
| Documentation de l'API | `http://localhost:8080/api/docs` | l'OpenAPI **généré** — **livré avec T116 et T117, en phase 11** ; jusque-là, la route n'existe pas et le catalogue se lit dans [`contracts/errors.md`](contracts/errors.md) |

**Nouvelles clés à renseigner dans `.env`** avant le premier lancement :

```
AUTH_SIGNING_KEY=            # clé Ed25519 — engendrée une fois, gardée
MAIL_TRANSPORT=relay         # relay aujourd'hui ; smtp le jour de l'autorisation
MAIL_RELAY_URL=http://localhost:3000/api/internal/mail
MAIL_RELAY_TOKEN=            # le même secret des deux côtés
TRUSTED_PROXIES=             # VIDE en local, et c'est le bon défaut
```

Le démarrage **échoue** si l'une manque ou est mal écrite : une durée nulle, une clé de signature vide, une adresse de relais qui n'est pas absolue, une adresse de mandataire illisible. C'est voulu — un réglage fautif doit arrêter le service, jamais une requête. `TRUSTED_PROXIES` fait exception à l'obligation, pas à la validation : **vide est une valeur juste**, et même la bonne en local, où l'API est appelée en direct. Elle ne se renseigne que derrière un frontal, avec l'adresse de CELUI-CI — sans quoi n'importe quel client choisirait l'adresse enregistrée dans sa session.

Le site lit le même secret, plus les identifiants SMTP : **c'est lui qui envoie, jamais l'API** (contrainte d'hébergement du 20/08).

---

## Lancer

```bash
cd backend && cargo run -p api        # l'API, sur 127.0.0.1:8080
cd backend && cargo run -p worker     # le relais d'outbox et la file de travaux
cd frontend && npm run dev            # le site — indispensable pour que les courriels partent
```

**Les trois sont nécessaires pour éprouver un parcours complet.** Sans le worker, aucun courriel n'est mis en route ; sans le site, aucun n'est envoyé.

`cargo run` échoue si la base n'est pas démarrée : SQLx vérifie ses requêtes **à la compilation**. Ce n'est pas une gêne, c'est le mécanisme qui fait qu'un nom de colonne inventé ne compile pas.

---

## Éprouver les parcours à la main

### D'abord, un compte à qui parler — et il n'y en a pas encore

**`900_seed.sql` ne sème aucun mot de passe.** La seule personne qu'il crée est l'administrateur de
l'IFDD, explicitement « sans moyen d'authentification » ; les adresses des données simulées du site —
`a.sowfall@roac-afrique.org` et les autres — n'existent que dans `frontend/app/mocks/`, et
`.env.example` le dit : ce mot de passe « n'ouvre rien d'autre que des mocks ».

Aucune commande `psql` ne peut fabriquer l'empreinte manquante :
`COMMENT ON COLUMN identity.accounts.password_hash` interdit à la base de connaître les mots de passe,
et la seule chose qui sache en calculer une est le service. **La première route qui en crée est
l'inscription, en phase 6 (T072, T075).**

D'ici là, ce qui suit se lit de deux façons : les vérifications qui exigent un compte sont **tenues par
les tests d'intégration**, qui sèment leur propre compte dans une base jetable — c'est
`discretion_temps_de_reponse` qui mesure l'écart de temps ci-dessous, et `routes_auth` qui exerce les
routes par HTTP. Les commandes `curl` valent telles quelles dès qu'un compte existe : remplacer
`compte.reel@example.org` par son adresse.

### La règle de discrétion, qui est la plus facile à casser

```bash
# Adresse inconnue
time curl -s localhost:8080/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"personne.inconnue@example.org","password":"nimportequoi","remember_me":false}'

# Adresse connue, mot de passe faux
time curl -s localhost:8080/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"compte.reel@example.org","password":"nimportequoi","remember_me":false}'
```

**Deux choses doivent être identiques** : la réponse — `{"status":"invalid_credentials"}` au caractère près — **et le temps**. Si la première revient nettement plus vite, l'empreinte factice n'est pas calculée, et le formulaire est redevenu un annuaire des comptes.

### Le cycle d'inscription

```bash
curl -s localhost:8080/api/auth/register -H 'Content-Type: application/json' \
  -d '{"first_name":"Awa","last_name":"Diallo","email":"awa.diallo@example.org",
       "country_id":"<un pays de reference.countries>","password":"Belem2027!",
       "preferred_locale":"fr","timezone":"Africa/Dakar"}'
```

Puis, **dans l'ordre** :

1. Mailpit montre le courriel et son lien.
2. Rejouer la même inscription : **la réponse est identique**, et Mailpit montre cette fois un rappel de compte existant.
3. Suivre le lien → l'adresse est vérifiée.
4. Le rejouer → « déjà utilisé », et non « expiré ».
5. Se connecter **avant** d'avoir vérifié → `email_unverified`, et **aucun cookie de session n'est posé**.

### Le verrouillage

Cinq mots de passe faux d'affilée sur le même compte. Le sixième essai, **avec le bon mot de passe**, rend `locked` **et porte la date de fin** — c'est ce que l'écran affiche. Attendre la fin du verrou : le compteur repart de zéro.

### Le périmètre d'administration

Avec le compte de Claire Perret, administratrice de la seule COP31 :

```bash
curl -s localhost:8080/api/admin/users -b cookies.txt          # ne rend que son périmètre
curl -s localhost:8080/api/admin/privacy-requests -b cookies.txt   # 403 — la file RGPD exige la portée globale
```

Avec un compte sans aucun droit d'administration : `/api/admin/users` rend **403**, jamais une liste vide.

### La chaîne différée, de bout en bout

1. Provoquer une inscription.
2. **Arrêter le worker** avant qu'il ne prenne le travail : le courriel n'est pas parti, la ligne est en file.
3. Le relancer : le courriel part.
4. Le relancer encore : **rien ne repart** — la garde d'idempotence a fait son travail.
5. Arrêter le site et provoquer une inscription : le travail échoue, se replanifie, et `/api/health` le fait remonter — **route livrée avec T102** ; jusque-là, la file se lit en base.
6. Tuer le worker en cours de lot : le travail reste réservé, puis **revient à la file** au bout de trente minutes de bail expiré. La base sait voir ce cas (`ix_jobs_stuck`, alerte `travaux_bloques`) mais ne le répare pas — la reprise est écrite dans le worker.

---

## Les tests

```bash
cd backend && cargo test              # base réelle et jetable
make check-back                       # mise en forme, analyse statique, tests
make check                            # les trois — DÉTRUIT la base
```

Chaque test crée sa propre base par recopie d'une base modèle chargée une fois depuis `docs/database/`. **Aucun mock de base** : la moitié des invariants de ce projet vit dans PostgreSQL, et un double en mémoire ferait passer au vert un code que la base refuserait.

### Les quatre obligations de la constitution, et le test qui les tient

**La colonne « Test » nomme des FICHIERS**, pas des fonctions : `cargo test --test <nom>` les joue.

| Obligation (principe X) | Test |
|---|---|
| 1. Le chemin nominal de chaque route | `routes_auth` (crate `api`) — l'application du binaire est montée telle quelle, avec ses trois intergiciels, et chaque route livrée est appelée **par HTTP**. Les routes des phases suivantes s'y ajoutent au fur et à mesure |
| 2. Un refus d'accès par périmètre, **URL forgée comprise** | `perimetre_url_forgee` — compte détaché sur une édition, identifiant d'une autre édition dans l'URL, sur chaque route paramétrée |
| 3. La traduction d'une erreur d'invariant de la base | `role_portee` (T098) — attribution d'un rôle sur une portée interdite : le message français du trigger ressort tel quel |
| 4. L'écriture des événements attendus dans l'outbox | `outbox_transactionnel` (T103) — un événement par changement, **et zéro si la transaction est annulée** |

### Les vérifications propres à ce module

| Ce qui est vérifié | Test |
|---|---|
| Adresse inconnue et mot de passe faux : même réponse, **même temps** | `discretion_temps_de_reponse` — 100 tentatives de chaque sorte, médianes, écart sous 10 % |
| Les six issues de connexion, et l'ordre des contrôles | `connexion_issues` |
| Adresse non vérifiée → refus, sans session (FR-024) | `connexion_issues` |
| Un verrou échu rend **tous** ses essais au compte (FR-015) | `connexion_issues` |
| Renouvellement, rotation, déconnexion | `session_rotation` |
| Jeton rejoué → toutes les sessions révoquées (FR-031) | `rejeu_du_jeton` |
| Deux renouvellements simultanés n'ouvrent qu'une session (FR-031) | `rejeu_du_jeton` |
| Suspension, changement de mot de passe, anonymisation → sessions coupées (FR-033) | `suspension_coupe_les_sessions` |
| Statuts, corps, cookies et corps d'erreur, **par HTTP** | `routes_auth` (crate `api`) |
| Deux consommations simultanées d'un jeton → une seule aboutit (FR-041) | `consommation_concurrente_du_jeton` |
| « Déjà utilisé » l'emporte sur « périmé » (FR-039) | `ordre_des_refus_de_jeton` |
| Retirer un rôle exige le même droit qu'attribuer (FR-053) | `retrait_exige_le_meme_droit` |
| Les paramètres de droits envoyés par le client sont **ignorés** (FR-055) | `droits_declares_ignores` (T099) |
| Effacement refusé sur une demande d'export (FR-060) | `effacement_reserve_a_la_demande_deffacement` |
| Une écriture sans contexte ne doit pas exister (FR-004) | `toute_ecriture_laisse_son_auteur` — parcourt `platform.audit_log` **pour les seules lignes écrites pendant le test** (borne sur `occurred_at`, prise au début : le harnais charge `900_seed.sql` comme le conteneur, et le semis laisse six lignes d'audit sans acteur), et **échoue s'il reste une ligne sans acteur** |
| Aucun secret utilisable en base (SC-009) | `aucun_secret_en_base` — après un cycle complet, ni mot de passe, ni jeton de session, ni jeton de lien en clair |

Le test `toute_ecriture_laisse_son_auteur` mérite d'exister pour une raison précise : **une écriture sans contexte n'échoue pas**. Elle écrit une trace anonyme, et rien ne le signale. C'est le seul défaut de ce module qu'aucun mécanisme ne rattrape — d'où un test qui le cherche.

---

## Les portes à passer avant de livrer

```bash
make check
```

| Porte | Ce qu'elle assure |
|---|---|
| `check-db` | schéma rechargé de zéro, seize schémas présents, **rapport de frontières vide**, projections analytiques rafraîchies |
| `check-front` | le site compile toujours — la route de relais en fait partie |
| `check-back` | mise en forme, **analyse statique sans un seul avertissement**, tests |

Et trois vérifications qui ne sont pas dans le `Makefile` mais que la constitution impose :

```bash
# tail -n +2 écarte la ligne RACINE, qui porte elle-même le chemin
# crates/modules/identity — sans elle, le compte vaut 1 et jamais 0.
cd backend && cargo tree -p identity | tail -n +2 | grep -c "crates/modules/"

# backend/crates, pas backend : target/ porte des fichiers Rust ENGENDRÉS par
# les dépendances, qui ne sont le code de personne.
find backend/crates -name '*.rs' | xargs wc -l | sort -rn | head

# À partir de la phase 11 seulement : la route /api/docs est livrée par T117.
curl -s localhost:8080/api/docs | grep -c '"code"'           # chaque code d'erreur documenté
```

**Rappel** : `make check` commence par détruire le volume de la base. Pendant le développement, `make check-db-safe`.

---

## Une fois que tout passe

Ne **pas** renseigner `NUXT_PUBLIC_API_BASE` : le raccordement du site à l'API est le prompt **B7**. Tant qu'elle est vide, les écrans lisent les données simulées, et c'est ce qui permet de développer les modules B2 à B6 sans casser l'interface à chaque étape.
