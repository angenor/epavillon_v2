# Feature Specification: Média + Engagement (B6)

**Feature Branch**: `main` (aucun crochet de branche n'est enregistré ; le dossier de la fonctionnalité est `specs/006-media-engagement`)

**Created**: 2026-08-21

**Status**: Draft

**Input**: User description: "Modules MÉDIA + ENGAGEMENT de l'API ePavillon v2 (Rust + Actix Web + SQLx). Téléversement vers Garage avec déduplication par empreinte, génération des variantes en tâche de fond, quotas. Notifications, modèles multilingues, rappels sans doublon. Plus les exigences issues des écarts n° 32 et n° 34, les quatre décalages cumulés et la règle séance-sinon-édition. Livrable : deux crates, `backend/crates/modules/media` et `backend/crates/modules/engagement`."

---

## Amendements

| Date | Ce qui change | Pourquoi |
|---|---|---|
| 21/08 | **Le téléversement passe de trois temps à un seul geste**, précédé d'une annonce facultative — FR-010, FR-011, FR-016, FR-017, US1 scénarios 1 à 4 et 8, cas limite n° 4, H1, H2, SC-002 | Décidé par le plan (`research.md` § R6). H1 annonçait ce repli : exposer le stockage en écriture au navigateur exigerait une déclaration d'origines croisées et une joignabilité publique, pour un gain qui n'existe que sur les très gros fichiers. Le contrat du front ne porte aucune empreinte, donc le serveur la calcule pendant la réception — et la vérifie gratuitement |
| 21/08 | **Les déclinaisons d'image sont trois tailles, pas deux formats** — US2 | Décidé par le plan (§ R12). L'encodeur WebP disponible est sans perte, ce qui alourdirait une photographie au lieu de l'alléger, et l'AVIF exige un encodeur hors de proportion avec le besoin. Le modèle a prévu ce cas : ajouter un format est une insertion, jamais une migration |

---

## Contexte

**Ce jalon ferme les deux trous par lesquels tout le reste fuit.** Le premier est un fichier qui n'arrive nulle part : depuis A4, le formulaire de soumission *retient* les documents et les photos sans les déposer, B3 *accepte* les trois identifiants d'image d'une édition **sans les poser** (`event/src/service/edition_write.rs`, en-tête), la vitrine montre des emplacements vides, et aucune route de la plateforme ne sait recevoir un octet. Le second est un courriel qui ne part pas : B5 a livré les inscriptions, et rien ne rappelle une séance à qui s'y est inscrit — l'espace organisation sert `reminders: []` depuis B4 (écart n° 108), en le disant.

Le modèle fait autorité. `docs/database/050_media.sql` et `docs/database/110_engagement.sql` ont été relus en entier. Le premier porte les objets stockés et leur cycle de vie, les variantes, la table blanche des rattachements, les quotas et la purge des orphelins ; le second, le catalogue ouvert des types de notification, les notifications in-app, les préférences par type **et** par canal, les modèles de messages versionnés et multilingues, le journal d'expédition et sa délivrabilité, les règles de rappel et leur matérialisation. S'y ajoutent `010_platform.sql` (réglages, file de travaux, outbox, garde de rejeu, audit, partitions mensuelles), `075_programme_sessions.sql` § 1 et § 4 (les séances, les inscriptions, et surtout **les deux déclencheurs qui émettent déjà**), `060_events.sql` § 5 (l'appel et ses échéances), `030_identity.sql` (les permissions et le périmètre d'administration), `115_content.sql` § 5 (les trois rôles média de la vitrine) et `040_organizations.sql` (le registre de fusion, auquel `media` est déjà inscrit).

**Deux crates, jamais un.** Un module = un schéma = un crate : `backend/crates/modules/media` porte le schéma `media`, `backend/crates/modules/engagement` porte le schéma `engagement`. **Ils ne dépendent pas l'un de l'autre**, pas plus que d'un autre crate de module : uniquement de `kernel` et de `contracts`. Le seul lien que le modèle établit entre eux — la pièce jointe d'un message direct — appartient à la messagerie, hors périmètre.

**Cinq faits gouvernent tout ce qui suit, et chacun coûterait cher découvert en chemin.**

1. **Les déclencheurs émettent et enfilent déjà.** Insérer une ligne dans `media.assets` met en file `media.process_asset` **et** émet `media.asset.uploaded` (`050` § 3). `engagement.schedule_session_reminders()` émet `engagement.reminders.scheduled` **et** met en file un travail par rappel (`110` § 6). Un service zélé enverrait tout en double : deux traitements par fichier, deux courriels par rappel — visibles seulement en production. C'est le piège n° 1 de `identity`, de `org` et de `programme`, et il se répète ici deux fois.
2. **`programme.registration.confirmed` n'existe pas.** Le commentaire de `schedule_session_reminders()` annonce que la fonction est appelée sur cet événement. Or `programme.registration_status` vaut `registered`, `waitlisted`, `cancelled`, `attended` ou `no_show` — jamais `confirmed` — et le déclencheur émet `programme.registration.created` à la création, puis `programme.registration.<statut>` au changement d'état. Un consommateur écrit d'après ce commentaire **ne serait jamais réveillé**, et aucun rappel ne partirait, sans qu'aucune erreur ne le signale (écart n° 126).
3. **Les quatre décalages sont cumulés.** `{2 days, 1 day, 1 hour, 30 minutes}` est le défaut de `reminder_rules.offsets`, et les quatre rappels partent. Ce n'est pas un choix parmi quatre : une écriture qui accepterait « le » rappel au singulier contredirait la règle du commanditaire avant le premier envoi.
4. **La règle applicable est celle de la séance si elle existe, sinon celle de son édition, sans cumul.** C'est ce que fait la fonction SQL, et c'est ce qui permet à l'administrateur de savoir ce qui va partir. Fusionner les deux jeux de décalages rendrait la question sans réponse.
5. **La base ne stocke jamais d'URL.** Un objet est décrit par `(bucket, object_key)` ; l'adresse publique est composée à la lecture depuis un réglage. Aucune réponse d'API ne rend une clé d'objet nue, aucune écriture n'accepte une URL.

**Le front existe depuis le 16 août et consomme des données simulées.** Ses contrats — `frontend/app/types/media.ts`, `types/engagement.ts`, `ReminderSlot` et `TrackedSession` dans `types/organization-workspace.ts`, `DraftUpload` et `DraftDocument` dans `types/proposal-form.ts`, `EditionFormValues.images` et `EditionDetail.images` dans `types/admin-events.ts`, `ShowcaseMediaSlot` dans `types/admin-showcase.ts`, `HighlightMediaRule` dans `types/content.ts` — **sont le contrat de cette API**. Ils ne se renégocient pas. `mocks/reminders.ts` rejoue déjà `schedule_session_reminders()` en TypeScript et **rend l'agrégat attendu** : il vaut spécification exécutable de l'écart n° 34.

**L'écart n° 32 est déjà refermé, et ce jalon ne le rouvre pas.** Le prompt B6 demande d'assainir le HTML de la présentation détaillée d'un dossier ; B4 l'a livré le 21/08 — `programme/src/domain/sanitize.rs`, appelé par `service/draft_write.rs`, liste blanche relevée sur la barre d'outils de l'éditeur, assainissement **à l'écriture** et unique chemin d'écriture de la colonne. Le refaire ici serait un second filtre, et il ne pourrait de toute façon pas y vivre : la colonne appartient au schéma `programme`, qu'aucun de ces deux crates n'a le droit de toucher. **Ce que ce jalon doit à cet écart, c'est de l'appliquer là où il ne l'est pas encore** : le corps HTML des modèles de courriel, qu'un administrateur saisit et que la plateforme envoie à des milliers de personnes.

Le socle existe depuis B1, les frontières depuis B2, l'édition depuis B3, le dossier depuis B4, la séance depuis B5 : `kernel` (contexte de requête, erreurs à code stable, unique porte d'écriture, garde d'autorisation testant permission **et** portée, contrat d'envoi de courriel, file de travaux, garde de rejeu d'outbox), `contracts`, `api`, `worker`. **Ce module ne les réinvente pas.** La règle de frontière posée en B2 s'applique telle quelle : *un module lit hors de son schéma quand la question porte sur ses propres entités, il n'y écrit jamais, et il n'appelle jamais un autre module.*

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Un fichier arrive, et il n'occupe la place qu'une fois (Priority: P1)

Une chargée de communication d'une organisation choisit le logo de sa fiche. Le fichier part sur le stockage, l'API en garde la description, et l'écran reçoit un identifiant d'objet. Si ce même logo a déjà été déposé par quelqu'un — la plateforme en compte des dizaines identiques — **il n'occupe pas la place une seconde fois** : l'API reconnaît le contenu à son empreinte et rend l'objet existant. Si l'organisation a épuisé son espace, ou si le fichier ne convient pas au rôle visé, elle l'apprend **avant** d'avoir envoyé quoi que ce soit, avec ce qui lui reste.

**Why this priority**: sans elle, aucun fichier n'entre dans la plateforme. Les trois écrans qui téléversent — soumission, édition, vitrine — sont livrés et attendent cette route ; les cinq histoires suivantes n'ont pas d'objet sans un objet à traiter.

**Independent Test**: déposer une image de 2 Mio, obtenir un identifiant d'objet et vérifier que le fichier est réellement sur le stockage à la clé annoncée ; redéposer le même contenu sous un autre nom de fichier et vérifier qu'aucun second objet n'est écrit et qu'aucun octet supplémentaire n'est conservé ; remplir le quota d'une organisation puis tenter un dépôt et obtenir un refus nommant l'espace restant.

**Acceptance Scenarios**:

1. **Given** une personne autorisée à écrire sur une entité porteuse, **When** elle annonce un fichier par son nom, son type, son poids et le rôle visé — **sans l'envoyer** —, **Then** l'API répond ce qu'elle en ferait : accepté, refusé pour son type, refusé pour son poids, refusé faute d'espace, ou « cet objet existe déjà » si une empreinte est fournie et connue.
2. **Given** une personne autorisée, **When** elle envoie les métadonnées et le fichier en un seul geste, **Then** l'API calcule l'empreinte pendant la réception, écrit l'objet à sa clé définitive, enregistre sa description et rend son identifiant.
3. **Given** une empreinte déjà connue du même dépôt de stockage, **When** le fichier arrive, **Then** **il n'est pas conservé**, aucun second objet n'est écrit, et l'identifiant rendu est celui de l'objet existant.
4. **Given** un fichier dont la réception s'interrompt, **When** le flux se rompt, **Then** aucune description n'est écrite et ce qui avait été reçu est retiré du stockage.
5. **Given** une organisation dont l'espace restant est inférieur au poids annoncé, **When** elle déclare le fichier, **Then** elle est refusée **avant** l'obtention d'une adresse de dépôt, avec le plafond, l'espace consommé et l'espace restant.
6. **Given** un refus d'espace prononcé par la base au moment de l'écriture, **When** il remonte, **Then** il est rendu sous le même code stable et le même message que le refus préalable — jamais une erreur anonyme.
7. **Given** un fichier dont le type ou le poids ne convient pas au rôle visé, **When** il est déclaré, **Then** il est refusé en nommant le rôle, ce qui était attendu et ce qui a été reçu.
8. **Given** une annonce de fichier sans envoi, **When** rien ne suit, **Then** **rien n'a été écrit** : l'annonce ne réserve ni espace, ni clé, ni identifiant.
9. **Given** un objet écrit, **When** on relit sa description, **Then** elle porte une **adresse composée** et jamais une clé de stockage nue.
10. **Given** une écriture d'objet, **When** elle aboutit, **Then** le service **n'émet aucun événement de dépôt et ne met aucun travail en file** : le déclencheur du modèle fait déjà les deux.

---

### User Story 2 — Le fichier devient servable sans faire attendre personne (Priority: P1)

Le fichier déposé est aussitôt visible : l'écran affiche l'original pendant que la plateforme travaille. En arrière-plan, ses dimensions sont relevées, il est analysé, ses déclinaisons sont fabriquées — une version large, une version de carte, une vignette. Quand tout est prêt, l'objet devient servable au public. Si l'analyse trouve quelque chose, il est mis de côté et n'est **jamais** servi.

**Why this priority**: c'est ce qui remplace les six colonnes d'URL de la v1. Sans le traitement différé, ou bien l'écran attend la fabrication des variantes — plusieurs secondes sur une photographie de conférence —, ou bien la plateforme ne sert que des originaux de dix mégaoctets à des visiteurs sur téléphone.

**Independent Test**: déposer une photographie, constater qu'elle s'affiche immédiatement et que ses déclinaisons apparaissent ensuite sans nouveau geste ; déposer un fichier reconnu comme dangereux et constater qu'aucune lecture ne le rend, ni en original ni en variante.

**Acceptance Scenarios**:

1. **Given** un objet fraîchement décrit, **When** on demande l'image rattachée à son entité, **Then** l'adresse de l'original est rendue et la liste des déclinaisons est **vide mais présente** — l'écran affiche l'image, pas un trou.
2. **Given** une image déposée, **When** le traitement s'achève, **Then** ses dimensions sont relevées, ses déclinaisons sont écrites, et elle devient servable au public.
3. **Given** un fichier vidéo ou audio, **When** le traitement s'achève, **Then** sa durée est relevée ; aucune déclinaison d'image n'est fabriquée.
4. **Given** un document, **When** le traitement s'achève, **Then** aucune dimension n'est relevée, aucune déclinaison n'est fabriquée, et l'objet devient servable.
5. **Given** un fichier dont l'analyse est positive, **When** le traitement s'achève, **Then** l'objet est mis en quarantaine, **aucune lecture ne le rend**, et tout rattachement le visant est refusé.
6. **Given** un traitement interrompu — worker arrêté en cours de route —, **When** le worker repart, **Then** le travail est repris et **aucune déclinaison n'est fabriquée deux fois**.
7. **Given** un traitement en échec définitif, **When** il abandonne, **Then** l'objet porte son échec et son motif, il n'est pas servi, et l'écran peut le distinguer d'un objet absent.
8. **Given** des déclinaisons fabriquées, **When** elles sont écrites, **Then** l'espace qu'elles occupent est compté dans la consommation de l'organisation propriétaire.
9. **Given** une image servable, **When** on la relit, **Then** ses déclinaisons sont rendues sous une forme directement exploitable par un `<picture>` — adresse, largeur, hauteur, poids par déclinaison.

---

### User Story 3 — Un fichier se rattache à ce qu'il illustre, et jamais à autre chose (Priority: P1)

Une administratrice enregistre les trois visuels d'une édition : le bandeau panoramique, l'image de couverture, la vignette carrée. Chacun est rattaché à son rôle. Elle ne peut pas poser deux bandeaux, ni glisser un document à la place d'une image, ni déposer un 4:3 là où un 32:9 est attendu — la plateforme le lui dit avec le rapport reçu et le rapport attendu. Remplacer un visuel remplace celui qui s'y trouvait ; le retirer ne détruit pas le fichier, qui peut servir ailleurs.

**Why this priority**: c'est l'obligation que B3 a explicitement laissée à ce jalon — le formulaire d'édition envoie trois identifiants d'image que le service **accepte sans les poser**. Tant qu'elle n'est pas livrée, un écran livré ment à son utilisateur.

**Independent Test**: enregistrer les trois visuels d'une édition puis relire sa fiche et constater les trois déclinaisons résolues ; tenter un second bandeau et obtenir un refus nommant le rôle ; tenter un carré comme bandeau et obtenir un refus citant le rapport attendu ; retirer un visuel et vérifier que l'objet stocké existe toujours.

**Acceptance Scenarios**:

1. **Given** une combinaison entité × rôle déclarée à la table blanche, **When** un objet y est rattaché, **Then** le rattachement est écrit et la lecture de l'entité rend l'image.
2. **Given** une combinaison non déclarée, **When** un rattachement est tenté, **Then** il est refusé en nommant l'entité et le rôle, jamais en 500.
3. **Given** un rôle qui n'accepte qu'un objet et qui en porte déjà un, **When** un second est rattaché **par une écriture de remplacement**, **Then** l'ancien rattachement est retiré et le nouveau posé, dans la même transaction.
4. **Given** ce même rôle, **When** un second objet est **ajouté** sans remplacement, **Then** l'écriture est refusée en disant que le rôle n'accepte qu'un objet.
5. **Given** un rôle imposant une forme, **When** un objet mesuré ne la respecte pas, **Then** le rattachement est refusé en citant les dimensions reçues, le rapport obtenu, le rapport attendu et la tolérance.
6. **Given** un rôle imposant une forme et un objet **dont les dimensions n'ont pas été relevées**, **When** il est rattaché, **Then** il est accepté : c'est le relevé qui a échoué, pas le cadrage.
7. **Given** un objet en quarantaine ou supprimé, **When** il est rattaché, **Then** le rattachement est refusé.
8. **Given** un formulaire d'édition portant trois identifiants d'image, **When** il est enregistré, **Then** les trois rôles sont posés, remplacés ou retirés indépendamment, et une valeur nulle **retire** la déclinaison sans toucher aux deux autres.
9. **Given** un rattachement retiré, **When** on relit l'objet stocké, **Then** **il existe toujours** : le module ne détruit pas un fichier qui peut servir ailleurs.
10. **Given** une entité portant plusieurs objets d'un rôle multiple, **When** on les relit, **Then** ils arrivent dans l'ordre de tri déclaré, et cet ordre est modifiable.
11. **Given** un objet partagé par déduplication entre deux entités, **When** on lui donne un texte alternatif propre à un usage, **Then** ce texte prime pour cet usage-là et ne touche pas l'autre.
12. **Given** un compte détaché sur une édition, **When** il tente de rattacher un visuel à une **autre** édition, **Then** il reçoit le même refus qu'avec une édition inexistante.

---

### User Story 4 — L'organisation lit le calendrier des rappels de sa séance, en quatre lignes et sans un nom (Priority: P2)

Une organisation ouvre le dossier d'une activité retenue. Sous chaque séance, quatre lignes : « 2 jours avant — parti — 40 destinataires », « 1 jour avant — parti — 40 », « 1 heure avant — à venir — 40 », « 30 minutes avant — à venir — 40 ». Elle sait ce qui est parti, ce qui va partir, et combien de personnes viendront. Elle ne sait pas **qui**.

**Why this priority**: c'est l'écart n° 34, relevé le 17/08 et explicitement renvoyé à ce jalon. Le contrat existe déjà côté front, et sans agrégation côté serveur l'écran chargerait cent soixante lignes pour en afficher quatre — en se voyant remettre une liste nominative qu'il devrait s'interdire d'utiliser.

**Independent Test**: sur une séance à quarante inscrits et une règle à quatre décalages, demander le calendrier et compter **quatre** lignes portant chacune quarante destinataires ; balayer la réponse entière et n'y trouver aucun identifiant de personne, aucun nom, aucune adresse.

**Acceptance Scenarios**:

1. **Given** une séance dont les rappels sont matérialisés, **When** son calendrier est demandé, **Then** la réponse porte **une ligne par (décalage, canal)**, avec le décalage exprimé en minutes, le canal, l'instant d'envoi, l'état consolidé, le nombre de destinataires et l'instant du dernier envoi.
2. **Given** cette réponse, **When** on la balaye entièrement, **Then** elle ne contient **ni identifiant de personne, ni nom, ni adresse électronique, ni identifiant d'inscription**.
3. **Given** une règle à deux canaux, **When** le calendrier est rendu, **Then** un même décalage produit **deux** lignes : deux canaux sont deux envois, et les fondre en cacherait un.
4. **Given** un groupe dont toutes les lignes sont parties, **When** l'état est consolidé, **Then** il vaut « parti » ; s'il en reste une seule à traiter, l'état du groupe est celui de la ligne **la moins avancée**.
5. **Given** un groupe dont toutes les lignes ont été écartées ou annulées, **When** l'état est consolidé, **Then** il le dit, et le motif le plus fréquent accompagne la ligne.
6. **Given** les lignes rendues, **When** on les lit, **Then** elles sont ordonnées **du plus lointain au plus proche du début** — l'ordre dans lequel elles partent.
7. **Given** une séance sans règle applicable, **When** son calendrier est demandé, **Then** la réponse est une liste **vide accompagnée du fait qu'aucune règle ne s'applique** — jamais une liste vide muette, qui se confondrait avec « tout est parti ».
8. **Given** une séance annulée, **When** son calendrier est demandé, **Then** les lignes portent l'état « écarté » et le motif d'annulation.
9. **Given** une personne sans lien avec la séance, **When** elle demande le calendrier, **Then** elle reçoit le même refus qu'avec une séance inexistante.
10. **Given** l'espace d'une organisation, **When** il est composé, **Then** chaque séance suivie porte ce même calendrier, dans la forme exacte que le front attend, et **le contrat de B5 cesse de rendre une liste vide**.

---

### User Story 5 — Le rappel part une fois, et une seule (Priority: P2)

Une personne s'inscrit à une séance. Quatre rappels sont programmés pour elle. Deux jours avant, elle reçoit le premier ; la veille, le deuxième ; une heure avant, le troisième ; trente minutes avant, le dernier. Si la plateforme redémarre, si un travail est rejoué, si l'équipe reprogramme la séance, **elle ne reçoit jamais deux fois le même rappel**. Si elle annule son inscription, plus rien ne part. Si la séance est déplacée, les rappels suivent le nouveau créneau.

**Why this priority**: c'est la demande explicite du commanditaire, et le défaut corrigé de la v1 — des rappels déclenchés par du code dispersé, sans état, dont personne ne pouvait dire s'ils étaient partis.

**Independent Test**: inscrire quarante personnes, faire passer l'heure du premier décalage, constater quarante envois ; rejouer l'événement d'inscription et le travail d'envoi, et constater qu'aucun second courriel n'est écrit ; déplacer la séance et constater que les instants d'envoi ont suivi.

**Acceptance Scenarios**:

1. **Given** une inscription créée à l'état « inscrit », **When** l'événement est consommé, **Then** les rappels de cette personne sont matérialisés d'après la règle applicable — **et le consommateur écoute la création comme le changement d'état, sur le statut porté par l'annonce**, jamais un événement « confirmé » qui n'existe pas.
2. **Given** une personne en liste d'attente, **When** ses rappels seraient matérialisés, **Then** **rien n'est créé** : elle n'a pas de place, la prévenir serait lui en promettre une.
3. **Given** une personne promue depuis la liste d'attente, **When** son inscription change d'état, **Then** ses rappels sont matérialisés à ce moment-là.
4. **Given** une séance programmée ou reprogrammée, **When** l'événement est consommé, **Then** les rappels de **tous** ses inscrits sont matérialisés ou remis à l'heure du nouveau créneau.
5. **Given** un décalage dont l'instant d'envoi est déjà passé, **When** les rappels sont matérialisés, **Then** ce décalage n'est pas créé — on ne réveille personne à trois heures du matin parce qu'un import a pris du retard.
6. **Given** une matérialisation rejouée, **When** elle s'exécute une seconde fois, **Then** **aucune ligne supplémentaire n'est créée** et aucun travail supplémentaire n'est mis en file.
7. **Given** une matérialisation, **When** elle aboutit, **Then** le service **n'émet pas l'annonce de programmation et ne met aucun travail en file** : la fonction du modèle fait déjà les deux.
8. **Given** l'heure d'un rappel atteinte, **When** le travail s'exécute, **Then** un courriel est composé dans la langue du destinataire, envoyé, et le rappel porte son instant d'envoi.
9. **Given** un destinataire dont l'adresse figure sur la liste de suppression, **When** son rappel s'exécute, **Then** **rien n'est envoyé**, le rappel est écarté et son motif le dit.
10. **Given** un destinataire ayant coupé ce canal pour ce type, **When** son rappel s'exécute, **Then** rien n'est envoyé et le motif le dit.
11. **Given** une séance annulée, **When** l'événement est consommé, **Then** les rappels encore à traiter sont annulés et leur motif le dit.
12. **Given** un inscrit qui annule, **When** l'événement est consommé, **Then** ses rappels encore à traiter sont annulés, et ceux déjà partis restent tracés.
13. **Given** un envoi en échec technique, **When** il est repris, **Then** il l'est dans la limite du nombre de tentatives du modèle, et l'échec définitif est visible sans lire un journal de serveur.
14. **Given** le relais d'envoi arrêté, **When** l'heure d'un rappel passe, **Then** **rien ne part**, rien n'est perdu, et le rappel part au premier redémarrage.

---

### User Story 6 — L'administrateur décide ce qui part, et voit ce qui va partir (Priority: P2)

Une administratrice ouvre le paramétrage des rappels d'une édition. Elle voit les quatre décalages cumulés, le canal, le modèle de message. Elle peut ajouter un décalage, en retirer un, couper la règle. Pour une séance particulière — la séance d'ouverture, très attendue —, elle pose une règle propre, qui **remplace** celle de l'édition sans s'y ajouter. À tout moment, elle peut demander « qu'est-ce qui va partir pour cette séance ? » et la réponse lui dit **quelle** règle s'applique et **d'où** elle vient.

**Why this priority**: sans écriture, la règle par défaut n'existe pour aucune édition — rien ne la sème — et **aucun rappel ne part, en silence**. Sans la lecture « d'où vient la règle », la non-cumulation est invérifiable de l'extérieur.

**Independent Test**: créer une règle d'édition, demander la règle applicable d'une de ses séances et constater qu'elle vient de l'édition ; poser une règle de séance à deux décalages et constater que la réponse ne porte plus que ces deux-là, en disant qu'elle vient de la séance.

**Acceptance Scenarios**:

1. **Given** un compte détenant le droit de paramétrer les rappels sur une édition, **When** il écrit une règle, **Then** elle porte **une liste de décalages** et jamais un décalage unique ; la liste par défaut est celle des quatre du modèle.
2. **Given** une écriture de règle sans décalage, ou avec un décalage nul ou négatif, ou avec plus de huit décalages, **When** elle est soumise, **Then** elle est refusée sur le champ des décalages, avec le message que la base formule.
3. **Given** une écriture de règle portant à la fois une édition et une séance, ou aucune des deux, **When** elle est soumise, **Then** elle est refusée : une règle vise l'une **ou** l'autre.
4. **Given** une édition portant déjà une règle, **When** une seconde est écrite pour la même édition, **Then** c'est la première qui est modifiée — il n'y en a qu'une par édition, et une par séance.
5. **Given** une séance, **When** on demande la règle qui lui est applicable, **Then** la réponse porte les décalages, les canaux, le modèle, **l'origine de la règle** — séance ou édition — et **l'identifiant de l'entité dont elle vient**.
6. **Given** une séance sans règle propre et dont l'édition n'en a pas, **When** on demande la règle applicable, **Then** la réponse dit explicitement qu'**aucune règle ne s'applique**, et l'écran peut proposer d'en créer une.
7. **Given** une règle modifiée, **When** elle est enregistrée, **Then** les rappels **déjà partis** ne sont pas touchés, et les rappels encore à traiter des séances concernées sont remis en conformité.
8. **Given** une règle coupée, **When** elle l'est, **Then** les rappels encore à traiter des séances qu'elle gouvernait sont annulés.
9. **Given** un compte détaché sur une édition, **When** il paramètre les rappels d'une **autre** édition, **Then** il reçoit le même refus qu'avec une édition inexistante.
10. **Given** un compte sans le droit de paramétrer les rappels, **When** il tente une écriture, **Then** il est refusé par permission et non par nom de rôle.

---

### User Story 7 — Les textes des courriels se corrigent sans redéploiement (Priority: P3)

L'IFDD relit le courriel de rappel et y trouve une faute. Une éditrice ouvre le modèle, corrige, prévisualise avec des valeurs d'exemple, publie. Le prochain rappel part corrigé. La version anglaise se corrige au même endroit, dans la même page. Si la nouvelle version est pire que l'ancienne, on revient à la précédente d'un geste.

**Why this priority**: c'est le défaut de la v1 nommé par le modèle — des gabarits en dur dans le code, une faute d'orthographe qui exigeait un redéploiement, une version anglaise inexistante, et deux fonctions qui divergeaient sur le même courriel. Il est en P3 parce qu'un rappel part correctement avec un texte de secours, mais il ne se corrige pas sans cette histoire.

**Independent Test**: publier une révision d'un modèle, envoyer un message de ce type et constater le nouveau texte ; revenir à la révision précédente et constater l'ancien ; retirer une variable attendue du gabarit et constater que la publication est refusée.

**Acceptance Scenarios**:

1. **Given** un modèle, **When** on écrit une révision, **Then** elle porte un sujet, un corps HTML et un corps texte, chacun en français et en anglais, et elle **n'est pas servie** tant qu'elle n'est pas publiée.
2. **Given** une révision, **When** elle est publiée, **Then** elle devient la version servie, et la précédente reste lisible et republiable.
3. **Given** un corps HTML saisi, **When** il est enregistré, **Then** il est **assaini à l'écriture** contre la liste blanche des courriels, et ce qui n'y figure pas disparaît sans emporter le texte qu'il contenait.
4. **Given** un gabarit citant une variable que le type de notification ne fournit pas, **When** il est publié, **Then** la publication est refusée en nommant la variable — mieux vaut un refus visible qu'un « Bonjour  , » envoyé à deux mille personnes.
5. **Given** un gabarit, **When** on demande son aperçu avec des valeurs d'exemple, **Then** le rendu est retourné dans les deux langues sans rien envoyer.
6. **Given** un rendu, **When** une variable attendue manque à l'exécution, **Then** **rien n'est envoyé**, le travail échoue de façon visible, et le motif nomme la variable.
7. **Given** un type de notification sans modèle publié, **When** un message de ce type doit partir, **Then** il part avec le texte de secours du module, et la trace d'expédition dit qu'aucun modèle n'a servi — un rappel ne se perd jamais en silence.
8. **Given** une langue absente d'un gabarit, **When** un destinataire la préfère, **Then** le français est servi, conformément à la règle de repli du modèle.
9. **Given** un compte sans le droit de gérer les modèles, **When** il tente une écriture, **Then** il est refusé par permission.
10. **Given** une révision publiée, **When** un message est envoyé, **Then** la trace d'expédition porte le modèle et le **numéro de révision** réellement servis.

---

### User Story 8 — Chacun choisit ce qu'il reçoit, et une adresse morte cesse d'être sollicitée (Priority: P3)

Une personne inscrite reçoit trop de courriels. Dans ses préférences, elle coupe les rappels par courriel et les garde dans la plateforme. Les avis de sécurité de son compte, eux, continuent d'arriver : ils ne se coupent pas. Ailleurs, une adresse rebondit durement ; la plateforme cesse définitivement de lui écrire, quel que soit le module émetteur, et l'IFDD peut voir pourquoi.

**Why this priority**: sans préférences, la plateforme n'a aucun moyen honnête de répondre à « arrêtez de m'écrire » ; sans liste de suppression, le taux de rebond monte et le domaine expéditeur finit en indésirable **pour tout le monde** — y compris pour les confirmations d'inscription.

**Independent Test**: couper un canal pour un type, déclencher une notification de ce type et constater qu'elle n'arrive que sur l'autre canal ; couper un type critique et constater que la coupure est sans effet ; poser une suppression sur une adresse et constater qu'aucun envoi ne part vers elle, quel que soit le module.

**Acceptance Scenarios**:

1. **Given** une personne sans préférence enregistrée, **When** une notification lui est destinée, **Then** les canaux du type s'appliquent — elle n'a rien à configurer pour que la plateforme se comporte correctement.
2. **Given** une personne ayant coupé un canal pour un type, **When** une notification de ce type est destinée à ce canal, **Then** elle n'est pas envoyée.
3. **Given** un type critique, **When** la personne tente de le couper, **Then** la préférence est enregistrée mais **sans effet** : sécurité du compte, annulation de séance et obligations légales passent toujours, et l'écran le dit.
4. **Given** une notification in-app écrite, **When** la personne consulte ses notifications, **Then** elle voit les non lues d'abord, leur nombre, leur lien de rebond, et peut les marquer lues ou les archiver, une par une ou toutes ensemble.
5. **Given** trois notifications de même nature pour la même personne, **When** elles portent la même clé de regroupement, **Then** elles forment **une** ligne portant un compte, tant qu'elle n'est pas lue.
6. **Given** une notification, **When** elle est écrite, **Then** son lien de rebond est un **chemin relatif** : aucun nom d'hôte de préproduction ne fuite dans les données.
7. **Given** un rebond dur ou une plainte remontés par le fournisseur, **When** ils sont enregistrés, **Then** l'adresse entre sur la liste de suppression et **aucun module** ne lui écrit plus.
8. **Given** une suppression temporaire échue, **When** un envoi est tenté, **Then** il part : la levée est automatique, sans intervention.
9. **Given** un envoi quelconque de la plateforme, **When** il est mis en file, **Then** la liste de suppression est consultée **avant**, et la trace d'expédition est écrite quel que soit le résultat.
10. **Given** une remise, un rebond ou une plainte annoncés par le fournisseur, **When** l'annonce arrive, **Then** la trace d'expédition correspondante est mise à jour, et une annonce rejouée ne crée pas une seconde trace.
11. **Given** une personne anonymisée pour motif d'effacement, **When** on relit le journal d'expédition, **Then** les traces subsistent sans la rattacher — c'est un journal, pas un annuaire.

---

### User Story 9 — Le disque ne se remplit pas tout seul (Priority: P3)

Un fichier téléversé puis jamais utilisé n'est rattaché à rien. Au bout d'un mois, la plateforme le signale ; l'IFDD peut le supprimer, et il reste récupérable un temps avant de quitter réellement le disque. Les compteurs d'espace se réalignent périodiquement sur la réalité. Au back-office, l'IFDD voit qui consomme quoi et peut relever le plafond d'une organisation qui en a besoin.

**Why this priority**: c'est le mécanisme qui manquait totalement à la v1 — sans registre des usages, un fichier retiré d'une page restait sur le disque pour toujours. Il est en P3 parce que le disque ne déborde pas le premier jour, et parce que rien d'autre n'en dépend.

**Independent Test**: déposer un fichier sans le rattacher, avancer sa date de création d'un mois, le voir apparaître dans les orphelins ; programmer sa purge, constater qu'il est encore récupérable, puis, la fenêtre échue, constater qu'il a quitté le stockage et que la consommation a baissé d'autant.

**Acceptance Scenarios**:

1. **Given** des objets servables et non rattachés depuis plus d'un mois, **When** la liste des orphelins est demandée, **Then** ils sont rendus du plus lourd au plus léger, avec l'espace qu'ils occupent, variantes comprises.
2. **Given** un objet **encore rattaché**, **When** sa suppression est demandée, **Then** elle est **refusée** en disant combien d'entités l'utilisent : la déduplication fait qu'un même fichier sert plusieurs fiches, et le supprimer effacerait l'image d'une autre.
3. **Given** un objet orphelin, **When** sa suppression est demandée, **Then** il est marqué supprimé, porte sa date de purge, cesse d'être servi, et la consommation de son organisation baisse immédiatement.
4. **Given** un objet supprimé dont la fenêtre de rétention n'est pas échue, **When** le travail de purge passe, **Then** il n'est pas touché.
5. **Given** un objet dont la fenêtre est échue, **When** le travail de purge passe, **Then** l'objet et ses variantes quittent le stockage, et la ligne porte l'instant de la purge.
6. **Given** une purge dont l'objet a déjà disparu du stockage, **When** elle s'exécute, **Then** elle aboutit sans échouer — l'objectif est atteint.
7. **Given** des compteurs ayant dérivé, **When** la réconciliation passe, **Then** ils sont réalignés sur la consommation calculée, et le nombre de lignes corrigées est tracé.
8. **Given** le back-office, **When** l'IFDD demande les quotas, **Then** elle voit par organisation le plafond, la consommation, le nombre de fichiers et la part consommée, triés par proximité du plafond.
9. **Given** une organisation à l'étroit, **When** son plafond est relevé, **Then** l'écriture est gardée par permission, tracée, et prend effet immédiatement.
10. **Given** une organisation absorbée par une fusion, **When** la fusion s'exécute, **Then** ses objets suivent la fiche survivante et son quota disparaît — le module est déjà inscrit au registre de fusion, et **ce jalon ne le réécrit pas**.

---

### Edge Cases

1. **Deux organisations déposent le même fichier.** L'empreinte est unique par dépôt de stockage : une seule ligne existe, et elle appartient à la première. La seconde obtient un rattachement vers cet objet et **ne consomme aucun espace** — c'est l'effet recherché de la déduplication, et il doit être dit. En revanche, la première ne doit pas pouvoir faire disparaître l'image de la seconde : la suppression d'un objet encore rattaché est refusée (écart n° 128).
2. **Une image sans texte alternatif.** Le modèle interdit à une image d'atteindre l'état servable sans texte alternatif, et aucun des trois écrans qui téléversent ne le demande. Sans arbitrage, l'objet resterait « en traitement » pour toujours et l'écran afficherait un emplacement vide sans raison (écart n° 129, arbitrage provisoire à l'hypothèse H3).
3. **Le stockage est injoignable.** Une déclaration de fichier échoue proprement, sans écrire de description ; une confirmation échoue sans marquer l'objet servable ; une purge est reprise plus tard. Rien ne laisse une ligne décrivant un objet qui n'existe pas.
4. **La réception s'interrompt en cours de route.** Ce qui a été écrit sous une clé temporaire est retiré immédiatement ; s'il subsiste — coupure du service au mauvais moment —, il n'est atteignable par personne, aucune lecture ne composant d'adresse sans ligne en base, et le travail de purge le ramasse à la même échéance que les orphelins.
5. **Une inscription est créée directement à l'état « inscrit ».** L'annonce est une **création**, pas un changement d'état, et elle porte le statut. Un consommateur qui n'écouterait que les changements d'état raterait la quasi-totalité des inscriptions (écart n° 126).
6. **La séance est déplacée après que deux rappels sont partis.** Les rappels partis restent tracés ; les rappels à venir sont remis à l'heure du nouveau créneau ; un décalage dont l'instant est désormais passé n'est pas envoyé en rattrapage.
7. **La séance est déplacée d'une heure en avant, à J-40 minutes.** Le décalage de 30 minutes est encore devant : il part. Le décalage d'une heure est derrière : il ne part pas, et son état le dit.
8. **Deux worker traitent le même rappel.** La clé d'unicité du modèle et la clé d'idempotence du travail forment deux barrières indépendantes : le second envoi n'a pas lieu.
9. **Une édition n'a aucune règle de rappel.** Rien n'en sème : sur une base neuve, aucun rappel ne part. La lecture doit dire « aucune règle » et non rendre une liste vide (écart n° 130).
10. **Un type de notification n'a aucun modèle publié.** Aucun n'est semé (écart n° 131). Le message part avec le texte de secours du module, et la trace le dit.
11. **Le corps d'un modèle contient un script.** L'assainissement à l'écriture le retire. Un modèle enregistré avant l'assainissement — il n'y en a aucun aujourd'hui — resterait dangereux : c'est pourquoi le filtre est à l'écriture et pourquoi il n'y a rien à rattraper.
12. **Le journal d'expédition franchit un mois sans partition.** Les partitions sont amorcées sur trois mois seulement ; passé ce délai, tout tombe dans la partition par défaut, et la purge par bascule de partition cesse de fonctionner. Un travail récurrent doit préparer les mois à venir (écart n° 137).
13. **Une organisation atteint son plafond en cours de dépôt.** Le contrôle préalable disait « il reste 4 Mio », un autre dépôt de 3 Mio aboutit entre-temps, et l'écriture est refusée par la base. Le refus final doit porter **le même code stable** que le refus préalable, sinon l'écran affiche une erreur technique là où il sait afficher un message.
14. **Un rôle exclusif est posé deux fois simultanément.** L'index unique partiel du modèle tranche ; le second reçoit le refus explicite, pas une violation d'index brute.
15. **Une personne se désinscrit puis se réinscrit à la même séance.** Ses rappels annulés ne se ressuscitent pas ; les nouveaux se matérialisent sous la même clé d'unicité — la seconde matérialisation ne crée donc rien tant que les lignes existent. Le service doit **réactiver** les lignes annulées plutôt qu'en attendre de nouvelles, sans quoi la personne ne recevrait plus rien.

---

## Requirements *(mandatory)*

### A. Frontières, socle et ce qui n'est pas refait

- **FR-001** : Deux crates sont créés — `backend/crates/modules/media` et `backend/crates/modules/engagement` — chacun exposant domaine, dépôts, service et routes.
- **FR-002** : Aucun des deux crates NE DOIT dépendre de l'autre, ni d'un autre crate de module. Le graphe des dépendances est vérifié mécaniquement.
- **FR-003** : Les routes des deux modules sont montées par `api`, leurs travaux différés déclarés au registre de `worker`. Ni `api` ni `worker` ne sont réinventés.
- **FR-004** : Aucun fichier de `docs/database/` n'est modifié en structure. Toute exception est justifiée par écrit et se borne à une **fonction de lecture** ou à un **amorçage de données**, jamais à une table ni à une colonne.
- **FR-005** : Le système NE DOIT PAS réimplémenter l'assainissement de la présentation détaillée d'un dossier : il est livré et éprouvé en B4, à l'écriture, sur l'unique chemin d'écriture de la colonne.
- **FR-006** : Toute erreur porte un code stable et un message français, et une erreur de validation désigne le champ fautif.
- **FR-007** : Toute écriture positionne l'acteur et l'identifiant de requête en début de transaction.
- **FR-008** : Aucun identifiant hors du périmètre de l'appelant ne se distingue d'un identifiant inexistant par la forme de la réponse.
- **FR-009** : La documentation OpenAPI est **engendrée** depuis les gestionnaires et le catalogue d'erreurs, jamais écrite à la main.

### B. Téléverser (US1)

- **FR-010** : Le téléversement se fait en **un seul geste** — métadonnées et fichier ensemble —, précédé d'une **annonce facultative** qui rend le verdict de l'API sans qu'un octet soit envoyé.
- **FR-011** : L'annonce comme le dépôt portent le nom d'origine, le type, le poids, l'entité porteuse visée, le rôle visé, le texte alternatif s'il s'agit d'une image, et **l'empreinte du contenu si le client sait la calculer**.
- **FR-012** : Le système DOIT rechercher l'empreinte dans le dépôt de stockage visé **avant** toute autre chose et, si elle est connue d'un objet vivant, rendre cet objet sans réclamer d'octet.
- **FR-013** : Le système DOIT refuser une déclaration dont le type ou le poids ne convient pas au rôle visé, d'après la table blanche, en nommant le rôle, l'attendu et le reçu.
- **FR-014** : Le système DOIT vérifier l'espace disponible de l'organisation propriétaire **avant** de délivrer une adresse de dépôt, en s'appuyant sur la fonction du modèle prévue à cet effet.
- **FR-015** : Un refus d'espace, qu'il vienne du contrôle préalable ou de la base, DOIT porter **le même code stable**, avec le plafond, la consommation et l'espace restant.
- **FR-016** : L'annonce n'écrit rien et ne réserve rien : sans envoi, il ne reste aucune trace.
- **FR-017** : L'empreinte est calculée **pendant la réception** ; un flux interrompu ou dont le poids dépasse celui annoncé n'écrit aucune description, et ce qui a été reçu est retiré du stockage.
- **FR-018** : La clé d'objet suit la convention du modèle et ne contient ni protocole, ni nom d'hôte, ni barre oblique initiale, ni espace.
- **FR-019** : Le propriétaire d'un objet est la personne qui le dépose et, s'il y a lieu, l'organisation pour laquelle elle agit ; au moins l'un des deux est renseigné.
- **FR-020** : Le service NE DOIT NI émettre l'annonce de dépôt NI mettre le traitement en file : le déclencheur du modèle fait déjà les deux.
- **FR-021** : Le système DOIT rendre l'objet décrit avec son adresse composée, jamais avec sa clé de stockage nue.
- **FR-022** : Le droit de déposer un fichier est **le droit d'écrire sur l'entité porteuse visée**, vérifié par permission et sur la portée de cette entité. Aucune permission propre au média n'existe dans le modèle (écart n° 127).
- **FR-023** : Chaque combinaison (schéma, table) de la table blanche DOIT être associée, dans le service, à la garde qui lui correspond ; toute combinaison non associée est **refusée**, jamais autorisée par défaut. Un test parcourt la table blanche et vérifie qu'aucune ligne n'est sans garde.

### C. Traiter (US2)

- **FR-024** : Le traitement d'un objet est différé et consomme le travail que le déclencheur du modèle a mis en file.
- **FR-025** : Le traitement DOIT relever les dimensions d'une image et la durée d'un média temporel, et les écrire sur l'objet.
- **FR-026** : Le traitement DOIT faire analyser le contenu et enregistrer le verdict, le moteur et l'instant de l'analyse.
- **FR-027** : Un verdict positif met l'objet en quarantaine ; il n'est **jamais** servi, et tout rattachement le visant est refusé.
- **FR-028** : Le moteur d'analyse est **configurable**, et l'absence de moteur est une valeur déclarée et tracée sur l'objet — jamais un verdict emprunté à un autre cas.
- **FR-029** : Le traitement DOIT fabriquer les déclinaisons d'une image d'après un jeu de variantes configurable, et écrire chacune avec sa taille et son instant de fabrication.
- **FR-030** : Le traitement NE DOIT fabriquer aucune déclinaison pour un document ni pour un média temporel.
- **FR-031** : Le traitement est **rejouable** : une reprise ne fabrique pas deux fois la même déclinaison.
- **FR-032** : L'objet ne devient servable qu'une fois l'analyse concluante et les déclinaisons obtenues ; un échec définitif est enregistré avec son motif et se distingue d'une absence.
- **FR-033** : Tant que l'objet n'est pas servable, aucune lecture publique ne le rend.
- **FR-034** : L'original est servi **dès** que l'objet est servable, indépendamment de l'état des déclinaisons ; la liste des déclinaisons est toujours présente, vide s'il n'y en a pas.

### D. Rattacher (US3)

- **FR-035** : Un rattachement associe un objet à une entité porteuse pour un rôle, et n'existe que pour une combinaison déclarée à la table blanche.
- **FR-036** : Les refus du modèle — combinaison non déclarée, type non accepté, poids dépassé, forme non respectée, objet supprimé ou en quarantaine, rôle exclusif déjà pourvu — sont **traduits** en codes stables et messages français, jamais réimplémentés.
- **FR-037** : Le refus de forme DOIT citer les dimensions reçues, le rapport obtenu, le rapport attendu et la tolérance.
- **FR-038** : Le système DOIT offrir une écriture de **remplacement** pour un rôle exclusif : retrait de l'ancien et pose du nouveau dans la même transaction.
- **FR-039** : Un rôle multiple accepte plusieurs objets, dont l'ordre est déclaré et modifiable.
- **FR-040** : Un rattachement peut porter un texte alternatif propre à son usage, qui prime sur celui de l'objet.
- **FR-041** : Retirer un rattachement NE DOIT PAS détruire l'objet stocké.
- **FR-042** : Le système DOIT rendre l'ensemble des médias d'une entité pour un rôle, dans la forme que le front attend — identifiant, adresse de l'original, dimensions, texte alternatif résolu, légende, crédit et déclinaisons.
- **FR-043** : Le système DOIT offrir l'écriture des **trois déclinaisons d'une édition** — bandeau, couverture, vignette — en un seul geste, chacune indépendante, une valeur nulle retirant la déclinaison sans toucher aux autres. C'est l'obligation que B3 a laissée à ce jalon.
- **FR-044** : Le système DOIT offrir la lecture des règles de la table blanche pour une entité — libellé, multiplicité, types acceptés, poids maximal, forme attendue —, afin qu'un écran annonce la contrainte au lieu de la deviner.
- **FR-045** : Toute écriture de rattachement est bornée au périmètre d'administration de l'appelant quand l'entité porteuse en relève.

### E. Le calendrier des rappels (US4, écart n° 34)

- **FR-046** : Le système DOIT exposer une lecture **agrégée** des rappels d'une séance : une ligne par (décalage, canal).
- **FR-047** : Chaque ligne porte le décalage **en minutes**, le canal, l'instant d'envoi, l'état consolidé, le **nombre** de destinataires et l'instant du dernier envoi.
- **FR-048** : Cette lecture NE DOIT contenir aucun identifiant de personne, aucun nom, aucune adresse électronique et aucun identifiant d'inscription. Un test balaye la charge utile **sérialisée** entière.
- **FR-049** : L'état consolidé d'un groupe est celui de sa ligne **la moins avancée** parmi les lignes encore actives ; si aucune ne l'est, il est celui des lignes écartées ou annulées, accompagné du motif le plus fréquent.
- **FR-050** : Les lignes sont ordonnées du décalage le plus lointain au plus proche du début.
- **FR-051** : Une séance sans règle applicable rend une liste **vide accompagnée du fait qu'aucune règle ne s'applique**, jamais une liste vide muette.
- **FR-052** : L'agrégation est écrite **une seule fois** et servie à ses deux lecteurs — la lecture par séance et la composition de l'espace organisation — sans être recomposée à deux endroits.
- **FR-053** : L'espace d'une organisation DOIT porter ce calendrier pour chacune de ses séances suivies, dans la forme exacte que le front attend, refermant l'écart n° 108 ouvert par B4.
- **FR-054** : L'accès au calendrier d'une séance est ouvert à l'organisation qui l'anime — par adhésion active, jamais par périmètre d'administration, une organisation n'administrant rien — et à qui détient le droit de gérer les inscriptions sur son édition.

### F. Matérialiser et envoyer les rappels (US5)

- **FR-055** : Le système consomme les annonces de **création** et de **changement d'état** des inscriptions, et branche sur le **statut porté par l'annonce** — jamais sur un type d'événement « confirmé », qui n'existe pas.
- **FR-056** : Le système consomme les annonces de **programmation** et de **report** d'une séance et remet ses rappels en conformité avec le nouveau créneau.
- **FR-057** : La matérialisation appelle la fonction du modèle et NE DOIT NI émettre l'annonce de programmation NI mettre les travaux en file : la fonction fait déjà les deux.
- **FR-058** : Aucun rappel n'est matérialisé pour une inscription annulée ou en liste d'attente.
- **FR-059** : Aucun rappel n'est matérialisé pour un décalage dont l'instant d'envoi est déjà passé.
- **FR-060** : Toute consommation d'annonce est gardée du rejeu par le mécanisme du noyau ; une seconde livraison ne produit aucun effet supplémentaire.
- **FR-061** : Une inscription reprise après annulation DOIT **réactiver** ses lignes de rappel encore à venir plutôt qu'en attendre de nouvelles, la clé d'unicité du modèle interdisant leur recréation.
- **FR-062** : L'annulation d'une séance annule les rappels encore à traiter, avec leur motif ; les rappels partis restent tracés.
- **FR-063** : L'annulation d'une inscription annule les rappels encore à traiter de cette personne, avec leur motif.
- **FR-064** : L'envoi d'un rappel compose le message dans la langue préférée du destinataire, avec repli sur le français.
- **FR-065** : Avant tout envoi, le système DOIT consulter la liste de suppression et la préférence de la personne pour ce type et ce canal ; un envoi écarté l'est **avec son motif**, jamais en silence.
- **FR-066** : Tout envoi écrit une trace d'expédition portant le destinataire, la langue, le type, le modèle et sa révision, le sujet rendu, les variables, le rappel d'origine et l'état.
- **FR-067** : Un échec technique est repris dans la limite du nombre de tentatives du modèle ; l'échec définitif est visible sans lire un journal de serveur.
- **FR-068** : Le relais d'envoi arrêté, rien ne part et rien n'est perdu : les rappels partent au redémarrage.
- **FR-069** : Le système NE DOIT PAS envoyer deux fois le même rappel, quel que soit le nombre de rejeux, de redémarrages ou de reprogrammations.

### G. Paramétrer les rappels (US6)

- **FR-070** : L'écriture d'une règle porte une **liste** de décalages, jamais un décalage unique ; la liste par défaut est celle des quatre décalages du modèle.
- **FR-071** : Les refus du modèle sur les décalages — liste vide, valeur nulle ou négative, plus de huit — sont traduits sur le champ des décalages.
- **FR-072** : Une règle vise **une édition ou une séance**, jamais les deux ni aucune ; le refus du modèle est traduit.
- **FR-073** : Il n'existe qu'une règle par édition et une par séance ; une seconde écriture modifie la première.
- **FR-074** : Le système DOIT exposer, pour une séance, la **règle applicable** avec ses décalages, ses canaux, son modèle, **son origine** — séance ou édition — et **l'identifiant de l'entité dont elle vient**.
- **FR-075** : Les deux jeux de décalages NE DOIVENT JAMAIS être fusionnés : la règle de la séance **remplace** celle de l'édition.
- **FR-076** : Une séance sans règle applicable est annoncée comme telle, explicitement.
- **FR-077** : Modifier une règle ne touche pas les rappels déjà partis et remet en conformité les rappels encore à traiter des séances qu'elle gouverne.
- **FR-078** : Couper une règle annule les rappels encore à traiter des séances qu'elle gouvernait.
- **FR-079** : Toute écriture de règle est gardée par la permission de paramétrer les rappels, sur la portée de l'édition visée, et bornée au périmètre d'administration de l'appelant.

### H. Modèles de messages (US7)

- **FR-080** : Une révision de modèle porte sujet, corps HTML et corps texte, chacun multilingue, et n'est servie qu'une fois publiée.
- **FR-081** : Publier une révision la rend servie ; la révision précédente reste lisible et republiable.
- **FR-082** : Le corps HTML d'une révision est **assaini à l'écriture**, contre une liste blanche propre au courriel ; ce qui n'y figure pas disparaît sans emporter le texte contenu.
- **FR-083** : La publication est refusée si le gabarit cite une variable que le type de notification ne s'engage pas à fournir, en nommant la variable.
- **FR-084** : Le système DOIT offrir un aperçu rendu avec des valeurs d'exemple, dans les deux langues, sans rien envoyer.
- **FR-085** : Une variable manquante à l'exécution fait **échouer** l'envoi de façon visible, en la nommant ; aucun message incomplet ne part.
- **FR-086** : Un type sans modèle publié DOIT partir avec le texte de secours du module, et la trace d'expédition DOIT dire qu'aucun modèle n'a servi.
- **FR-087** : Une langue absente d'un gabarit se replie sur le français.
- **FR-088** : Les écritures de modèle sont gardées par la permission de gérer les modèles.
- **FR-089** : La trace d'expédition porte le modèle **et le numéro de révision** réellement servis.

### I. Notifications, préférences et délivrabilité (US8)

- **FR-090** : Le système écrit une notification in-app pour les types de son catalogue, dans la transaction du consommateur, avec titre, corps ou variables, lien de rebond et sujet visé.
- **FR-091** : Le lien de rebond est un **chemin relatif**, jamais une adresse absolue.
- **FR-092** : Les notifications portant la même clé de regroupement pour une même personne forment **une** ligne portant un compte, tant qu'elle n'est pas lue.
- **FR-093** : Une personne DOIT pouvoir lire ses notifications, connaître le nombre de non lues, les marquer lues ou archivées, une par une ou toutes ensemble.
- **FR-094** : Une personne DOIT pouvoir lire et écrire ses préférences par type **et** par canal ; l'absence de préférence retombe sur les canaux du type.
- **FR-095** : Une préférence sur un type **critique** est enregistrée mais sans effet, et la lecture le dit à l'écran.
- **FR-096** : Un type inconnu ou désactivé vaut **refus d'envoi** — on n'invente pas d'envoi.
- **FR-097** : Un rebond dur ou une plainte inscrivent l'adresse sur la liste de suppression ; **aucun module** ne lui écrit plus.
- **FR-098** : Une suppression temporaire échue se lève sans intervention.
- **FR-099** : La liste de suppression est consultée avant **tout** envoi de la plateforme, y compris ceux des modules livrés avant ce jalon, sans qu'aucun d'eux soit modifié.
- **FR-100** : Toute expédition de la plateforme écrit une trace, quel que soit son issue, y compris pour les modules livrés avant ce jalon.
- **FR-101** : Les annonces de remise, de rebond et de plainte du fournisseur mettent la trace à jour ; une annonce rejouée ne crée pas de seconde trace.
- **FR-102** : Le journal d'expédition survit à l'anonymisation d'une personne : il conserve ses traces sans la rattacher.
- **FR-103** : L'IFDD DOIT pouvoir consulter la liste de suppression, y inscrire une adresse et l'en retirer, sous permission.

### J. Quotas, orphelins et purge (US9)

- **FR-104** : Le système DOIT rendre les objets servables non rattachés depuis un délai configurable, du plus lourd au plus léger, avec l'espace occupé, variantes comprises.
- **FR-105** : La suppression d'un objet **encore rattaché** est refusée, en disant combien d'entités l'utilisent.
- **FR-106** : La suppression d'un objet orphelin le marque supprimé avec sa date de purge ; il cesse d'être servi et la consommation baisse immédiatement.
- **FR-107** : Un travail récurrent efface du stockage les objets et leurs variantes dont la fenêtre de rétention est échue, et enregistre l'instant de la purge.
- **FR-108** : Une purge dont l'objet a déjà disparu du stockage **aboutit** : l'objectif est atteint.
- **FR-109** : Un travail récurrent réaligne les compteurs de consommation sur la consommation calculée et trace le nombre de lignes corrigées.
- **FR-110** : Un travail récurrent prépare les partitions mensuelles à venir du journal d'expédition, afin que la purge par bascule de partition reste possible.
- **FR-111** : Le back-office DOIT rendre, par organisation, le plafond, la consommation, le nombre de fichiers et la part consommée, triés par proximité du plafond.
- **FR-112** : Le relèvement d'un plafond est gardé par permission, tracé, et prend effet immédiatement.
- **FR-113** : Le système NE DOIT PAS réécrire le comportement de fusion des organisations : le module est déjà inscrit au registre, et la fusion réaffecte les objets et supprime le quota absorbé sans intervention de ce jalon.

### Key Entities

- **Objet stocké** — un fichier déposé, décrit par son dépôt de stockage et sa clé, son empreinte, son type, son poids, ses dimensions ou sa durée, son propriétaire, sa visibilité, son état de traitement, son verdict d'analyse, son texte alternatif, sa légende, son crédit et sa licence. Il porte son cycle de suppression : marquage, fenêtre de rétention, purge effective.
- **Déclinaison** — une variante d'un objet pour un usage : code de variante, format, dimensions, clé, poids, état de fabrication. Une ligne remplace une colonne d'URL de la v1.
- **Règle de rattachement** — la table blanche : pour une entité porteuse et un rôle, le libellé, la multiplicité, les types acceptés, le poids maximal et la forme attendue avec sa tolérance. Elle déclare ce qui est permis ; elle ne déclare **pas** qui a le droit.
- **Rattachement** — le lien entre un objet et l'entité qu'il illustre, pour un rôle, avec son ordre et son texte alternatif propre.
- **Quota de stockage** — par organisation : plafond en octets et en fichiers, consommation courante. La ligne sans organisation est le quota par défaut.
- **Type de notification** — le catalogue ouvert : code, module, libellé, canaux par défaut, criticité, modèle par défaut, variables promises.
- **Notification** — un avis destiné à une personne : type, titre et corps ou variables, lien de rebond, sujet visé, clé de regroupement et compte, états lu et archivé.
- **Préférence de notification** — l'arbitrage d'une personne pour un type et un canal.
- **Modèle de message** et **révision** — le contenu multilingue d'un courriel, versionné, publié ou non.
- **Trace d'expédition** — ce qui est parti et ce que le fournisseur en a dit : destinataire, langue, type, modèle et révision, sujet rendu, variables, rattachements d'exploitation, état de délivrabilité, tentatives, motif d'échec.
- **Suppression d'adresse** — une adresse qu'on n'écrit plus, son motif, son échéance éventuelle.
- **Règle de rappel** — la politique : portée (édition ou séance), décalages cumulés, canaux, type et modèle.
- **Rappel programmé** — la matérialisation : séance, personne, inscription, canal, décalage, instant d'envoi, état, travail, motif d'écartement.
- **Créneau de rappel agrégé** — ce que lit une organisation : décalage, canal, instant d'envoi, état consolidé, **nombre** de destinataires, instant du dernier envoi. Jamais une liste nominative.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : Un fichier de 2 Mio déposé par une organisation est visible sur sa fiche **en moins de trois secondes**, avant même que ses déclinaisons existent.
- **SC-002** : Le même contenu déposé une seconde fois n'écrit **aucun** second objet et n'occupe **aucun** octet de plus ; annoncé à l'avance avec son empreinte, il ne transite pas du tout.
- **SC-003** : Sur cent dépôts du même fichier, l'espace consommé est celui **d'un** fichier.
- **SC-004** : Une organisation ayant atteint son plafond reçoit un refus **avant** l'envoi du fichier, portant les trois chiffres — plafond, consommé, restant.
- **SC-005** : Les déclinaisons d'une photographie de 4 Mio sont disponibles **en moins d'une minute** après le dépôt, sans nouveau geste de l'utilisateur.
- **SC-006** : Un fichier reconnu comme dangereux n'est rendu par **aucune** lecture de la plateforme, et aucun rattachement le visant n'aboutit.
- **SC-007** : Les trois déclinaisons d'une édition s'enregistrent en **un** geste et se relisent résolues sur la fiche.
- **SC-008** : Une forme non conforme est refusée en citant le rapport reçu et le rapport attendu — jamais par un message technique.
- **SC-009** : Retirer un visuel d'une édition laisse l'objet stocké intact, vérifié par relecture après le retrait.
- **SC-010** : Une séance à quarante inscrits et quatre décalages rend **quatre** lignes de calendrier, chacune portant **quarante** destinataires.
- **SC-011** : La charge utile sérialisée de ce calendrier ne contient **aucune** occurrence d'un identifiant de personne, d'un nom ou d'une adresse électronique.
- **SC-012** : Une règle à deux canaux et quatre décalages rend **huit** lignes, et non quatre.
- **SC-013** : Une séance sans règle applicable est distinguable d'une séance dont tous les rappels sont partis, par la seule lecture de la réponse.
- **SC-014** : Quarante inscrits et quatre décalages produisent **cent soixante** rappels et, l'heure venue, **quarante** envois par décalage — jamais quarante et un.
- **SC-015** : L'annonce d'inscription rejouée dix fois produit **zéro** rappel supplémentaire et **zéro** travail supplémentaire.
- **SC-016** : Une séance déplacée de trois heures voit les instants d'envoi de ses rappels non partis décalés d'autant, et ses rappels partis inchangés.
- **SC-017** : Le relais d'envoi arrêté, l'heure du rappel passée puis le relais relancé : **un** courriel arrive, pas zéro, pas deux.
- **SC-018** : Une adresse inscrite sur la liste de suppression ne reçoit plus **aucun** courriel de la plateforme, y compris ceux des modules livrés avant ce jalon.
- **SC-019** : Couper un canal pour un type non critique supprime l'envoi sur ce canal et le laisse sur l'autre ; couper un type critique est sans effet, et l'écran le dit.
- **SC-020** : Une révision de modèle publiée change le texte du **prochain** message envoyé, sans redéploiement ; le retour à la révision précédente rétablit l'ancien.
- **SC-021** : Un gabarit citant une variable non promise est refusé à la publication, en la nommant.
- **SC-022** : Un corps de modèle contenant un script est enregistré **sans** le script et **avec** tout son texte.
- **SC-023** : Un type sans modèle publié envoie tout de même son message, et la trace dit qu'aucun modèle n'a servi.
- **SC-024** : Un objet non rattaché depuis plus d'un mois apparaît dans les orphelins ; un objet rattaché n'y apparaît jamais.
- **SC-025** : La suppression d'un objet rattaché à deux fiches est **refusée**, en disant deux.
- **SC-026** : Un objet purgé a quitté le stockage, et la consommation de son organisation a baissé de son poids **variantes comprises**.
- **SC-027** : Après réconciliation, la consommation enregistrée d'une organisation est **égale** à sa consommation calculée.
- **SC-028** : Aucune arête ne relie les deux nouveaux crates entre eux ni à un autre crate de module, vérifié mécaniquement.
- **SC-029** : Le rapport de frontières inter-schémas reste vide, et la chaîne de vérification complète passe au vert.

---

## Assumptions

- **H1 — Le dépôt passe par l'API, en flux** *(amendé le 21/08 par le plan, § R6 — la version initiale décrivait un protocole en trois temps avec adresse de dépôt signée, et annonçait ce repli)*. Exposer le stockage en écriture au navigateur exigerait une déclaration d'origines croisées et une joignabilité publique, pour un gain qui n'existe que sur les très gros fichiers. Le fond vidéo de 200 Mio traverse donc l'API, en flux, sans être chargé en mémoire : c'est une vidéo par édition.
- **H2 — L'empreinte est calculée par le serveur pendant la réception**, et par le client seulement s'il le peut. Le contrat du front ne la porte pas : `DraftUpload` connaît le nom, le type et le poids, jamais l'empreinte. La déduplication économise donc **toujours** le disque, et la bande passante uniquement quand le client sait annoncer l'empreinte à l'avance.
- **H3 — Le texte alternatif d'une image est exigé au dépôt** (arbitrage provisoire, question ouverte au commanditaire). Le modèle interdit à une image d'être servie sans lui ; ne pas l'exiger produirait des objets bloqués en traitement, ce qui est le pire des deux mondes. Les trois écrans qui téléversent ne le demandent pas encore : l'ajout du champ est inscrit aux obligations de B7. En attendant, un écran qui n'envoie rien reçoit un refus clair sur le champ, pas un silence.
- **H4 — Le moteur d'analyse antivirus est configurable, et son absence est déclarée.** En développement, aucun moteur n'est branché et l'objet porte explicitement le nom du moteur qui a répondu. En production, un moteur réel est exigé par la configuration.
- **H5 — Le jeu de variantes vit dans la configuration**, comme le modèle le dit lui-même. Trois tailles en deux formats modernes sont retenues par défaut pour les images ; aucune pour les documents et les médias temporels.
- **H6 — Les courriels des modules livrés ne sont pas réécrits.** Ce jalon ne refait pas les six courriels de B1 et B2. Il les fait passer par la garde de suppression et par le journal d'expédition **sans les modifier**, en enveloppant le contrat d'envoi du noyau : les modules livrés reçoivent le même contrat et ne changent pas d'une ligne. La bascule de leur composition vers les modèles administrables est une dette consignée, non un chantier de ce jalon.
- **H7 — Le rendu d'un gabarit est une substitution de variables nommées**, sans logique conditionnelle ni boucle. Les gabarits de la plateforme sont des courriels transactionnels courts ; un langage de gabarit complet serait une dépendance d'ampleur pour un besoin qui n'existe pas.
- **H8 — Les notifications in-app n'ont pas encore d'écran.** Elles sont écrites et lisibles parce que le catalogue, les préférences et la criticité forment le seul mécanisme honnête de dire « n'écrivez plus », et parce que le prompt les demande. L'écran viendra avec son prompt.
- **H9 — Les commentaires, les réactions, la messagerie directe, la mise en relation, les blocages et les infolettres sont hors périmètre.** Le modèle les porte ; aucun écran ne les consomme, la messagerie est fermée par drapeau, et les infolettres sont déclarées hors phase 1 par le modèle lui-même. Le canal de notification par poussée est hors périmètre pour la même raison : aucun client ne s'y abonne.
- **H10 — Le rappel d'échéance d'un appel à propositions est hors de ce jalon.** L'obligation relevée en B3 suppose des destinataires — quelles organisations, à quel titre — que rien ne définit aujourd'hui. Le mécanisme livré ici (règles, modèles, journal, suppression) le rendra mécanique le jour où le périmètre des destinataires sera arbitré.

---

## Écarts relevés en écrivant cette spécification

Numérotation continuant celle de B5, qui s'arrête à n° 125.

| N° | Constat | Où | Pourquoi c'est grave | Traitement |
|---|---|---|---|---|
| **126** | **`programme.registration.confirmed` n'existe pas.** Le commentaire de `schedule_session_reminders()` annonce que la fonction est appelée sur cet événement ; le déclencheur émet `programme.registration.created` puis `programme.registration.<statut>`, et l'énumération des statuts ne contient ni `confirmed` ni `declined` | `110` § 6, `075` § 4 | **Gravité haute.** Un consommateur écrit d'après le commentaire ne serait jamais réveillé : aucun rappel ne partirait, et rien ne le signalerait. C'est le défaut le plus silencieux du module | Le consommateur écoute la **création** et le **changement d'état**, et branche sur le **statut porté par la charge utile** (FR-055). Un test vérifie qu'une inscription créée directement à l'état « inscrit » matérialise bien ses rappels |
| **127** | **Aucune permission `media.*` n'existe** dans `identity.permissions` : le modèle en déclare pour dix modules, jamais pour le média | `030` § 5, `050` | Rien ne dit qui peut téléverser ni rattacher. Une garde inventée au fil des routes divergerait d'une route à l'autre | Le droit de rattacher est **le droit d'écrire sur l'entité porteuse** (FR-022). Chaque ligne de la table blanche est associée à sa garde dans le service, et un test parcourt la table pour qu'aucune ligne ne reste sans garde (FR-023) |
| **128** | **La déduplication traverse les propriétaires.** L'unicité d'empreinte porte sur le dépôt de stockage, pas sur le propriétaire : le même fichier déposé par deux organisations donne **une** ligne, appartenant à la première | `050` § 2 | La seconde ne consomme aucun espace — effet recherché — mais une suppression par la première **ferait disparaître l'image de la seconde**, sans que rien ne l'annonce | La suppression d'un objet **encore rattaché** est refusée en disant combien d'entités l'utilisent (FR-105). La purge ne vise que les orphelins, ce que le modèle prévoit déjà |
| **129** | **Le texte alternatif manque au contrat du front.** Le modèle interdit à une image d'être servie sans lui ; ni `DraftUpload`, ni le formulaire d'édition, ni la vitrine ne le demandent | `050` § 2, `types/proposal-form.ts`, `types/admin-events.ts`, `types/admin-showcase.ts` | Sans arbitrage, l'objet reste « en traitement » **pour toujours** et l'écran affiche un emplacement vide sans raison — un défaut qui ne produit aucune erreur | Exigé au dépôt (H3), refusé sur le champ. **Question ouverte au commanditaire**, et ajout du champ inscrit aux obligations de B7 |
| **130** | **Aucune règle de rappel n'est semée.** Sur une base neuve, aucune édition n'a de règle, et la fonction de matérialisation rend zéro | `110` § 6, `900` | Aucun rappel ne part, et le chiffre rendu — zéro — ne distingue pas « aucune règle » de « rien à programmer » | La lecture de la règle applicable dit explicitement qu'aucune ne s'applique (FR-076), et l'écriture de règle est livrée par US6 |
| **131** | **Aucun modèle de message n'est semé**, alors que le catalogue des types réserve une colonne pour le modèle par défaut | `110` § 4 et § 11 | Un rendu sans modèle échouerait, et le rappel se perdrait en silence | Repli sur le texte de secours du module, **tracé** dans le journal d'expédition (FR-086) |
| **132** | **Le contrôle d'espace ne vérifie pas le nombre de fichiers** quand l'organisation n'a pas encore de ligne de quota : le repli sur la ligne par défaut ne compare que les octets | `050` § 5 | Une organisation neuve peut dépasser le plafond de fichiers tant que sa ligne n'existe pas — la ligne étant créée à la première écriture, la fenêtre est étroite mais réelle | Consigné. Le service ne réimplémente pas le contrôle ; le refus de la base reste la barrière finale |
| **133** | **Les courriels de B1 et B2 ne passent ni par le journal d'expédition ni par la liste de suppression** : ils appellent directement le contrat d'envoi du noyau | `crates/modules/identity/src/jobs/emails.rs`, `crates/modules/org/src/jobs/emails.rs` | Une adresse en rebond dur continue de recevoir des invitations, et la réputation du domaine expéditeur en pâtit **pour tous les envois**, confirmations d'inscription comprises | Le contrat d'envoi du noyau est **enveloppé** : la garde et la trace s'appliquent sans qu'aucun module livré ne change d'une ligne (FR-099, FR-100, H6) |
| **134** | **Aucun rôle média n'est déclaré pour une séance.** La table blanche couvre les organisations, les éditions, les dossiers, les personnes, les articles et la vitrine — jamais `programme.sessions` | `050` § 8, `115` § 5 | Le compte rendu d'une activité ne peut porter aucune photographie. Point déjà relevé le 16/08 et écarté faute d'écran | Reste écarté : aucun écran ne le consomme dans ce jalon. À semer le jour où le bilan d'activité est développé |
| **135** | **Rien en base ne dit quelles déclinaisons ont été demandées.** Le jeu de variantes vit dans la configuration du worker ; si elle change, aucune lecture ne dit lesquelles manquent | `050` § 3 | Une image traitée avant l'ajout d'un format reste sans ce format, et rien ne permet de retrouver les objets à retraiter | Consigné. Le repli sur l'original couvre le cas à l'affichage ; un retraitement de masse resterait manuel |
| **136** | **Le refus d'espace de la base sort en `disk_full`**, un état d'erreur système et non un refus métier | `050` § 5 | Traduit naïvement, il sortirait en erreur technique là où l'écran sait afficher un message avec les trois chiffres | Traduit sur le **même code stable** que le refus préalable (FR-015) |
| **137** | **Les partitions mensuelles du journal d'expédition ne sont amorcées que sur trois mois.** Le commentaire annonce un worker de maintenance qui n'existe pas | `110` § 5 | Passé trois mois, tout tombe dans la partition par défaut : la purge par bascule de partition, seule raison du partitionnement, cesse de fonctionner | Travail récurrent de préparation des partitions à venir (FR-110) |

---

## Une question au commanditaire

**Faut-il obliger à décrire une image au moment où on la dépose ?**

Sur cette plateforme, une image publiée doit être accompagnée d'une courte phrase qui dit ce qu'elle montre — « les participants de la séance d'ouverture, salle du pavillon » —, lue à voix haute par les logiciels des personnes malvoyantes. La règle est déjà inscrite dans la plateforme : une image sans cette phrase ne s'affiche jamais.

Or les trois écrans qui déposent des fichiers ne la demandent pas encore.

| Option | Ce que ça change |
|---|---|
| **A. Oui, obliger** *(recommandée)* | Une ligne de plus à remplir au moment du dépôt d'une image. Trois écrans à compléter au raccordement. En échange, une image déposée s'affiche **toujours**, et la plateforme reste accessible aux personnes malvoyantes, ce qui est une obligation pour un organisme de la Francophonie |
| **B. Non, laisser vide** | Rien à remplir, rien à changer aux écrans. Mais l'image déposée **ne s'affichera jamais** : elle restera indéfiniment « en cours de traitement », et personne ne comprendra pourquoi l'emplacement reste vide |
| **C. Demander plus tard** | Le dépôt reste sans friction ; un écran de relance liste les images en attente de description. Un écran de plus à concevoir et à faire vivre, et des images invisibles tant que personne n'y passe |

**Recommandation : A.** Le symptôme de B est le **silence** — le pire des trois —, et C reporte le même travail en y ajoutant un écran. En attendant la réponse, l'API tient l'option A et refuse clairement, en nommant le champ.
