# Phase 0 — Décisions techniques : Sessions (B5)

> Vingt-sept décisions. Chacune porte ce qui a été retenu, pourquoi, et ce qui a été écarté. Aucune n'est un choix libre : toutes sont déduites du modèle, du contrat du front, ou d'un précédent déjà livré.

---

## R1 — On étend le crate `programme`, on n'en crée pas un second

**Décision** : tout le code de ce jalon va dans `backend/crates/modules/programme`, qui existe depuis B4.

**Motif** : la décision est déjà prise et écrite en tête de son `lib.rs` — *« Un seul crate pour tout le schéma : B4 y pose la partie propositions (fichier `070`), B5 y ajoutera la partie séances (fichier `075`) sans en créer un second »* — et ses dossiers internes sont nommés **par agrégat** (`proposals.rs`, `speakers.rs`, `reviews.rs`…) justement pour recevoir les séances sans réorganisation. Un module = un schéma = un crate ; `programme` est un schéma.

**Écarté** : un crate `sessions` séparé. Il faudrait alors décider qui porte `proposal_id`, `v_edition_stats` et `session_tracks`, et les deux crates partageraient le même schéma PostgreSQL — la définition même de ce que le principe II interdit.

**Conséquence à surveiller** : le crate passe de 17 372 à environ 25 000 lignes. Le garde-fou porte sur le **fichier**, pas sur le crate ; aucun fichier ne doit franchir mille lignes, et c'est le module où la marge est la plus mince.

---

## R2 — Le service n'émet **aucun** événement de domaine (écart n° 117)

**Décision** : zéro appel à `kernel::events::emit` dans tout ce jalon.

**Motif** : les deux déclencheurs du fichier `075` émettent déjà, et cela a été vérifié **dans le corps des fonctions**, pas déduit du précédent de B4.

- `tg_sessions_emit_events()` — `AFTER INSERT OR UPDATE OF status, starts_at, ends_at`. Il émet `programme.session.created` à l'insertion, `programme.session.<état>` à chaque changement d'état, `programme.session.rescheduled` sur un report. Son corps sort immédiatement (`RETURN NULL`) si ni l'état ni le début n'ont changé.
- `tg_registrations_emit_events()` — `AFTER INSERT OR UPDATE OF status`. Il émet `programme.registration.created` puis `programme.registration.<état>`.

Émettre à son tour produirait **deux** événements par séance créée et par inscription : deux courriels de confirmation, deux jeux de rappels planifiés. **Le doublon ne se verrait qu'en production.** C'est le piège de B1 (`anonymize_person()`), de B2 (`merge_organizations()`) et de B4 (`tg_guard_proposal_status()`), à l'identique.

**Vérification** : un test compte les lignes de `platform.outbox_events` après une acceptation produisant trois séances et exige **trois**, pas six ; un autre en compte **une** après une inscription.

**Conséquence** : `crates/contracts/src/programme.rs` n'est pas modifié. Aucune charge utile nouvelle — **son absence est la décision**, comme en B4.

---

## R3 — La naissance des séances s'accroche à **un seul** point : `transition::tenter`

**Décision** : `service/birth.rs` est appelé depuis `service/transition.rs::tenter`, dans la transaction, lorsque l'état d'arrivée est « retenu ».

**Motif** : mesuré, pas supposé — **les deux chemins d'acceptation passent par cette fonction**. La décision individuelle (`routes/admin_desk.rs::decider`) l'appelle ; l'action groupée (`transition::changer_en_groupe`) l'appelle aussi, dossier par dossier. Il n'y a donc **pas trois hameçons à poser, mais un**. Le troisième chemin d'écriture d'état, la reprise v1 (`service/backfill.rs`), **n'écrit pas `status`** : il journalise sans réveiller le déclencheur, et ne doit surtout pas créer de séances — les activités de la v1 sont importées par `910_migration_v1.sql`, pas par ce service.

**Écarté** : consommer `programme.proposal.accepted` par l'outbox. Plus orthodoxe, et moins bon : le planificateur doit avoir quelque chose à placer **au moment où l'équipe regarde son écran**. Un décalage de quelques secondes entre « retenu » et l'apparition de la carte se lit comme une panne, et personne ne rechargera pour vérifier. La transaction unique donne aussi l'atomicité : un dossier retenu **a** ses séances, ou n'est pas retenu.

**Écarté** : un déclencheur SQL. Ce serait modifier le modèle, que le prompt interdit sans justification, et la règle de composition du créneau lit `event.calls_for_proposals` — une fonction de `programme` qui lirait `event` déplacerait la frontière dans la base.

---

## R4 — Le créneau d'une séance naissante, et sa conversion **en base**

**Décision** : le début est `proposals.preferred_start_at` quand il existe. Sinon, le **premier jour de l'édition à l'heure d'ouverture quotidienne de l'appel**, composé par PostgreSQL :

```sql
(($jour::text) || ' ' || ($heure::text))::timestamp AT TIME ZONE $fuseau
```

La fin est `début + durée`, où la durée est `proposals.duration_minutes`, à défaut `calls_for_proposals.default_duration_minutes`.

**Motif** : c'est le patron de B4 (R6), déjà écrit dans `repo/proposals.rs` : *« l'écrire en Rust demanderait une base de fuseaux »*, et c'est exactement ce qui a fait tomber le formulaire du front sur `Europe/Geneva`. La base de fuseaux de PostgreSQL fait foi.

`ReglesDeLAppel` porte déjà `daily_start_time`, `default_duration_minutes` et `min/max_duration_minutes` : **rien à ajouter**. `ContexteEdition` gagne `starts_at` et `programme_published_at`, deux colonnes additives sur une lecture qui existe.

**Motif du repli** : `sessions.starts_at` et `ends_at` sont `NOT NULL` et `ck_sessions_period` exige une fin strictement postérieure. Une séance sans créneau n'est pas écrivable ; refuser l'acceptation parce que l'organisation n'a pas proposé d'horaire serait absurde. Le repli est visible dans le panneau « à placer », que l'équipe déplacera de toute façon.

---

## R5 — Un dossier **sans appel** : début de l'édition, soixante minutes

**Décision** : quand `proposals.call_id` est nul, le repli est `event.events.starts_at` et une durée de **60 minutes**.

**Motif** : la colonne est nullable — *« l'événement reste connu même si l'appel est supprimé »* — et l'IFDD peut programmer directement. Sans appel, il n'y a ni heure d'ouverture ni durée par défaut à lire. Soixante minutes est la valeur que le modèle lui-même retient pour `default_duration_minutes` : reprendre ce nombre plutôt qu'en inventer un autre garde une seule source à la convention.

---

## R6 — L'idempotence de la naissance repose sur une contrainte du modèle

**Décision** : l'insertion des séances est `ON CONFLICT (proposal_id, sequence_number) DO NOTHING`, et le service crée les rangs `1..requested_sessions`.

**Motif** : `ux_sessions_proposal_sequence UNIQUE (proposal_id, sequence_number)` existe. Une acceptation rejouée — un dossier rejeté puis remis en évaluation puis retenu, une action groupée passée deux fois — ne doit pas doubler les séances. **On ne compte pas avant d'insérer** : compter puis insérer est une course, la contrainte n'en est pas une.

**Conséquence** : si un dossier retenu voit son nombre d'occurrences augmenter puis est retenu à nouveau, les rangs manquants sont créés et les existants intacts. C'est le comportement voulu, et il tombe de la contrainte plutôt que d'une règle écrite.

---

## R7 — L'adresse d'URL d'une séance réutilise la dérivation de B4

**Décision** : `domain/slug.rs` est réemployé tel quel ; le rang est suffixé quand le dossier demande plusieurs occurrences (`atelier-mangroves-2`), et la collision dans l'édition ajoute un suffixe numérique comme pour un dossier.

**Motif** : `sessions.slug` est `NOT NULL` et `ux_sessions_slug UNIQUE (event_id, slug)`. Le contrat de naissance ne demande rien au client — il n'y a pas de client. Deux dossiers de la même édition peuvent porter le même titre ; c'est déjà traité pour les propositions (écart n° 95), et la même fonction répond.

---

## R8 — Les trois champs dérivés : ce qu'on refuse, ce qu'on accepte (écarts n° 111 et n° 112)

**Décision** :

| Champ | Contrat d'écriture | Motif |
|---|---|---|
| `time_range` | **refusé**, `field` nommé | Colonne `GENERATED ALWAYS` : PostgreSQL refuse déjà, mais par une erreur brute. Le refus nommé donne un message français plutôt qu'un 500. |
| `enforce_room_exclusivity` | **refusé**, `field` nommé | Le déclencheur ne se déclenche **pas** sur cette colonne (`UPDATE OF room_id, starts_at, event_id, is_streamed, broadcast_channel_id`). Une valeur envoyée **tiendrait**, et ferait colorer un chevauchement matériel sur une salle virtuelle. Le refus ne protège pas d'un écrasement : il protège d'une valeur fausse durable. |
| `broadcast_channel_id` | **accepté** quand la diffusion est activée · **refusé**, `field` nommé, quand elle est retirée | Le déclencheur ne le pose que lorsqu'il est **nul** (`IF NEW.is_streamed AND NEW.broadcast_channel_id IS NULL`) : il complète, il n'écrase jamais. La branche `ELSIF NOT NEW.is_streamed` **efface** — c'est là, et là seulement, qu'une valeur choisie disparaîtrait en silence. |
| `event_day_id` | **accepté**, facultatif | Régime explicitement reconnu par l'écart n° 7 : déduite quand elle n'est pas fournie, saisissable sinon. Voir R9. |

**Ce que cela corrige** : l'écart n° 7, recopié dans le prompt, demande de refuser le canal. À la lettre, cela **casserait une fonctionnalité livrée** — `SessionBroadcastPayload` porte le canal, et le commentaire du front dit pourquoi : *« L'écran laisse le choix quand l'édition en a plusieurs »*. La consigne est tenue dans son **intention** : aucune valeur envoyée n'est modifiée sans que la personne le sache.

---

## R9 — La journée de rattachement se remet à nul quand le créneau change (écart n° 113)

**Décision** : à chaque écriture de créneau où le client ne fournit pas de journée, le service écrit `event_day_id = NULL` **dans le même ordre**, pour que le déclencheur la redéduise.

**Motif** : le déclencheur ne déduit que si la colonne est nulle. Une séance déjà rattachée qu'on déplace du 12 au 14 novembre **reste rattachée au 12**, en silence — et déplacer est le geste le plus fréquent de tout l'écran.

**Écarté** : recalculer la journée en Rust. Ce serait reproduire dans le service une requête que la base porte (`day_date = (starts_at AT TIME ZONE e.timezone)::date`), contre le principe VIII, et rouvrir la question du fuseau que R4 vient de fermer.

**Écarté** : modifier le déclencheur pour qu'il redéduise à chaque changement de début. C'est la correction la plus propre, et elle demande de toucher au modèle. La mise à nul obtient le même résultat sans DDL ; si l'IFDD veut un jour rendre la déduction inconditionnelle, la ligne du service disparaîtra sans rien casser.

---

## R10 — L'écran du planificateur se lit en **une transaction en lecture seule, sur une connexion**

**Décision** : les sept lectures de `PlannerScreen` — édition, jours, salles, fils, canaux, séances, conflits — sont exécutées dans une seule transaction `READ ONLY`, sur la même connexion.

**Motif** : c'est le patron de B3 pour le détail d'une édition, et il vaut davantage ici. Les conflits sont calculés **sur les séances** ; lus à un autre instant, ils décriraient une grille que l'écran n'affiche pas. Le bandeau annoncerait un chevauchement entre deux blocs dont l'un vient d'être déplacé.

**Motif secondaire** : la leçon de B2 — une transaction qui retient deux connexions du pool sort en « service indisponible » sous charge. Une connexion, une transaction.

---

## R11 — Les écritures du planificateur rendent les conflits **lus dans la transaction**

**Décision** : `PlannerMutationResult` est composé **après l'écriture et avant la validation**, sur la connexion de la transaction.

**Motif** : le contrat l'exige — *« déplacer un bloc peut résoudre le conflit d'un autre bloc à l'autre bout de la semaine »*. `programme.detect_conflicts()` est `STABLE` et lit `programme.sessions` : appelée dans la transaction, elle voit l'écriture non encore validée. Appelée après validation, elle rendrait l'état d'une édition qu'une écriture concurrente a pu changer entre-temps, et l'écran afficherait une grille et des conflits qui ne se correspondent pas.

---

## R12 — La publication pose la date **et** fait passer « pressenti » à « programmé », en un seul ordre

**Décision** : le consommateur exécute un seul `UPDATE` posant `published_at` et, pour les seules séances encore « pressenties », `status = 'scheduled'`.

**Motif** : trois preuves concordantes que la publication est ce qui fait passer d'un état à l'autre.

1. Le modèle nomme l'état : `'scheduled'  -- programmé et publié`, contre `'planned'  -- créneau pressenti, non public`.
2. Le front le dit en toutes lettres dans sa feuille de style : *« UNE SÉANCE PROGRAMMÉE MAIS PAS ENCORE PUBLIQUE reste neutre »* pour `planner-event--planned`.
3. Les données simulées le font : les séances publiées portent `scheduled`, les séances du panneau « à placer » portent `planned`.

Ne poser que la date laisserait `scheduled` **mort**, et le calendrier du back-office colorerait en « état de travail » des séances déjà publiques.

**Ce que le contrat autorise** : la fiche des points bloqués fixe **quelles lignes** sont visées — édition, états portés par l'annonce, non encore publiées — et non ce qui y est écrit. Le prédicat est appliqué tel quel ; l'état changé l'est **à l'intérieur** de ce même jeu de lignes, jamais au-delà.

**Effet voulu, pas subi** : le changement d'état réveille `tg_sessions_emit_events()`, qui émet un `programme.session.scheduled` **par séance**. C'est précisément le signal dont B6 a besoin pour planifier les rappels. Et le déclencheur filtre lui-même : son corps sort si l'état n'a pas changé, donc une séance déjà « programmée » et simplement rendue publique n'émet rien.

---

## R13 — Le premier consommateur d'outbox du dépôt, et une addition au noyau

**Décision** : `programme` expose `event_consumers() -> Vec<Arc<dyn EventConsumer>>`, et `kernel::events::ConsumerRegistry` gagne `register_all()`, par symétrie exacte avec `JobRegistry::register_all()`.

**Motif** : le noyau porte la machinerie depuis B1 — `EventConsumer`, `ConsumerRegistry`, `claim()`, et le relais qui réserve `(consommateur, événement)` avant d'appeler `handle` — mais **aucun module ne l'utilise** : seul `TelemetryConsumer` est enregistré. B5 est le premier, B6 en aura plusieurs. `register_all` est cinq lignes, et évite que `worker/main.rs` grossisse d'une ligne par consommateur.

**Ce que le module n'a pas à écrire** : la garde de rejeu. Le relais appelle `claim()` **avant** `handle()` et n'appelle pas le consommateur si l'événement lui a déjà été livré. FR-053 et FR-057 sont donc tenus par le noyau, et le test qui rejoue l'événement mesure l'absence d'effet, pas la présence d'un code.

**Contexte d'écriture** : le relais ouvre la transaction avec `RequestContext::background("outbox")`. La publication est donc auditée comme une écriture de fond, sans acteur nommé. C'est exact — personne ne publie une séance à la main — et `platform.audit_log.actor_label` porte l'étiquette.

---

## R14 — L'écart entre le nombre annoncé et l'effet : **une seule cause possible**, nommée

**Décision** : le test de bout en bout compare `published_count` au nombre de séances effectivement publiées, et le plan nomme d'avance la seule dérive possible.

**Analyse** : l'émetteur compte sous l'instantané de **sa** transaction, à l'instant T ; le consommateur applique le prédicat à T + ε. Entre les deux, aucune route de ce jalon ne change l'état d'une séance ni ne la dépublie. La seule écriture capable de faire diverger les deux nombres est **la naissance d'une nouvelle séance** — un dossier retenu dans cet intervalle produit une séance « pressentie » et non publiée, que le prédicat attrapera.

**Conséquence** : l'effet peut dépasser l'annonce, jamais l'inverse. Le symptôme est bénin — une séance rendue publique quelques millisecondes plus tôt que prévu, alors que l'équipe venait de publier le programme. Il est **mesuré et consigné**, pas supposé nul.

**Écarté** : faire voyager la liste des identifiants plutôt qu'un prédicat. Ce serait modifier `SessionSelection`, donc le contrat et le code livré de B3.

---

## R15 — La validation dynamique est **pure**, et les options se lisent une fois

**Décision** : `domain/answers.rs` reçoit les champs du formulaire, les jeux d'options déjà résolus, et le document de réponses ; il rend un refus nommant le champ. Aucune requête, aucun accès à l'état.

**Motif** : c'est la règle la plus dense du module et celle qui se teste le mieux seule — onze types de champ, cinq règles de saisie, la présence, les clés inconnues. Le service fait **deux** lectures avant de l'appeler : les champs actifs du formulaire résolu, et les codes des taxonomies auxquelles ces champs renvoient. Une lecture par champ à choix serait un N+1 sur un formulaire de six questions.

---

## R16 — Cinq clés de `validation` sont honorées ; les autres sont ignorées

**Décision** : `minLength`, `maxLength`, `pattern`, `min`, `max`. Toute autre clé est **ignorée sans erreur**.

**Motif** : le commentaire du modèle dit « min, max, pattern… » et les données simulées emploient `maxLength`. Ces règles sont des **données écrites par un administrateur** depuis le back-office : refuser un formulaire porteur d'une clé qu'une version future de l'API ne connaît pas encore fermerait les inscriptions d'une édition entière au premier déploiement décalé. Ignorer est le comportement sûr ; la trace journalise la clé inconnue.

---

## R17 — Le vide vaut absence, comme le déclencheur ; le choix multiple vide est refusé

**Décision** : une réponse présente mais vide est traitée comme absente, exactement comme `COALESCE(NEW.answers ->> f.code, '') = ''` le fait en base. Un champ **à choix multiple** obligatoire exige au moins une valeur, ce que la base ne vérifie pas — `->>` d'un tableau vide rend `'[]'`, non vide.

**Motif** : diverger de la base dans le sens **strict** est sans risque — l'API refuse avant elle. Diverger dans l'autre sens produirait un refus du déclencheur que le service n'aurait pas su expliquer. La seule divergence est nommée ici plutôt que découverte.

---

## R18 — Une réponse « pays » est le code ISO à deux lettres (écart n° 11)

**Décision** : la valeur attendue est le code ISO 3166-1 alpha-2, en majuscules, validé contre `reference.countries.iso2`.

**Motif** : l'écart n° 11 laissait deux formes possibles et disait qu'un export mêlant les deux serait irrécupérable. Ce n'est pas un arbitrage du commanditaire mais une décision d'API. Le code ISO est lisible dans un export, stable si la fiche d'un pays est refaite, et c'est déjà ce que les données simulées retiennent (`country: 'SN'`). Un champ de type `taxonomy_term` suit la même logique : le **code** du terme, jamais son identifiant ni son libellé.

---

## R19 — La jauge et la liste d'attente se sérialisent par un verrou de ligne (écart n° 124, **nouveau**)

**Décision** : toute écriture d'inscription commence par `SELECT … FROM programme.sessions WHERE id = $1 FOR UPDATE`.

**Motif** : le contrôle de jauge du déclencheur **n'est pas sûr en concurrence**. Il exécute un `count(*)` sans verrou : sous `READ COMMITTED`, deux transactions simultanées lisent toutes deux neuf places prises sur dix et insèrent toutes deux. La position d'attente souffre du même défaut — `COALESCE(max(waitlist_position), 0) + 1` donne deux fois le même rang —, et **aucun index unique ne porte sur cette colonne**.

**Ce que le verrou coûte** : les inscriptions à une même séance se sérialisent. Une séance reçoit quelques dizaines à quelques centaines d'inscriptions sur plusieurs semaines ; deux inscriptions strictement simultanées sont déjà l'exception. Les inscriptions à des séances différentes ne se gênent pas.

**Écarté** : un verrou consultatif (`pg_advisory_xact_lock`). Il faudrait dériver une clé entière d'un `uuid`, avec le risque de collision entre deux séances, pour économiser la lecture d'une ligne qu'on lit de toute façon.

**Vérification** : cent inscriptions concurrentes sur une séance de dix places donnent **exactement** dix confirmées, et des positions d'attente qui se suivent sans trou ni doublon.

---

## R20 — L'annulation promeut dans la même transaction, exactement le nombre de places libérées (écart n° 116)

**Décision** : annuler une inscription **confirmée** appelle `programme.promote_from_waitlist(session, 1)` dans la transaction de l'annulation, sous le même verrou de ligne. Annuler une inscription **en attente** ne promeut personne.

**Motif** : le contrôle de capacité du déclencheur ne porte que sur l'insertion (`TG_OP = 'INSERT'`), et une promotion est une mise à jour. Promouvoir plus que le nombre de places libérées ferait dépasser la jauge **sans un mot**, et cela ne se verrait que le jour de l'activité. Le service compte donc ce qu'il libère : une annulation, une place, une promotion.

**Effet voulu** : la promotion change l'état, donc `tg_registrations_emit_events()` émet `programme.registration.registered` pour la personne promue — l'avis qu'elle attend. Le service, lui, n'émet rien (R2).

---

## R21 — Le déclencheur revalide à **chaque** changement d'état : ce que cela interdit (écart n° 125, **nouveau**)

**Constat** : `tg_registrations_validate` se déclenche `BEFORE INSERT OR UPDATE OF answers, status`. Deux de ses contrôles ne sont pas bornés à l'insertion, et s'appliquent donc aussi à une **annulation** :

- `IF v_session.status IN ('cancelled') THEN RAISE …` — **on ne peut pas annuler son inscription à une séance annulée** ;
- le contrôle des réponses obligatoires — **une question rendue obligatoire après coup bloque l'annulation d'une inscription ancienne**, qui ne la porte pas.

**Décision** : consigné, non corrigé. Le service **traduit** les deux refus en réponse nommée plutôt que de laisser sortir un 500, et n'essaie pas de les contourner — il n'y a pas de contournement sans modifier le déclencheur.

**Motif de ne pas corriger** : le premier symptôme est sans conséquence pratique — une séance annulée ne réunit personne, et rester « inscrit » à ce qui n'a pas lieu ne coûte rien. Le second est réel et se réveillerait si l'IFDD ajoutait une question obligatoire en cours de campagne ; il est écrit ici pour être reconnu au premier signalement plutôt que cherché.

---

## R22 — Le consentement aux réponses sensibles : écriture hors schéma n° 3, bornée

**Décision** : le service écrit une ligne dans `identity.consents` — `(person_id, purpose = 'registration_sensitive_data', is_granted = true, policy_version, source = 'registration_form', ip_address)` — dans la transaction de l'inscription, depuis un fichier unique, `repo/consents.rs`. **Une seule finalité**, quel que soit le nombre de champs sensibles.

**Motif** : le modèle prévoit exactement cet usage — la colonne `source` documente `'registration_form'` — et **aucun autre module ne peut poser ce consentement au moment où il est donné**. La preuve doit vivre dans la même transaction que la donnée qu'elle couvre : sinon on refuse sans preuve, ou l'on accepte et la preuve se perd si le relais meurt.

**Écarté** : un contrat d'événement consommé par le module Identité. Il n'y a pas de consommateur côté `identity`, et la preuve serait écrite **après** l'inscription : refuser faute de consentement deviendrait impossible à garantir.

**Écarté** : ranger le consentement dans `registrations.answers`. Ce document a pour clés les codes de champs qu'un administrateur renomme ; la preuve d'un consentement RGPD n'y survivrait pas, et disparaîtrait avec l'inscription.

**Bornes** : ce fichier n'écrit que cette table, n'y lit que la vue d'état courant, et ne touche jamais `identity.people`, les comptes, les rôles ni les demandes RGPD. C'est la troisième écriture hors schéma du crate, après les thématiques (B4, R11) et la création d'une personne inconnue (B4, R12).

**Une finalité, pas une par champ** : multiplier les finalités multiplierait les lignes de preuve sans que personne l'ait demandé, et rendrait le retrait du consentement ingérable — retirer lequel ?

**`policy_version`** est `NOT NULL` : elle vient de la configuration du service (`PRIVACY_POLICY_VERSION`), comme le seuil de verrouillage de B1. C'est un réglage d'exploitation : la mettre en base la rendrait modifiable par migration seulement.

---

## R23 — L'inscrit sans compte réutilise la création bornée de B4

**Décision** : `repo/people.rs::trouver_ou_creer`, livré en B4, sert tel quel. L'identité vient de **champs dédiés** de la charge utile — adresse, prénom, nom — et **jamais** des réponses au formulaire.

**Motif** : le précédent est établi deux fois (l'invité d'une organisation en B2, l'intervenant inconnu en B4) et borné de la même façon : adresse, prénom, nom, civilité, et rien d'autre. Ni compte, ni mot de passe, ni rôle, ni visibilité d'annuaire.

**Pourquoi des champs dédiés** : les codes de champs sont des données qu'un administrateur renomme depuis le back-office. Y loger l'identité ferait dépendre la création d'une personne d'un libellé modifiable — et le jour où quelqu'un renommerait `email` en `courriel`, les inscriptions sans compte cesseraient de rattacher qui que ce soit, en silence.

**`allows_anonymous`** commande : faux, une session est exigée et le refus le dit.

---

## R24 — Dix-sept routes, et le préfixe `/admin/planner` composé comme B3 l'a préparé

**Décision** : `api/src/lib.rs` compose `/admin/planner` à partir des **deux** modules, sur le patron de `/people` et de `/organizations`.

**Motif** : c'est ce que B3 a annoncé en toutes lettres — *« B5 y déposera les routes du planificateur de séances »* — et le défaut qu'il évite a déjà coûté **trois routes muettes sur vingt et une** en B2 : deux `web::scope` du même préfixe ne se complètent pas, Actix retient le premier et rend 404 sur les routes du second.

**Chemins littéraux avant chemins paramétrés** : `/sessions/conflicts` (deux segments) et `/sessions/{id}/…` (trois) ne se recouvrent pas, et `/registrations/mine` ne recouvre pas `/registrations/{id}/…`. Le risque de capture n'existe pas ici — il n'existe que lorsque les méthodes **et** le nombre de segments coïncident, comme B4 l'a mesuré. Le découpage en groupes littéraux et paramétrés est repris quand même, pour que la règle soit tenue par la structure.

**Un chemin du contrat n'est pas servi** (écart n° 121) : `/sessions/publication-readiness`. B3 sert la même réponse sous `/admin/planner/readiness`, aucun écran n'appelle le premier, et livrer deux chemins pour une même lecture dans deux modules différents garantit qu'ils divergeront.

---

## R25 — Aucun travail différé ; un consommateur

**Décision** : `programme::job_handlers()` n'est pas créé. Le module n'ajoute aucune entrée à `platform.jobs`.

**Motif** : vérifié plutôt que constaté. Rien de ce jalon n'a d'effet à échéance. Le passage d'une séance en « en cours » puis « terminée » **n'est écrit par personne**, et c'est exact : `v_public_schedule` calcule l'état temporel depuis les instants, et le front l'affiche — un travail périodique qui écrirait le même fait produirait deux sources pour une seule vérité. Les rappels appartiennent à B6.

**Ce que le worker gagne** : une ligne, l'enregistrement du consommateur de publication.

---

## R26 — Les traversées de type, héritées et deux nouveautés

**Décision** : les quatre traversées de B1 à B4 servent telles quelles — énumération en `text`, `i18n_text` en `jsonb`, `numeric` en `float8`, domaines par double transtypage. Deux ajouts :

- **`tstzrange` se lit en `text`.** Le contrat du front déclare `TsTzRange = string` et les données simulées écrivent `["…","…")` — la représentation textuelle de PostgreSQL. Un `::text` dans la requête suffit ; introduire `PgRange` obligerait à recomposer cette chaîne côté Rust pour rendre exactement la même.
- **`answers` et `options` restent des `serde_json::Value`.** C'est la conséquence voulue du formulaire configurable, et l'écart n° 6 interdit de la « corriger ».

**Rappel de B3** : toute colonne lue depuis une **vue** est rendue nullable par SQLx, une vue ne portant aucune contrainte de nullité. `v_public_schedule` s'annote donc colonne par colonne (`AS "id!"`).

---

## R27 — Aucune dépendance nouvelle

**Décision** : rien n'est ajouté à `backend/Cargo.toml`.

**Motif** : la validation dynamique n'a besoin que de `serde_json` et de la bibliothèque standard. Le motif d'un champ (`pattern`) est le seul point qui pourrait appeler une bibliothèque d'expressions régulières — `regex` **est déjà une dépendance transitive du workspace**, mais l'ajouter comme dépendance directe demanderait une décision consignée. Elle est prise ici : `pattern` est honoré, et si l'expression est invalide, la règle est **ignorée** avec une trace, jamais transformée en refus d'inscription. Une expression fautive est une donnée d'administrateur, pas une faute de l'inscrit.

**Vérification** : `regex` est déclaré au workspace avant usage, et la décision est reportée dans `docs/progression/decisions/`.
