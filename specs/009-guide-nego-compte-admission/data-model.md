# Modèle de données — 0b, compte et admission

> **Le SQL d'abord.** Tout ce qui suit s'écrit dans `docs/database/`, et le code ne s'écrit qu'ensuite —
> principe I. Aucun nom n'est inventé : ce qui existe est cité tel quel, ce qui s'ajoute suit les
> conventions du fichier hôte.
>
> **La base ne se détruit pas.** `identity.sessions` est une table en service : l'écart se franchit par
> [migration.sql](migration.sql), rejouable sans dégât, en local comme en production — c'est la méthode
> du § 13 de [DEPLOIEMENT.md](../../docs/DEPLOIEMENT.md). `down -v` effacerait la base locale, qui n'a
> pas de sauvegarde, et il n'est pas nécessaire : le harnais de test reconstruit seul sa base modèle
> dès que l'empreinte de `docs/database/` change (`kernel/src/testing.rs`).

## 1. Ce qui existe et ne bouge pas

| Objet | Où | Ce qu'on en fait |
|---|---|---|
| `identity.people`, `identity.accounts`, `identity.one_time_tokens` | `030_identity.sql` | Repris tels quels : l'inscription, la vérification d'adresse, la connexion et la réinitialisation de l'ePavillon servent l'application sans être réécrites |
| `identity.permissions` → `negotiation.space.access` | `030_identity.sql` l. 583, re-semée dans `100_negotiations.sql` | **La** permission de l'accès réservé. Rien à créer |
| `identity.roles` → `negotiator`, `allowed_scopes = '{global,negotiation_space}'` | `030_identity.sql` l. 556 | Porte la permission. Les **deux portées autorisées sont exactement les deux choix offerts à la création d'un code** — un espace précis, ou Guide Négo en entier |
| `identity.role_assignments` | `030_identity.sql` | **L'accès effectif.** Porte déjà `granted_by`, `valid_from`, `valid_until`, `revoked_at`, `revoked_by`, `revoked_reason` : le retrait d'accès n'a besoin d'aucune colonne nouvelle |
| `identity.has_permission(person, permission, scope_type, scope_id)` | `030_identity.sql` | Le seul test d'autorisation — principe V |
| `identity.administered_events(person)` | `030_identity.sql` | Le périmètre d'administration des écrans de back-office |
| `negotiation.spaces` | `100_negotiations.sql` § 1 | Cible de portée RBAC. `archived_at` marque un espace clos |
| `negotiation.space_members` | `100_negotiations.sql` § 2 | L'**annuaire** d'un espace, qui n'accorde aucun droit. L'API écrit l'adhésion et l'attribution de rôle dans la même transaction ; en cas de divergence, le RBAC fait foi |
| `identity.negotiator_profiles` | `030_identity.sql` | Rattaché à l'adhésion quand il existe. Aucun champ n'y est ajouté |
| `platform.settings` | `010_platform.sql` | Porte le mode d'admission et les seuils d'essais |
| `platform.modules` → `('negotiation','negotiation',…)` | `010_platform.sql` l. 531 | Le module est **déjà** au registre : le crate se monte sans rien y changer |
| `platform.emit_event()`, `platform.tg_audit()`, `platform.jobs` | `000_bootstrap`, `010_platform` | Outbox, audit et travaux différés |

**Le drapeau `negotiation.enabled` ne commande pas ces routes.** Il ferme l'espace `/negociations` du
site ; Guide Négo a le sien, `guide_nego.enabled`, posé à l'étape 0a. Aucun drapeau nouveau ici.

---

## 2. `030_identity.sql` — la session dit d'où elle vient

ADR-001 l'annonçait : « `identity.sessions` reçoit le type de client et l'appareil — à ajouter au SQL ».
La table porte aujourd'hui `user_agent` et `ip_address`, qui ne distinguent pas le site de l'application.

```sql
CREATE TYPE identity.session_client AS ENUM ('web', 'app');

ALTER TABLE identity.sessions
    ADD COLUMN client_kind      identity.session_client NOT NULL DEFAULT 'web',
    ADD COLUMN device_id        text,
    ADD COLUMN device_label     text,
    ADD COLUMN device_platform  text;

CREATE INDEX ix_sessions_client ON identity.sessions (client_kind, issued_at DESC);
```

*(À écrire dans la définition de la table elle-même, pas en `ALTER` : le fichier est rechargé de zéro.)*

| Colonne | Rôle |
|---|---|
| `client_kind` | `web` ou `app`. Un ENUM fermé et technique, au même titre que `identity.auth_provider` — ce n'est pas un vocabulaire métier ouvert. La coquille Capacitor d'ADR-002 y ajoutera sa valeur le jour venu |
| `device_id` | Identifiant d'installation **opaque**, engendré par le client et gardé sur l'appareil. Il sert à nommer une session dans une liste et à compter les essais de code — **jamais à accorder un droit** : un client peut le forger |
| `device_label` | Ce que la personne lit : « Android · Chrome ». Composé par le client |
| `device_platform` | Plateforme déclarée : `android`, `ios`, `other` |

**Ce que ça donne** : le nombre de personnes qui utilisent l'application se compte sans dédoubler
personne (FR-005), et la déconnexion ne ferme que la session de cet appareil (FR-006), ce que la table
permettait déjà par sa clé.

---

## 3. `020_reference.sql` — le réseau est un vocabulaire ouvert

ADR-007 : ce qui distingue une négociatrice est son **appartenance au réseau**, et « un code par
public ». Le réseau n'est donc ni une case sur l'identité, ni un ENUM : c'est un terme de taxonomie,
comme les thématiques — c'est le défaut D1 corrigé de `100_negotiations.sql`, et la règle du projet.

```sql
INSERT INTO reference.taxonomies (code, label, description, is_system) VALUES
    ('negotiation_network',
     '{"fr":"Réseaux de négociation","en":"Negotiation networks"}',
     'Réseaux auxquels un code d''invitation peut donner l''appartenance.', true);

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('negotiation_network', 'women_negotiators',
     '{"fr":"Réseau des négociatrices francophones","en":"Francophone women negotiators network"}', 10);
```

*(Les colonnes exactes de `reference.taxonomies` se relisent à `020_reference.sql` l. 92 avant
d'écrire ; `is_system` n'est ajouté que si elle existe.)*

**Aucun champ « genre » n'est créé, lu ni affiché** (FR-008, SC-006). L'appartenance vient du code
utilisé, et de rien d'autre.

---

## 4. `100_negotiations.sql` — cinq tables et deux vues

Elles s'insèrent après le § 2 « Appartenance à un espace », dont elles prolongent l'articulation avec
le RBAC. Le fichier passe d'environ 780 à environ 1050 lignes — `docs/database/` est hors du garde-fou
des mille lignes, et le découper le rendrait moins lisible.

### 4.1 Les codes d'invitation

```sql
CREATE TABLE negotiation.invitation_codes (
    id                     uuid    PRIMARY KEY DEFAULT platform.uuid_v7(),
    code                   text    NOT NULL CHECK (code ~ '^[A-Z0-9-]{6,16}$'),
    -- Comparaison insensible à la casse et aux séparateurs : le code circule
    -- recopié à la main depuis WhatsApp (FR-010).
    code_normalized        text    GENERATED ALWAYS AS
                                   (upper(regexp_replace(code, '[^A-Za-z0-9]', '', 'g'))) STORED,
    label                  text    NOT NULL,
    scope_type             identity.scope_type NOT NULL,
    space_id               uuid    REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    grants_network_term_id uuid    REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    max_uses               integer,                    -- NULL = sans limite
    used_count             integer NOT NULL DEFAULT 0, -- tenu par trigger, jamais écrit à la main
    valid_from             timestamptz NOT NULL DEFAULT now(),
    valid_until            timestamptz,
    revoked_at             timestamptz,
    revoked_by             uuid    CONSTRAINT xmod_fk_invitation_codes_revoker
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    revoked_reason         text,
    created_by             uuid    CONSTRAINT xmod_fk_invitation_codes_creator
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at             timestamptz NOT NULL DEFAULT now(),
    updated_at             timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_invitation_codes_scope CHECK (
        (scope_type = 'global'            AND space_id IS NULL) OR
        (scope_type = 'negotiation_space' AND space_id IS NOT NULL)),
    CONSTRAINT ck_invitation_codes_uses   CHECK (max_uses IS NULL OR max_uses > 0),
    -- Le quota se tient EN BASE. Sans ce CHECK, deux entrées simultanées sur le
    -- dernier usage passeraient toutes les deux : chacune lit `used_count = 119`
    -- avant que l'autre n'écrive. L'incrément du trigger prend le verrou de
    -- ligne, la seconde échoue ici, et l'API la traduit en « épuisé ».
    CONSTRAINT ck_invitation_codes_quota  CHECK (max_uses IS NULL OR used_count <= max_uses),
    CONSTRAINT ck_invitation_codes_period CHECK (valid_until IS NULL OR valid_until > valid_from),
    -- Un auteur de révocation sans date de révocation serait une révocation
    -- qu'on ne peut pas dater — or l'écran « 04c » doit dire « révoqué le … ».
    CONSTRAINT ck_invitation_codes_revoked CHECK (revoked_by IS NULL OR revoked_at IS NOT NULL)
);

CREATE UNIQUE INDEX ux_invitation_codes_normalized ON negotiation.invitation_codes (code_normalized);
CREATE INDEX ix_invitation_codes_space ON negotiation.invitation_codes (space_id, created_at DESC);
```

Cinq points qui se paient cher si on les rate :

- **`ck_invitation_codes_scope` est la traduction en base du choix du commanditaire** : un code ouvre
  un espace précis, ou Guide Négo en entier — et les deux valeurs possibles de `scope_type` sont
  exactement les `allowed_scopes` du rôle `negotiator`. Rien d'autre ne passe.
- **L'unicité du code normalisé est totale, révoqués compris.** L'écran « 04c Code révoqué » doit
  pouvoir dire *« Ce code a été révoqué le 8 novembre »* : il faut donc retrouver un code révoqué. Une
  unicité partielle rendrait ce message impossible.
- **Il n'y a pas de colonne d'état.** Actif, révoqué, épuisé, terminé se **dérivent** — un état stocké
  se désynchronise le jour où une date passe. La vue du § 4.5 le calcule une fois pour tous.
- **`code_normalized` est bien `GENERATED`** : les fonctions employées sont immuables. Le piège du
  projet est `timestamptz + interval`, qui est STABLE — il n'y en a pas ici.
- **Le quota n'est pas gardé par le code.** `ck_invitation_codes_quota` et le verrou de ligne pris par
  l'incrément font qu'un code de 120 usages en accorde 120, jamais 121, même si deux personnes entrent
  à la même seconde. Un `SELECT` préalable en Rust laisserait passer les deux.

```sql
CREATE TRIGGER tg_invitation_codes_updated_at BEFORE UPDATE ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_invitation_codes_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE TRIGGER tg_invitation_codes_check_network
    BEFORE INSERT OR UPDATE OF grants_network_term_id ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'grants_network_term_id', 'negotiation_network');
```

Le dernier réemploie le garde-fou taxonomique **déjà écrit** au § 0 du fichier : un terme de la
mauvaise taxonomie est refusé par la base, pas par le code (principe VIII).

### 4.2 Qui est entré avec quel code

```sql
CREATE TABLE negotiation.invitation_code_uses (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    code_id      uuid        NOT NULL REFERENCES negotiation.invitation_codes(id) ON DELETE CASCADE,
    person_id    uuid        NOT NULL CONSTRAINT xmod_fk_invitation_code_uses_person
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    session_id   uuid        CONSTRAINT xmod_fk_invitation_code_uses_session
                             REFERENCES identity.sessions(id) ON DELETE SET NULL,
    used_at      timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX ux_invitation_code_uses_person ON negotiation.invitation_code_uses (code_id, person_id);
CREATE INDEX ix_invitation_code_uses_person ON negotiation.invitation_code_uses (person_id, used_at DESC);
```

C'est l'**historique**, pas le droit : comme `space_members`, cette table n'accorde rien. Elle répond à
« qui est entré avec ce code, et quand » (FR-040) ; l'accès effectif se lit dans
`identity.role_assignments`, et le retrait s'y écrit. **Aucune colonne de révocation ici** : dupliquer
l'état du RBAC, c'est se préparer à deux vérités.

`ux_invitation_code_uses_person` rend l'opération idempotente — deux appareils qui saisissent le même
code pour la même personne ne produisent qu'un usage (cas limite de la spécification).

### 4.3 L'appartenance à un réseau

```sql
CREATE TABLE negotiation.network_memberships (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_network_memberships_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    network_term_id uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    source_code_id  uuid        REFERENCES negotiation.invitation_codes(id) ON DELETE SET NULL,
    joined_at       timestamptz NOT NULL DEFAULT now(),
    left_at         timestamptz,
    CONSTRAINT ck_network_memberships_period CHECK (left_at IS NULL OR left_at >= joined_at)
);

CREATE UNIQUE INDEX ux_network_memberships_active
    ON negotiation.network_memberships (person_id, network_term_id) WHERE left_at IS NULL;
```

`source_code_id` porte la traçabilité demandée par ADR-007 : les chiffres des bailleurs se tirent de
l'appartenance, et l'on sait par quel code elle est venue. Même trigger taxonomique que ci-dessus.

**L'appartenance n'ouvre aucun droit à cette étape** (FR-014). Elle n'est testée par aucune route de
0b ; le canal réservé la lira à l'étape des Échanges.

### 4.4 Les demandes d'accès

```sql
-- `cancelled` = « annulée », le fait de la personne entrée par un code entre-temps.
-- Ce n'est PAS « révoquée », qui qualifie un accès retiré, jamais une demande.
CREATE TYPE negotiation.access_request_status AS ENUM ('pending', 'approved', 'rejected', 'cancelled');

CREATE TABLE negotiation.access_requests (
    id                 uuid    PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id          uuid    NOT NULL CONSTRAINT xmod_fk_access_requests_person
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    scope_type         identity.scope_type NOT NULL,
    space_id           uuid    REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    -- Mode « les deux » : le code a été reconnu, il n'a pas suffi à ouvrir.
    invitation_code_id uuid    REFERENCES negotiation.invitation_codes(id) ON DELETE SET NULL,
    message            text,
    status             negotiation.access_request_status NOT NULL DEFAULT 'pending',
    submitted_at       timestamptz NOT NULL DEFAULT now(),
    decided_at         timestamptz,
    decided_by         uuid    CONSTRAINT xmod_fk_access_requests_decider
                               REFERENCES identity.people(id) ON DELETE SET NULL,
    decision_reason    text,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_access_requests_scope CHECK (
        (scope_type = 'global'            AND space_id IS NULL) OR
        (scope_type = 'negotiation_space' AND space_id IS NOT NULL)),
    CONSTRAINT ck_access_requests_decision CHECK (
        (status = 'pending') = (decided_at IS NULL))
);

CREATE UNIQUE INDEX ux_access_requests_pending_space
    ON negotiation.access_requests (person_id, space_id)
    WHERE status = 'pending' AND space_id IS NOT NULL;
CREATE UNIQUE INDEX ux_access_requests_pending_global
    ON negotiation.access_requests (person_id)
    WHERE status = 'pending' AND space_id IS NULL;
CREATE INDEX ix_access_requests_queue
    ON negotiation.access_requests (space_id, submitted_at) WHERE status = 'pending';
```

Les deux index partiels portent « une seule demande en attente à la fois » (FR-024) **en base** : deux
appareils qui l'envoient ensemble ne produisent qu'une ligne, et le code traduit le conflit en message
français plutôt que de le prévenir par un `SELECT` préalable — principe VIII.

**La machine à états** est portée par un trigger, comme le veut le principe VIII : depuis `pending` on
va vers `approved`, `rejected` ou `cancelled` ; un état final ne se rouvre pas. Une nouvelle demande
après un refus est **une nouvelle ligne**, ce qui conserve l'historique des décisions (cas limite).

```sql
CREATE OR REPLACE FUNCTION negotiation.tg_access_request_transition() …  -- refuse tout départ d'un état final
CREATE OR REPLACE FUNCTION negotiation.tg_access_request_event()      … -- platform.emit_event() sur chaque décision
```

Le second appelle `platform.emit_event()` **dans la transaction de la décision** — c'est ce qui garantit
FR-028 (« le courriel ne part qu'une fois la décision enregistrée ») sans qu'aucun code n'ait à s'en
soucier, et ce qui interdit l'appel direct de `negotiation` vers `identity` (principe IV). Le fichier
porte déjà un précédent exact : `negotiation.tg_meeting_status_event()`.

Événements émis : `negotiation.access_request.submitted`, `.approved`, `.rejected`,
`negotiation.space_access.granted`, `negotiation.space_access.revoked`,
`negotiation.invitation_code.revoked` — tous au format `module.ressource.action` exigé par
`ck_outbox_event_type_format`.

### 4.5 Les essais de code

`01-stack.md` prévoyait Valkey pour ce compteur. **Valkey n'est câblé nulle part dans `backend/`**, et
l'y introduire est une dépendance d'ampleur qui exigerait une décision écrite. Une table le fait, se
purge, et donne en prime à l'administrateur la vue d'un essai de force brute.

```sql
CREATE TYPE negotiation.invitation_attempt_outcome AS ENUM (
    'accepted', 'unknown', 'revoked', 'exhausted', 'expired', 'not_yet_valid', 'throttled');

CREATE TABLE negotiation.invitation_code_attempts (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id    uuid        NOT NULL CONSTRAINT xmod_fk_invitation_attempts_person
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    device_id    text,
    outcome      negotiation.invitation_attempt_outcome NOT NULL,
    attempted_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_invitation_attempts_window
    ON negotiation.invitation_code_attempts (person_id, attempted_at DESC);
```

**Le compte se fait par personne, et par personne seulement.** `device_id` vient du corps de la
requête : il se forge. Compter « par personne **et** par appareil » suffirait à changer d'identifiant
pour remettre le compteur à zéro, et la limite ne limiterait rien. L'appareil est gardé comme
**information** — il aide un administrateur à lire une série d'échecs —, jamais comme borne.

```sql
CREATE OR REPLACE FUNCTION negotiation.invitation_attempts_recent(
    p_person_id uuid, p_window interval) RETURNS integer …
```

Elle compte les essais **non acceptés** de la fenêtre. Sa signature ne prend délibérément pas
d'appareil : ce qui n'est pas passé en argument ne peut pas être contourné.

**Le code essayé n'est jamais stocké**, ni en clair ni en empreinte : un essai raté peut être le vrai
code d'un autre espace, et cette table est lisible par un administrateur.

**Durée de garde : 90 jours**, inscrite au `COMMENT ON TABLE` et purgée par une chaîne récurrente du
worker (`platform.jobs`). Assez pour constater une série d'échecs et enquêter ; trop court pour que la
table devienne un journal de fréquentation.

### 4.6 La vue du back-office

```sql
CREATE VIEW negotiation.v_invitation_codes AS
SELECT c.*,
       CASE WHEN c.revoked_at IS NOT NULL                     THEN 'revoked'
            WHEN c.valid_until IS NOT NULL
                 AND c.valid_until <= now()                   THEN 'expired'
            WHEN c.valid_from > now()                         THEN 'not_yet_valid'
            WHEN c.max_uses IS NOT NULL
                 AND c.used_count >= c.max_uses               THEN 'exhausted'
            ELSE 'active' END AS state, …
```

Un écran, une requête — comme `programme.v_public_schedule` ou `content.v_showcase`. L'état affiché au
back-office et celui qui refuse un code à l'application **sortent de la même expression** : deux calculs
séparés divergeraient le jour où l'un des deux oublierait `valid_from`.

Une seconde vue, `negotiation.v_invitation_code_uses`, joint l'usage à `identity.role_assignments` pour
dire, ligne par ligne, si la personne a encore son accès ou quand il lui a été retiré.

---

## 5. `900_seed.sql` — deux réglages

```sql
INSERT INTO platform.settings (key, value, description, is_secret) VALUES
    ('negotiation.admission_mode', '"code"',
     'Comment on entre dans les modules réservés : "code", "approval" ou "code_and_approval" (ADR-006). Modifiable depuis le back-office, sans redéploiement.', false),
    ('negotiation.invitation_attempts',
     '{"max": 5, "window_minutes": 15, "lock_minutes": 15}',
     'Limite des essais de code d''invitation, comptés par personne.', false)
ON CONFLICT (key) DO NOTHING;
```

Le mode d'admission est un **réglage**, pas un drapeau : `platform.feature_flags` n'ouvre et ne ferme
qu'en binaire, et il y a trois valeurs. Les trois valeurs admises sont documentées ici et validées par
l'API ; la base ne les contraint pas, comme pour les autres réglages libres.

À semer aussi : la taxonomie `negotiation_network` et son premier terme (§ 3), et — pour le
développement local seulement — un espace de négociation et deux codes d'exemple, l'un du réseau,
l'autre général.

---

## 6. Transitions d'états

**Demande d'accès**

```
                 ┌──────────► approved   (un administrateur admet)
   pending ──────┼──────────► rejected   (un administrateur refuse, motif facultatif)
                 └──────────► cancelled  (la personne entre par un code entre-temps)
```

Un état final ne se rouvre jamais. Une nouvelle demande est une nouvelle ligne. **`cancelled` se dit « annulée »** ; « révoquée » est réservé à l'accès qu'on retire.

**Code d'invitation** — état dérivé, jamais stocké :

```
   not_yet_valid ──► active ──┬──► exhausted   (used_count = max_uses)
                              ├──► expired     (valid_until passé)
                              └──► revoked     (geste explicite d'un administrateur)
```

`revoked` est absorbant. **Il ne retire aucun accès déjà accordé** (ADR-006, FR-039) : le retrait est un
second geste, qui écrit dans `identity.role_assignments`.

**Accès d'une personne**, tel que « Mon accès » le lit :

```
   visiteuse ──► en attente ──► admise ──► retirée
        └──────────────────────►┘
```

---

## 7. Ce que le code ne réimplémente pas

| Invariant | Porté par | Erreur d'API à traduire |
|---|---|---|
| Un seul usage par personne et par code | `ux_invitation_code_uses_person` | déjà entré avec ce code |
| Quota jamais dépassé, même à deux entrées simultanées | `ck_invitation_codes_quota` + verrou de ligne du trigger | code épuisé |
| Une seule demande en attente | `ux_access_requests_pending_*` | demande déjà en attente |
| Portée cohérente avec l'espace | `ck_invitation_codes_scope`, `ck_access_requests_scope` | portée invalide |
| Terme de la bonne taxonomie | `tg_check_term_taxonomy` | réseau inconnu |
| Transition d'état interdite | `tg_access_request_transition` | demande déjà tranchée |
| Portée non autorisée pour le rôle | `tg_check_role_scope` | portée refusée pour ce rôle |

Chacune se teste par un test d'intégration qui provoque l'erreur PostgreSQL et vérifie le code d'erreur
français rendu — principe X, point 3.

---

## 8. Comment ce schéma arrive dans une base en service

[migration.sql](migration.sql) porte l'écart, du schéma d'aujourd'hui à celui-ci : le type et les
quatre colonnes de `identity.sessions` avec leur index, la taxonomie, les cinq tables, les deux vues,
les fonctions, les triggers et les réglages. Il est **rejouable sans dégât** — chaque objet est créé
sous condition —, ce qui permet de le répéter sur une copie de la production avant de le jouer.

En local : `pg_dump` de la base, puis le script, puis `make check-db-safe`. SQLx compile contre cette
base migrée. **Jamais `down -v`.**

Le contrôle est celui du § 13 de [DEPLOIEMENT.md](../../docs/DEPLOIEMENT.md), étape 6 : comparer le
`pg_dump --schema-only` de la base migrée à celui d'une base chargée depuis `docs/database/` — la base
modèle du harnais de test fait l'affaire, elle se reconstruit seule dès que le SQL change. Comparer
après tri des lignes.
