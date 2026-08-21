# Quickstart — Sessions (B5)

**Fonctionnalité** : [spec.md](spec.md) · **Plan** : [plan.md](plan.md) · **Routes** : [contracts/routes.md](contracts/routes.md)

Comment lancer ce module, l'éprouver à la main, et savoir qu'il tient. Ce fichier ne contient aucun code d'implémentation : c'est un guide de mise en route et de vérification.

---

## Préalables

```bash
cp .env.example .env                                   # si ce n'est pas déjà fait
docker compose -f ops/docker-compose.dev.yml up -d     # Postgres, Valkey, Mailpit, Jaeger, Garage
make garage-init
```

**Le schéma n'a pas bougé depuis B3 : B5 ne modifie aucun fichier de `docs/database/`.** Si la base a été rechargée entre-temps, rien à refaire.

**Une variable nouvelle**, et une seule : `PRIVACY_POLICY_VERSION` (défaut `2026-01`). Elle date la version de politique inscrite sur la preuve de consentement (R22). C'est un réglage d'exploitation, comme le seuil de verrouillage de B1.

`DATABASE_URL` doit être renseignée et la base démarrée pour **compiler** : SQLx vérifie les requêtes à la compilation.

---

## Lancer

```bash
cd backend && cargo run -p api        # l'API, http://localhost:8080
cd backend && cargo run -p worker     # LE WORKER EST INDISPENSABLE ICI
```

**Le worker n'est plus facultatif.** B5 est le premier module à **consommer** un événement de domaine : sans worker, le bouton « Publier » de B3 estampille l'édition et **rien ne devient public**. C'est le premier symptôme à connaître.

`GET /api/docs` rend l'OpenAPI engendrée. Les dix-sept routes de ce module y figurent avec leurs huit codes d'erreur.

---

## Éprouver les parcours à la main

### D'abord, de quoi parler

Le semis ne pose aucune séance. Il faut donc enchaîner : une **édition** avec son fuseau et ses jours (B3), une **salle physique** et une **salle virtuelle** (B3), un **canal de diffusion par défaut** (B3), un **appel ouvert** (B3), une **organisation vérifiée** (B2), une **personne** membre active (B1, B2), puis un **dossier déposé** et **évalué** (B4). Les routes existent toutes ; les enchaîner est le parcours complet du jalon.

### 1. La séance naît — et elle naît toute seule

Retenir le dossier. **Ne rien faire d'autre.**

Ouvrir `GET /admin/planner?event_id=…` : la séance doit être dans `unplaced`, avec le créneau que l'organisation avait souhaité, `room_id` nul, et le **même nombre d'intervenants** que le dossier.

Retenir un second dossier demandant **trois** occurrences : trois séances, rangs 1 à 3, trois adresses d'URL distinctes.

**Puis rejouer** : remettre le premier dossier en évaluation, le retenir à nouveau. Le nombre de séances **ne doit pas bouger**. Si le panneau en compte deux, l'insertion ne s'appuie pas sur la contrainte du modèle (R6).

**Compter les lignes d'outbox** après l'acceptation à trois occurrences :

```sql
SELECT event_type, count(*) FROM platform.outbox_events
 WHERE aggregate_type = 'session' GROUP BY 1;
```

Trois `programme.session.created`, pas six. Six signifie que le service émet en plus du déclencheur — le piège n° 1 du module (R2).

### 2. Le chevauchement passe, et c'est la règle

Placer deux séances de la même édition **en salle physique**, sur le même créneau. Les deux écritures doivent **réussir**, et la réponse de la seconde porter un conflit `venue_capacity` ou `room` de gravité `blocking`.

Placer maintenant une séance en **salle virtuelle** au même moment qu'une séance physique : **aucun conflit** ne doit remonter. Si l'un apparaît, la branche de « stand unique » a été rejouée dans le service au lieu d'être lue.

Retirer la salle d'une séance placée : elle doit revenir dans `unplaced`, avec son créneau intact.

### 3. Le piège du jour, celui qui ne se voit pas

Placer une séance le **12 novembre**, relever son `event_day_id`. La déplacer au **14 novembre** sans fournir de journée. Relire :

```sql
SELECT s.starts_at, d.day_date
  FROM programme.sessions s JOIN event.event_days d ON d.id = s.event_day_id
 WHERE s.id = '…';
```

`day_date` **doit** être le 14. Si c'est encore le 12, la mise à nul de R9 n'a pas été faite — et rien d'autre ne le signalera (écart n° 113).

### 4. Les champs dérivés

Envoyer une écriture de créneau portant `enforce_room_exclusivity` : refus **422**, `field` nommé. Idem avec `time_range`.

Envoyer `{ is_streamed: true, broadcast_channel_id: <un canal de l'édition> }` : **accepté**, et le canal retenu doit être celui-là — pas le canal par défaut. Si le service l'a écrasé, il a pris l'écart n° 7 à la lettre (R8).

Envoyer `{ is_streamed: false, broadcast_channel_id: <un canal> }` : refus **422**, `field` nommé — c'est là que la base efface en silence.

Retirer la diffusion d'une séance diffusée : `broadcast_channel_id` doit être **nul** en base.

### 5. Un seul direct, deux éditions

Marquer diffusées deux séances **de deux éditions différentes**, sur le même canal et le même créneau. Les deux écritures réussissent, et `GET /sessions/conflicts` doit remonter un conflit `broadcast` **bloquant** depuis l'une comme depuis l'autre.

### 6. La journée spéciale, et le fil d'à côté

Rattacher une séance à deux fils, puis renvoyer une liste d'un seul : le second doit être **détaché**. Vérifier `added_by` en base.

Rattacher une séance de la COP31 à un fil d'une **autre** édition : refus **422**, code `SESSION_TRACK_EVENT_MISMATCH`, message français — jamais l'exception brute de PostgreSQL.

### 7. La publication — les deux moitiés

Régler les points bloquants du récapitulatif, puis `POST /admin/planner/publish` (route de B3). Relever `published_count`.

**Avec le worker arrêté** : `event.events.programme_published_at` est posée, et `GET /schedule?event_id=…` rend **zéro ligne**. C'est le symptôme normal, et c'est pourquoi le worker est indispensable.

Démarrer le worker. Quelques secondes plus tard :

```sql
SELECT count(*) FROM programme.sessions
 WHERE event_id = '…' AND published_at IS NOT NULL;
SELECT status, count(*) FROM programme.sessions WHERE event_id = '…' GROUP BY 1;
```

Le premier chiffre doit **égaler** `published_count`. Le second doit montrer que les séances « pressenties » sont passées à « programmées » (R12).

**Rejouer l'annonce** — remettre `published_at` à nul sur l'événement d'outbox et le relancer, ou republier : **aucune séance de plus** ne doit être publiée, la garde de rejeu du noyau ayant déjà réservé le couple.

```sql
SELECT consumer, count(*) FROM platform.inbox_events GROUP BY 1;
```

### 8. Le formulaire, et ce qu'il refuse

`GET /sessions/{id}/registration-form` sur une séance **sans formulaire attaché** : le formulaire de l'édition, à défaut celui de la plateforme, doit être rendu — avec ses seuls champs **actifs** (le champ « régime alimentaire » des données simulées est désactivé et ne doit pas apparaître) et les options de `referral_source` **résolues depuis la taxonomie**, avec leurs libellés traduits.

Puis s'inscrire, en cinq essais :

| Essai | Attendu |
|---|---|
| Sans le pays (obligatoire) | 422, `REGISTRATION_ANSWER_INVALID`, `field: "country"` |
| `country: "Sénégal"` | 422, même code, même champ — c'est le **code ISO** qui est attendu (R18) |
| `referral_source: "carte-postale"` | 422, hors options |
| `badge_unfccc: "oui"` | 422, mauvais type — c'est un booléen |
| `dietary: "…"` | 422, **clé inconnue** : le champ est désactivé |

**Le troisième essai est celui qui compte** : il prouve que la validation se fait contre le formulaire **résolu** et non contre le formulaire attaché — qui, ici, n'existe pas (écart n° 114). Si l'inscription passe sans pays, le déclencheur n'a rien vérifié et le service non plus.

### 9. Le consentement

S'inscrire en répondant au champ « besoins d'accessibilité » **sans** `sensitive_data_consent` : refus 422, `REGISTRATION_CONSENT_REQUIRED`, `field: "access_needs"`.

Avec le consentement : l'inscription passe, et la preuve existe :

```sql
SELECT purpose, is_granted, policy_version, source, ip_address
  FROM identity.consents WHERE person_id = '…';
```

Une ligne, finalité `registration_sensitive_data`, origine `registration_form`.

### 10. La jauge, la liste d'attente, et la concurrence

Poser une jauge de **trois** places sur une séance avec liste d'attente. Inscrire cinq personnes une à une : trois `registered`, puis `waitlisted` en positions 1 et 2.

Sans liste d'attente : la quatrième doit rendre **200** avec `{ status: 'full', capacity: 3 }` — et `capacity` doit être **relu sur la séance**, jamais extrait de la phrase du déclencheur.

**Puis la vraie épreuve** : cent inscriptions **concurrentes** sur une séance de dix places.

```sql
SELECT status, count(*) FROM programme.registrations WHERE session_id = '…' GROUP BY 1;
SELECT waitlist_position, count(*) FROM programme.registrations
 WHERE session_id = '…' AND status = 'waitlisted'
 GROUP BY 1 HAVING count(*) > 1;
```

Le premier doit rendre **exactement dix** `registered`. Le second doit rendre **zéro ligne** : deux personnes au même rang d'attente signifient que le verrou de ligne de R19 n'a pas été posé — le déclencheur ne protège ni l'un ni l'autre (écart n° 124).

### 11. L'annulation promeut, et une seule fois

Annuler une inscription **confirmée** : la réponse porte `promoted: 1`, et la première personne de la file passe `registered` avec une position nulle. Le nombre de confirmées **ne dépasse jamais la jauge**.

Annuler une inscription **en attente** : `promoted: 0`.

Se réinscrire après annulation : **accepté** (l'index d'unicité ignore les annulations).

### 12. Ce que l'organisation voit — et ne voit pas

Ouvrir `GET /proposals/{id}/file` avec le compte de l'organisation porteuse d'un dossier retenu à quarante inscrits et onze en attente.

**Ne pas vérifier champ par champ.** Sérialiser la réponse entière et y chercher, en texte : le nom d'un inscrit, son adresse électronique, la valeur d'une réponse au formulaire. Aucun ne doit s'y trouver. C'est la forme de test qui a fonctionné en B4 : vérifier que la note est nulle laisserait passer celle qui arriverait par un champ ajouté demain.

Les trois nombres doivent être là, et `reminders` **vide, jamais absent** (écart n° 108).

### 13. Ce que l'administration voit

`GET /registrations?session_id=…` avec un compte détenant la permission de gérer les inscriptions **sur cette édition** : la liste nominative.

Avec un compte détenant seulement la permission de **planifier** : refus. C'est l'écart n° 119, et c'est le modèle qui le veut.

Avec un compte détaché sur une **autre** édition : refus **indiscernable** de celui d'une séance inexistante.

---

## Vérifier que le module tient

```bash
cd backend
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Puis, depuis la racine :

```bash
make check          # détruit la base, la recharge de zéro, et repasse tout
```

**Les cinq contrôles qui ne se déduisent pas des tests**

```bash
# 1. Aucune arête entre crates de module.
cargo tree -p programme | grep -E 'identity|org|event' || echo "aucune arête — attendu"

# 2. Aucun fichier au-dessus de mille lignes. C'est LE module où la marge est mince.
find backend/crates -name '*.rs' -exec wc -l {} + | sort -rn | head -5

# 3. Les frontières du modèle restent sans écart.
psql "$DATABASE_URL" -c "SELECT * FROM platform.cross_module_fk_report WHERE NOT is_compliant;"

# 4. Les écritures hors schéma sont exactement trois fichiers, et pas un de plus.
grep -rlE 'INSERT INTO (identity|org|event|reference|media)\.' backend/crates/modules/programme/src/
#    attendu : repo/themes.rs · repo/people.rs · repo/consents.rs

# 5. Le service n'émet aucun événement.
grep -rn 'events::emit' backend/crates/modules/programme/src/ || echo "aucune émission — attendu"
```

Le quatrième et le cinquième sont les deux contrôles propres à ce jalon. Le cinquième surtout : c'est un `grep` qui vaut un test, et il se relit en une seconde.

---

## Les six symptômes à reconnaître

| Symptôme | Cause probable |
|---|---|
| Le programme est « publié » mais la page publique est vide | **Le worker ne tourne pas.** C'est la moitié consommatrice de la publication |
| Deux courriels par inscription (le jour où B6 arrivera) | Le service émet en plus du déclencheur (R2) |
| Une séance déplacée reste rangée au mauvais jour | La journée n'a pas été remise à nul (R9, écart n° 113) |
| Une inscription passe sans réponse obligatoire | La validation porte sur le formulaire **attaché** et non **résolu** (écart n° 114) |
| Onze inscrits sur dix places | Le verrou de ligne n'est pas posé (R19, écart n° 124) |
| Le canal choisi est remplacé par celui par défaut | L'écart n° 7 a été pris à la lettre (R8, écart n° 111) |
