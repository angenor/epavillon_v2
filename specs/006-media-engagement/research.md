# Phase 0 — Décisions techniques

**Module** : Média + Engagement (B6) · **Date** : 2026-08-21 · [spec.md](spec.md) · [plan.md](plan.md)

Trente et une décisions. Chacune porte ce qui a été retenu, pourquoi, et ce qui a été écarté. Celles qui viennent d'une lecture du modèle citent le fichier et la section ; celles qui viennent d'un précédent livré citent le fichier de code.

---

## A. Structure et frontières

### R1 — Deux crates, et ils ne se connaissent pas

**Décision** : `backend/crates/modules/media` et `backend/crates/modules/engagement`, chacun avec `domain/`, `repo/`, `service/`, `routes/`, `jobs/`, `state.rs`, sur le patron de `org` et de `event`. Aucun `use` de l'un vers l'autre, aucune ligne de dépendance dans leurs `Cargo.toml` respectifs.

**Pourquoi** : le prompt l'impose et le principe II le vérifie mécaniquement. Le seul lien que le modèle établit entre les deux schémas — `engagement.direct_messages.attachment_id → media.assets` — appartient à la messagerie, hors périmètre : **aucune arête n'est donc nécessaire**, ce qui est vérifié plutôt que supposé.

**Écarté** : *un crate « services transverses »* portant les deux schémas, sur le précédent de `programme` qui porte `070` et `075`. Ce précédent ne s'applique pas : `programme` porte **un seul schéma** en deux fichiers. Ici il y a deux schémas, donc deux modules, donc deux crates — la règle ne souffre pas d'exception de commodité.

### R2 — Le module `live` n'est pas repris ici

**Décision** : ce jalon ne livre rien du schéma `live`. Le suivi d'API proposait de rattacher ses trois fonctions au « prompt des services transverses ».

**Pourquoi** : `live` est un schéma distinct, donc un module et un crate distincts. L'y greffer romprait la règle pour la seule raison qu'aucun prompt ne l'a nommé. Le point est inscrit aux points bloqués le 21/08, avec ses trois obligations d'API déjà relevées.

---

## B. Ce que la base fait déjà, et que le code ne refait pas

### R3 — Le service n'émet rien et n'enfile rien : les deux déclencheurs le font

**Décision** : ni `media` ni `engagement` n'appellent `platform.emit_event()` pour le dépôt d'un objet ou la programmation des rappels, et ni l'un ni l'autre ne met en file `media.process_asset` ni `engagement.send_reminder`.

**Vérifié dans le corps des fonctions**, pas déduit d'un précédent :

| Ce qui est écrit | Ce que la base fait alors, seule |
|---|---|
| Une ligne dans `media.assets` | `media.tg_enqueue_processing` (`050` § 3) met en file `media.process_asset` avec `NEW.id` en clé d'idempotence **et** émet `media.asset.uploaded` |
| Un appel à `engagement.schedule_session_reminders()` | insère les rappels, **met en file un travail par rappel** avec l'identifiant du rappel en clé d'idempotence, passe les lignes à `queued`, **et** émet `engagement.reminders.scheduled` (`110` § 6) |
| Un appel à `media.schedule_asset_purge()` | émet `media.asset.purge_scheduled` (`050` § 6) |

**Pourquoi c'est écrit en tête** : c'est le piège n° 1 de `identity`, de `org` et de `programme`. Ici il se présente **deux fois dans le même jalon**, et son symptôme — deux traitements par fichier, deux courriels par rappel — n'apparaît qu'en production.

**Le test qui l'attrape** : compter les lignes d'`outbox_events` et de `platform.jobs` après un dépôt et après une matérialisation. Un décompte, jamais une relecture de code.

### R4 — Le consommateur d'inscriptions branche sur le STATUT, jamais sur le type d'événement

**Décision** : le consommateur écoute tout `programme.registration.*` et lit `payload->>'status'`. Il ne cherche **jamais** `programme.registration.confirmed`.

**Pourquoi** (écart n° 126) : `programme.registration_status` vaut `registered`, `waitlisted`, `cancelled`, `attended`, `no_show` — le commentaire de `schedule_session_reminders()` nomme un événement que rien n'émet. Et le déclencheur émet `programme.registration.created` à la **création**, avec le statut en charge utile : une inscription ordinaire naît à l'état « inscrit » **par une création**. Un consommateur qui n'écouterait que les changements d'état raterait la quasi-totalité des inscriptions.

**Le test qui l'attrape** : créer une inscription directement à l'état « inscrit » et vérifier que ses rappels existent. C'est le chemin le plus courant, et c'est celui qu'une lecture du commentaire aurait cassé.

### R5 — Les invariants du modèle sont traduits, jamais recopiés

Onze refus de la base sont traduits en codes stables ([contracts/errors.md](contracts/errors.md)) : combinaison non déclarée, type non accepté, poids dépassé, forme non respectée, objet supprimé ou en quarantaine, rôle exclusif déjà pourvu, quota atteint, décalages invalides, portée de règle invalide, unicité de règle, unicité de rappel.

**Un seul contrôle est fait *avant* l'écriture, et ce n'est pas une réimplémentation** : `media.has_storage_capacity()` est la fonction que le modèle décrit comme « contrôle opposable au téléversement ». L'appeler est son emploi prévu ; le refus final reste celui de `tg_enforce_quota`, et **les deux sortent sous le même code stable** (R14).

---

## C. Le téléversement

### R6 — Le dépôt passe par l'API, en flux, et l'hypothèse H1 est révisée

**Décision** : un seul appel, `POST /media/assets` en `multipart/form-data` — les champs de métadonnées d'abord, le fichier ensuite. Une **pré-vérification facultative** (`POST /media/assets/precheck`, en JSON) permet à l'écran de refuser un fichier ou de reconnaître un doublon **avant** d'envoyer un octet.

**Pourquoi** : la spécification annonçait un protocole en trois temps avec adresse de dépôt signée, en prévoyant explicitement ce repli (H1 : « si cette exposition est refusée, le repli est un dépôt par l'API »). Trois raisons de le prendre tout de suite :

1. **Le stockage n'a pas à être exposé au navigateur.** Une adresse présignée exige que Garage réponde aux requêtes d'origine croisée et qu'il soit joignable publiquement en écriture. C'est une surface de plus, pour un gain qui n'existe que sur les très gros fichiers.
2. **L'empreinte est calculée au fil du flux**, donc vérifiée gratuitement et sans que le client ait à savoir la calculer — ce que le contrat du front ne prévoit pas : `DraftUpload` porte nom, type et poids, **jamais d'empreinte**.
3. **Un seul chemin à éprouver.** Le troisième temps — la confirmation — n'existe que parce que le serveur ne voit pas passer le fichier. Ici il le voit, et il écrit la ligne à la fin du flux.

**Ce que cela coûte, et qui est assumé** : un fond vidéo de 200 Mio traverse l'API. C'est une vidéo par édition. Actix la relaie en flux, sans la charger en mémoire.

**Écarté** : *l'adresse présignée*, pour les raisons ci-dessus. *Le dépôt en base64 dans du JSON* : la limite de corps de l'API est d'un mégaoctet, et le commentaire qui l'accompagne dit déjà pourquoi — « un envoi de fichier ne passera jamais par du JSON de toute façon ».

**La spécification est amendée en conséquence** — FR-010, FR-016, FR-017, quatre scénarios d'US1, un cas limite, H1, H2 et SC-002 —, et l'amendement est daté dans la spec plutôt que laissé implicite.

### R7 — Le stockage vit derrière un contrat, avec deux implémentations

**Décision** : un trait `ObjectStore` dans `media` — `put_stream`, `get`, `head`, `delete`, `rename` — et deux implémentations : **S3** (Garage) et **système de fichiers**. Le choix se fait par configuration (`MEDIA_STORAGE=s3|filesystem`).

**Pourquoi** : `make check-db` exécute `down -v`, ce qui **efface le layout de Garage** — le Makefile le rappelle lui-même. Des tests d'intégration qui frapperaient S3 échoueraient après chaque vérification complète, et l'on prendrait l'habitude de les sauter. Or *« une commande de vérification qui échoue toujours de la même façon finit par se lire comme du bruit »* est déjà écrit dans les pièges du dépôt.

Les tests d'intégration tournent donc sur le **système de fichiers** : ils exercent le service entier, tout le SQL, la déduplication, les quotas, le rattachement et la fabrication des variantes. **Le stockage S3 réel se vérifie à la main**, par le point de contrôle du [quickstart](quickstart.md) — exactement comme B1 a vérifié la chaîne de courriel dans Mailpit.

**Écarté** : *frapper Garage dans les tests*, pour la raison ci-dessus. *Un stockage en mémoire* : il ne prouverait pas qu'un flux de deux cents mégaoctets s'écrit sans être chargé en entier.

### R8 — La signature S3 est écrite ici, dans un fichier

**Décision** : SigV4 en propre — `crates/modules/media/src/storage/sigv4.rs` —, avec `reqwest` (déjà déclaré), `sha2` (déjà déclaré) et **`hmac` 0.12** en dépendance nouvelle. Quatre verbes : PUT, GET, HEAD, DELETE, en *path-style*, comme `.env.example` le déclare déjà (`S3_FORCE_PATH_STYLE=true`).

**Pourquoi pas le SDK d'Amazon** : il amène une quarantaine de crates et sa compilation se paie à **chaque** `make check-back` d'un dépôt sans intégration continue, tenu par une seule personne sous contrainte de temps. Et l'argument qui a fait choisir `ammonia` contre un filtre maison ne s'applique pas ici : **une erreur de signature est bruyante et totale** — 403 sur le premier dépôt —, jamais silencieuse, à l'inverse d'un filtre HTML dont le premier trou est une injection.

**Critère de bascule, écrit d'avance pour ne pas s'entêter** : si la signature n'est pas au vert contre Garage en une demi-journée, prendre `aws-sdk-s3` et consigner le changement.

**`hmac` 0.12 et non 0.13** : le workspace tient `sha2` en 0.10, dont l'API de condensat n'est compatible qu'avec `hmac` 0.12. La 0.13 est déjà dans l'arbre par transitivité — ce n'est pas la même version, et les mélanger ne compile pas.

### R9 — Le texte alternatif est exigé au dépôt d'une image

**Décision** : `POST /media/assets` refuse un `image/*` sans texte alternatif français, sous `MEDIA_ALT_TEXT_REQUIRED`, sur le champ `alt_text`.

**Pourquoi** (écart n° 129, hypothèse H3) : `ck_assets_alt_text_required` interdit à une image d'atteindre `ready` sans lui. Accepter le dépôt produirait un objet **bloqué en traitement pour toujours** et un emplacement vide inexplicable. Un refus nommant le champ est le seul symptôme visible des trois options.

**Conséquence assumée** : les trois écrans qui téléversent ne portent pas encore ce champ. L'ajout est inscrit aux obligations de B7, et la question est posée au commanditaire.

### R10 — La déduplication est tardive, et c'est ce que le modèle permet

**Décision** : le flux est écrit sur une clé **temporaire**, son empreinte calculée au passage, puis :

- empreinte déjà connue d'un objet vivant du même dépôt → le temporaire est **supprimé**, et l'objet existant est rendu ; un rattachement est posé vers lui si un rôle était visé ;
- sinon → le temporaire est **renommé** vers la clé définitive, et la ligne `media.assets` est écrite.

La pré-vérification, si le client fournit une empreinte, évite le transfert entièrement.

**Pourquoi** : `ux_assets_checksum` porte sur `(bucket, checksum)` — un objet par contenu, quel que soit son propriétaire. La déduplication économise donc **toujours** le disque, et la bande passante seulement quand le client sait annoncer l'empreinte.

**Ce que cela implique et qui est dit** (écart n° 128) : le second déposant ne consomme **aucun** quota, et l'objet appartient au premier. La suppression d'un objet **encore rattaché** est donc refusée (R11) — sans quoi le premier ferait disparaître l'image du second.

### R11 — On ne supprime pas ce qui est encore rattaché

**Décision** : `DELETE /media/assets/{id}` compte les rattachements. S'il y en a, refus `MEDIA_ASSET_IN_USE` portant le nombre. Sinon, `media.schedule_asset_purge()`.

**Pourquoi** : c'est la contrepartie de la déduplication, et c'est déjà la logique du modèle — `find_orphan_assets()` définit l'orphelin par l'absence de rattachement. Le module ne détruit pas ce qu'un autre utilise, exactement comme B4 ne détruit pas l'objet d'une pièce détachée.

---

## D. Le traitement différé d'un objet

### R12 — Trois tailles, un format par objet, et l'AVIF reste une insertion

**Décision** : `image` 0.25 (`default-features = false`, codecs `jpeg`, `png`, `webp`, `gif`), redimensionnement Lanczos3, trois déclinaisons — `lg` (1600 px de large), `md` (800), `thumb` (320) —, encodées en **JPEG** si l'image est opaque, en **PNG** si elle porte de la transparence. La liste vit dans la configuration, comme le modèle le prescrit.

**Pourquoi pas WebP ni AVIF tout de suite** : l'encodeur WebP de `image` 0.25 est **sans perte** — un WebP sans perte d'une photographie de conférence pèse plus lourd que son JPEG, ce qui prend l'objectif à l'envers. L'AVIF exige `rav1e`, dont la compilation et la lenteur d'encodage sont hors de proportion avec le besoin. **Le modèle a prévu exactement ce cas** : `rendition_format` porte déjà `webp` et `avif`, et son commentaire dit que les ajouter est une insertion, jamais une migration. Le contrat du front n'impose aucun format : `AssetSources` est indexé par `<variant>_<format>`.

**Écarté** : *appeler `vipsthumbnail` ou `cwebp` en sous-processus* — une dépendance système invisible à `cargo`, qui manque le jour du déploiement.

### R13 — L'analyse antivirus est un contrat, et son absence est déclarée

**Décision** : trait `Scanner` dans `media`, deux implémentations — `clamd` (protocole INSTREAM sur TCP, écrit ici, aucune dépendance nouvelle) et `none`. Un plafond de taille configurable ; au-delà, verdict `unsupported`.

**Pourquoi `none` rend `unsupported` et non `clean`** : `ck_assets_scan_before_ready` accepte les deux, mais `clean` affirmerait qu'un moteur a inspecté le fichier et n'a rien trouvé. `unsupported` — « aucun moteur ne sait analyser ceci » — est littéralement vrai quand aucun moteur n'est branché, et la colonne `scan_engine` porte alors `none`. Une plateforme institutionnelle doit pouvoir prouver ce qui a été inspecté ; écrire `clean` sans avoir regardé rendrait cette preuve fausse.

**Écarté** : *bloquer la mise en service quand aucun moteur n'est configuré* — l'environnement de développement n'en a pas, et rien ne serait jamais servable en local.

### R14 — Le quota se vérifie avant, et le refus final porte le même code

**Décision** : la pré-vérification et le dépôt appellent `media.has_storage_capacity()` ; le refus de `tg_enforce_quota` (`SQLSTATE 53100`) est traduit sous **le même** `MEDIA_QUOTA_EXCEEDED`, avec plafond, consommation et reste.

**Pourquoi** (écart n° 136) : `53100` est `disk_full`, un état d'erreur système. Traduit naïvement, il sortirait en 500 là où l'écran sait afficher trois chiffres. Et faire porter deux codes différents au même refus selon qu'il vient d'avant ou d'après l'écriture obligerait l'écran à traiter deux fois le même cas — la course décrite au cas limite n° 13 le rend inévitable.

---

## E. Le rattachement

### R15 — Chaque ligne de la table blanche porte sa garde, et un test le vérifie

**Décision** : `media/src/domain/guards.rs` associe chaque couple (schéma, table) de `media.attachable_roles` à la garde qui le gouverne. Toute combinaison **non associée est refusée**, jamais autorisée par défaut.

| Entité porteuse | Garde |
|---|---|
| `org.organizations` | adhésion active de référent sur la fiche, **ou** permission de gestion des organisations |
| `event.events` | permission de gestion des événements, sur la portée de l'édition, **et** périmètre d'administration |
| `programme.proposals` | adhésion active à l'organisation porteuse du dossier, **ou** permission de décision sur l'édition |
| `identity.people` | soi-même, **ou** permission de gestion des personnes |
| `content.highlights` | permission de gestion de la vitrine, sur la portée du contenu |
| `publication.articles` | permission de rédaction d'article — le module est fermé par drapeau, la garde existe quand même |

**Pourquoi** (écart n° 127) : **aucune permission `media.*` n'existe** dans le modèle, qui en déclare pour dix modules. Le droit de poser un fichier découle donc du droit d'écrire sur ce qu'il illustre — ce qui est aussi la règle la plus juste. La table blanche déclare la forme, le poids et le type ; elle ne déclare **pas** qui a le droit.

**Le test qui compte** : il lit `media.attachable_roles` et échoue si une ligne n'a pas de garde. Une table blanche est faite pour s'allonger, et une garde oubliée serait une porte ouverte.

**Écarté** : *ajouter des permissions `media.*` au modèle* — ce serait dupliquer une règle qu'il exprime déjà ailleurs. *Exiger une permission d'administration générale* — une organisation n'en a aucune, et ne pourrait plus joindre de pièce à son propre dossier.

### R16 — L'affectation par rôle est une écriture de remplacement, et elle accepte une liste

**Décision** : `PUT /media/attachments` reçoit une **liste** d'affectations `(schéma, table, entité, rôle, objet ou nul)` et les applique dans **une** transaction : poser, remplacer, ou retirer si l'objet est nul.

**Pourquoi** : c'est l'obligation que B3 a laissée — les trois déclinaisons d'une édition s'enregistrent en un geste, chacune indépendante (FR-043). Et la décision d'appel est déjà prise, aux points bloqués du 20/08 : *« le jour où Média expose le rattachement, l'écran l'appellera avant d'enregistrer l'édition »*. Le module Événements n'est donc pas modifié.

**Écarté** : *une route `PUT /events/{id}/images`* — elle vivrait sous le préfixe d'un autre module, pour une entité parmi six.

---

## F. Les rappels

### R17 — Une fonction SQL de lecture est ajoutée au modèle, et c'est la seule modification

**Décision** : ajouter `engagement.session_reminder_schedule(p_session_id uuid)` à `110_engagement.sql` § 6. Fonction `STABLE`, en lecture seule, rendant une ligne par `(offset_before, channel)` : décalage **en minutes**, canal, instant d'envoi, état consolidé, **nombre** de destinataires, motif dominant, dernier envoi. **Aucune table, aucune colonne, aucun type.**

**Pourquoi** : l'agrégat a **deux** lecteurs — la lecture par séance servie par `engagement`, et la composition de l'espace organisation servie par `programme` (FR-052, FR-053). Deux agrégations écrites séparément divergeraient au premier ajustement, et la divergence serait **silencieuse** : un nombre de destinataires faux ressemble à un nombre juste.

C'est le précédent exact de `media.attached_image()`, que `event`, `programme` et `content` appellent tous les trois, et dont l'en-tête du fichier média explique pourquoi elle est une fonction et non une vue : *« sans elle, chaque vue qui veut montrer une vignette réécrit la même jointure latérale […] Trois écritures, trois occasions de diverger — et la première divergence est silencieuse. »* Le raisonnement est mot pour mot celui-ci.

**Écarté** : *écrire l'agrégation dans chaque module* — la divergence ci-dessus. *N'exposer qu'une route et laisser le front composer* — cela contredirait FR-053 et laisserait l'espace organisation rendre une liste vide, c'est-à-dire l'écart n° 108 non refermé.

**Ce que cela impose** : `down -v` puis rechargement, et une entrée dans `docs/progression/modele.md`. C'est le prix, et il est payé une fois.

### R18 — L'état consolidé d'un groupe est celui de sa ligne la moins avancée

**Décision**, écrite dans la fonction de R17 et nulle part ailleurs :

1. s'il reste au moins une ligne `pending` → `pending` ;
2. sinon au moins une `queued` → `queued` ;
3. sinon au moins une `sent` → `sent` ;
4. sinon → l'état majoritaire parmi `skipped` et `cancelled`, avec le **motif dominant**.

**Pourquoi** : « parti » ne doit pas se dire tant qu'une personne attend encore son courriel. L'ordre choisi est celui de la prudence : le groupe n'est parti que lorsque plus rien ne reste à faire. Les données simulées du front prennent l'état de la première ligne du groupe, ce qui suffit à une démonstration mais pas à une réponse.

### R19 — Les décalages traversent en minutes, dans les deux sens

**Décision** : lecture `SELECT array(SELECT (extract(epoch FROM o)/60)::int FROM unnest(r.offsets) o)` ; écriture `offsets = (SELECT array_agg(make_interval(mins => m) ORDER BY m DESC) FROM unnest($1::int[]) m)`.

**Pourquoi** : c'est exactement le contrat du front, et son commentaire dit pourquoi — *« en MINUTES et non en texte : `'1 day'` et `'24 hours'` sont le même intervalle pour la base et deux chaînes différentes pour un `Map` »*. Cela évite aussi de traverser `interval[]`, dont la représentation binaire ne se lit pas à l'œil dans un test.

### R20 — Les décalages sont une liste, et la règle de séance remplace celle de l'édition

**Décision** : l'écriture d'une règle porte `offsets: Vec<i32>` — jamais un décalage seul. Le défaut proposé par l'API est `[2880, 1440, 60, 30]`. La lecture de la règle applicable rend l'**origine** (`session` ou `event`) et l'identifiant de l'entité dont elle vient.

**Pourquoi** : les quatre décalages sont cumulés, c'est la règle du commanditaire, le défaut du modèle et le contrat du front. Et l'origine rend la non-cumulation **vérifiable de l'extérieur** : sans elle, un administrateur ne peut pas distinguer une règle de séance à deux décalages d'une règle d'édition qu'on aurait tronquée.

### R21 — Une inscription reprise réactive ses lignes, elle n'en attend pas de nouvelles

**Décision** : à la reprise d'une inscription annulée, le service remet à `pending` les lignes `cancelled` dont l'instant d'envoi est encore devant, **puis** appelle la fonction de matérialisation pour les décalages manquants.

**Pourquoi** (cas limite n° 15) : `ux_scheduled_reminders_once` porte sur `(séance, personne, canal, décalage)` **sans condition d'état**. Les lignes annulées existent toujours ; `ON CONFLICT DO NOTHING` ne les ressuscite pas, et la personne ne recevrait plus jamais rien. C'est un défaut entièrement silencieux, et il se produit au premier désistement suivi d'un retour.

**Le même raisonnement vaut pour un report** : les lignes existantes sont remises à l'heure du nouveau créneau plutôt que recréées.

### R22 — Deux consommateurs d'outbox, nommés une fois pour toutes

**Décision** : `engagement.reminders` et `engagement.notifications`. Les noms entrent dans `platform.inbox_events` et ne se renomment pas — les renommer ferait rejouer tout l'historique.

| Consommateur | Écoute | Fait |
|---|---|---|
| `engagement.reminders` | `programme.registration.*`, `programme.session.*` | matérialise, remet à l'heure, réactive, annule |
| `engagement.notifications` | tout | cherche un type de notification **portant le code de l'événement** ; s'il n'y en a pas, ne fait rien |

**Pourquoi le second écoute tout** : `notification_types.code` suit *« la même grammaire que `outbox_events.event_type` »* — le modèle le dit dans son propre commentaire. La correspondance est donc une **donnée**, pas du code : ajouter une notification reste un INSERT, comme le modèle le promet. Un filtre synchrone exigerait un cache chargé au démarrage, qu'un type ajouté ensuite rendrait faux.

**Le coût, assumé** : une ligne d'`inbox_events` par événement relayé. C'est déjà le régime du consommateur de télémétrie.

### R23 — Quatre avis sont livrés, et les autres types restent déclarés sans être consommés

**Décision** : le module compose les destinataires et les variables pour quatre types seulement — inscription confirmée, séance annulée, séance reprogrammée, rappel de séance — plus **une diffusion directe** d'annonce de plateforme, gardée par sa permission.

**Pourquoi** : le catalogue ne dit **pas** qui est destinataire ni d'où viennent les variables — cette résolution est du code, type par type, et rien dans le modèle ne la porte (écart consigné). Les quatre retenus sont ceux dont toutes les données existent et dont un écran attend l'effet. Prétendre couvrir les dix-huit du catalogue reviendrait à écrire quatorze résolutions sans destinataire prouvé.

**Écarté** : *envoyer un avis à chaque type du catalogue* — la moitié n'a pas de destinataire déterminable aujourd'hui.

---

## G. Les courriels

### R24 — La garde d'envoi enveloppe le contrat du noyau

**Décision** : `engagement::mail::GardedMailer` implémente `kernel::mail::Mailer`, enveloppe l'expéditeur réel, et pour **tout** envoi de la plateforme : consulte `engagement.is_email_suppressed()`, écrit la trace dans `engagement.email_messages`, délègue, met la trace à jour. Il est composé dans `AppState::new` et dans `worker/main.rs`.

**Pourquoi** (écart n° 133) : les six courriels de B1 et B2 appellent le contrat directement, sans garde ni trace. Les réécrire supposerait que `identity` et `org` connaissent `engagement`, ce que le principe II interdit. Le patron retenu est celui que le noyau annonçait lui-même en B1 : *« le jour où l'envoi se réécrit ici, aucun module ne bouge »*. **Aucun module livré ne change d'une ligne.**

**Ce qui n'est pas fait, et qui est dit** : `OutgoingMail` n'est **pas** enrichie. Y ajouter le type de notification ou la personne casserait les six sites de construction des modules livrés — donc les modifierait, ce que la décision vise précisément à éviter. Les traces de B1 et B2 portent destinataire, langue, sujet et état, sans type. Le modèle les déclare nullables.

### R25 — Le rendu est une substitution de variables nommées, et rien d'autre

**Décision** : `{{variable}}`, remplacement littéral, aucune condition, aucune boucle. Une variable citée par le gabarit et absente à l'exécution **fait échouer** l'envoi en la nommant.

**Pourquoi** : le modèle l'écrit — *« le worker refuse le rendu si une variable manque : mieux vaut un job en échec visible qu'un email “Bonjour  ,” envoyé à 2 000 personnes »*. Et un langage de gabarit complet serait une dépendance d'ampleur pour des courriels transactionnels de dix lignes.

### R26 — Le HTML d'un modèle est assaini à l'écriture, avec une liste blanche de courriel

**Décision** : `ammonia` — **déjà déclarée**, employée par B4 — avec une liste blanche différente : `p`, `br`, `strong`, `b`, `em`, `i`, `ul`, `ol`, `li`, `blockquote`, `h1`–`h4`, `hr`, `a`, `img`, `table`, `thead`, `tbody`, `tr`, `td`, `th`, `div`, `span`, plus l'attribut `style` sur les balises de mise en page.

**Pourquoi une liste différente de celle de B4** : un gabarit de courriel a besoin de tableaux et de styles en ligne, parce que les clients de messagerie ignorent les feuilles de style. La liste de l'éditeur de présentation, qui refuse toute mise en forme, rendrait tout gabarit illisible.

**Le piège, et il est réel** : un gabarit contient `href="{{lien_participation}}"`. Un assainisseur qui normalise les URL peut **détruire la variable** et rendre le lien mort — un défaut qui ne se voit qu'à la réception du courriel. La politique d'URL relatives est donc réglée sur le laisser-passer, et **un test vérifie qu'un `href` porteur d'une variable survit à l'assainissement**. Sans ce test, la décision serait une intention.

**Ce qui reste hors de portée, et qui est dit** : un administrateur détenant la permission peut écrire du CSS malveillant dans un `style`. L'assainissement vise le HTML **collé** depuis ailleurs, pas un compte de confiance ; le dire vaut mieux que de laisser croire à une garantie plus forte.

### R27 — Un type sans modèle publié part quand même

**Décision** : à défaut de révision publiée, le module compose un texte de secours — sujet et corps dérivés du libellé du type et des variables —, et la trace d'expédition porte `template_id` nul, ce qui **dit** qu'aucun modèle n'a servi.

**Pourquoi** (écart n° 131) : rien ne sème de modèle. Échouer laisserait tous les rappels à terre sur une base neuve ; envoyer sans le dire empêcherait de découvrir qu'un modèle manque.

---

## H. Travaux différés, configuration, montage

### R28 — Cinq travaux différés, dont trois récurrents qui se replanifient

| Tâche | Nature | Mise en file par |
|---|---|---|
| `media.process_asset` | à l'unité | le déclencheur du modèle (R3) |
| `engagement.send_reminder` | à l'unité | la fonction du modèle (R3) |
| `media.purge_assets` | récurrente | elle-même, patron de B1 |
| `media.reconcile_quotas` | récurrente | elle-même |
| `engagement.ensure_partitions` | récurrente | elle-même |

Le worker **réarme** les trois récurrentes au démarrage, au cas où leur dernière occurrence serait morte avant d'avoir posé la suivante — c'est le mécanisme livré en B1 pour la purge des jetons, réemployé tel quel.

`engagement.ensure_partitions` appelle `platform.ensure_month_partition()` pour les mois à venir : le commentaire du modèle annonce un worker de maintenance qui n'existait pas (écart n° 137), et sans lui la purge par bascule de partition — seule raison du partitionnement — cesse de fonctionner au bout de trois mois.

**Aucun travail de reprise de courriel** : `platform.jobs` porte déjà les tentatives et le report. L'index de réémission du modèle reste donc inutilisé, et c'est consigné plutôt que compensé par un second mécanisme.

### R29 — Deux scopes sont composés par l'API, dont `/sessions` qui appartenait à un autre module

**Décision** : `api` compose `/sessions` à partir de `programme` **et** `engagement` — le second y dépose le calendrier des rappels et la règle applicable. `programme::session_routes` cesse d'ouvrir le scope lui-même et expose ses routes sans préfixe, comme `org` l'a fait pour `/organizations` en B4.

**Pourquoi** : deux `web::scope` du même préfixe **ne se complètent pas** — Actix retient le premier et rend 404 sur les routes du second. Le défaut a coûté trois routes sur vingt et une en B2 ; il est écrit dans `api/src/lib.rs`, dans `org/src/lib.rs`, et il serait difficile à défendre une seconde fois. **Aucune route de B5 ne change de chemin**, et le test qui frappe les dix-sept routes de `programme` le vérifie.

Les autres préfixes n'appartiennent qu'à un module : `/media`, `/notifications`, `/notification-preferences`, `/admin/media`, `/admin/reminder-rules`, `/admin/message-templates`, `/admin/email-suppressions`, `/internal`.

### R30 — Le webhook de délivrabilité est authentifié par un jeton, hors session

**Décision** : `POST /internal/mail-events`, protégée par un jeton porteur (`MAIL_WEBHOOK_TOKEN`), sans session et hors du contrôle d'origine. Une annonce déjà vue — reconnue par l'identifiant de message du fournisseur, dont le modèle porte l'unicité — ne crée pas de seconde trace.

**Pourquoi** : l'expéditeur réel est le site, sur un autre serveur ; c'est lui qui reçoit les retours du fournisseur. Le sens API → site est déjà authentifié par `MAIL_RELAY_TOKEN` ; le sens inverse en demande un second, et le confondre avec le premier ferait d'un jeton de sortie un jeton d'entrée.

**Le jeton absent de la configuration ferme la route** — elle rend 404, comme un module non monté : une route d'ingestion ouverte sans secret vaut mieux fermée.

### R31 — Le Makefile rend les identifiants Garage stables

**Décision** : `make garage-init` **importe** une clé fixe lue dans `.env` au lieu d'en créer une aléatoire, et `check-db` l'appelle après `up -d`.

**Pourquoi** : `down -v` efface le layout **et** la clé, si bien que les identifiants du `.env` deviennent faux après chaque vérification complète, et que le point de contrôle manuel du quickstart échoue pour une raison qui n'a rien à voir avec le code. Le Makefile le rappelle déjà par un message ; le rendre inutile vaut mieux que le rappeler.

---

## Ce qui n'a PAS été décidé ici, et pourquoi

- **Le format des variantes au-delà de JPEG et PNG** : le modèle rend l'ajout mécanique (R12). Rien ne se figera dans le code.
- **La bascule des courriels de B1 et B2 vers les modèles administrables** : dette consignée aux points bloqués, hors périmètre (H6).
- **Le rappel d'échéance d'un appel** : il suppose un périmètre de destinataires que rien ne définit (H10).
- **La reprise partielle d'un téléversement interrompu** : aucun écran ne la demande, et elle exigerait un état intermédiaire persistant.
