# Research — Guide Négo, signalements et notifications (étape 3b)

Chaque décision : ce qui est retenu, pourquoi, ce qui est écarté.

## R1 — Le signalement est une table à lui, jamais une écriture sur la session

**Décision** : `negotiation.session_reports` porte les signalements de changement **et** les réunions
non annoncées (motif `unannounced`, sans session). Aucune colonne de `meetings` ni de
`meeting_changes` ne bouge à cause d'un signalement (FR-015, principe XII) ; un test compare la ligne
officielle avant et après validation (SC-003).

**Écarté** : un état « signalé » sur `meetings` — c'est exactement la réécriture qu'ADR-010 interdit.

## R2 — La réunion non annoncée est un objet à part

**Décision** : `negotiation.network_meetings`, né de la validation d'un signalement `unannounced`
(quoi, où, début, jour, thématique), d'origine « réseau ». Il n'est **pas** une ligne de
`negotiation.meetings` : l'orchestrateur l'a demandé — jamais une ligne de la table des sessions qu'on
ferait passer pour officielle. la réunion non annoncée s'ajoute à « Mon agenda » (FR-018) par une table sœur,
`network_agenda_entries` — `agenda_entries` de 3a ne change pas.

**Pourquoi pas `meetings` avec une origine « réseau »** : chaque requête de 3a filtre les lignes
importées par `source_key` ; une ligne d'une autre origine dans la même table devrait être exclue
partout, et un oubli la ferait passer pour officielle. Deux tables, deux sens.

**Service** : la réponse de `GET /negotiation/sessions` gagne `network_meetings`, servi **même quand
l'affichage officiel est coupé** (FR-022, maquette 07 1c).

## R3 — Valider, annuler, sans jamais laisser partir une notification annulée

**Décision** : **rien n'est public avant `published_at`**. « Valider » écrit `status = validated`,
`decided_by`, `decided_at`, `source_snapshot` — et **rien d'autre ne le voit** : ni l'encart, ni la
réunion non annoncée, ni « Validé » chez l'autrice, ni un courriel. Il pose le travail
`negotiation.report.publish`, `run_at = now() + 30 s` calculé **par la base** (horloge unique), clé
`publish:<report>:<decided_at>`.

Le travail, dans **une** transaction : `SELECT … FOR UPDATE` sur le signalement ; n'agit que si
`status = 'validated' AND decided_at = <clé> AND published_at IS NULL AND withdrawn_at IS NULL` ; alors
pose `published_at = now()`, crée la réunion non annoncée si c'est le motif, émet les événements et pose
les courriels. **Rejoué** (la file est « au moins une fois »), il trouve `published_at` posé et ne fait
rien.

« Annuler » : `UPDATE … SET status = 'submitted', decided_by = NULL, decided_at = NULL, source_snapshot
= NULL WHERE id = $1 AND status = 'validated' AND published_at IS NULL` — **aucune borne de temps côté
serveur** : tant que rien n'est publié, l'annulation gagne ; publié, `409
NEGOTIATION_REPORT_UNDO_EXPIRED`. Le verrou de ligne ordonne l'annulation et le travail : l'un ou
l'autre, jamais les deux. L'écran offre « Annuler » six secondes ; le serveur en laisse trente, ce qui
couvre un réseau de COP lent.

**Conséquence visible** : le message de validation dit « Validé. Affiché dans une minute au plus. » —
et non « Affiché à toutes et tous », qui serait faux pendant trente secondes (écart à inscrire).

**Pourquoi** : SC-004 est une garantie, pas une course. Le test de concurrence (annulation pendant que
le travail tient la ligne) est dans T017.

**Écarté** : l'attente côté téléphone — l'application fermée pendant l'attente perdrait la décision ;
une borne de temps côté serveur — deux horloges et un réseau lent la rendent fausse.

## R4 — Pas de validation hors connexion

La validation n'entre pas dans la file d'écritures (FR-013) : une décision prise sur une vue périmée
de la source contredirait ADR-010 (« en validant, l'administrateur voit ce que dit la source à cet
instant »). Sans réseau, le geste est refusé et le dit.

## R5 — La référence client rend l'envoi rejoué inoffensif

**Décision** : le téléphone pose un `client_ref` (UUID) à la création ; `UNIQUE (author_id,
client_ref)`. `POST /negotiation/reports` rejoué rend **`200` avec le signalement existant**, jamais
une seconde ligne ni une erreur. L'intention de la file porte la clé `signalement:<client_ref>`.

**Doublon de fond** (FR-007) : index unique partiel `(author_id, meeting_id) WHERE status =
'submitted'` → `409 NEGOTIATION_REPORT_DUPLICATE` quand le `client_ref` diffère. Une annulation qui
heurterait cet index (l'autrice a renvoyé entre-temps) répond `409 NEGOTIATION_REPORT_UNDO_EXPIRED`. Le code est au
catalogue prévu par `03-api.md` (« signalement en double »).

## R6 — La permission de valider

**Décision** : `negotiation.report.validate`, nouvelle permission, attribuée au rôle `admin` sur la
portée globale ; garde `Requires<ReportValidate>` sur la portée globale (tranché le 21/09 : jamais
`RequiresAnyScope`). `GET /negotiation/me/access` gagne `can_validate_reports` — l'application sait
ainsi afficher l'entrée « Signalements » sans deviner.

**Écarté** : `space.manage` — il gouverne les codes et l'admission ; un modérateur de signalements
pendant une COP n'a pas à tenir les codes d'invitation.

## R7 — Rattrapage et fin : calculés par l'import, pas à l'affichage

**Décision** : dans la transaction de lecture de 3a (`jobs/import.rs::ecrire`), une fonction pure de
`import/rattrapage.rs` rapproche les signalements validés et affichés de la session lue :
`cancelled` ↔ statut annulé ; `time` ↔ même heure de début (à la minute) ; `venue` ↔ même salle
(comparée par `denominations::normaliser`, la normalisation de 3a) ; `other` jamais. Le rapprochement
lit l'état **en base après écriture** de la lecture, absences comprises (une session passée annulée par
disparition rattrape un signalement « annulée »). Rattrapé → `withdrawn_at`, `withdrawal = 'caught_up'`.
La fin (`ended`) se calcule à l'affichage : `end_at` passé (sinon fin du jour dans le fuseau), sans
écrire. Retrait par l'administration → `withdrawal = 'admin'`.

## R8 — Notifications : `negotiation` décide, `engagement` écrit (retouche du go, 25/09)

**Décision** : **`negotiation` calcule qui prévenir** — par ses fonctions `negotiation.change_recipients()`
et `negotiation.network_recipients()`, dans sa transaction — et **compose l'avis** (titre et corps
FR/EN, lien, sujet, clé de regroupement). Il émet par `platform.emit_event()` un événement qui porte la
liste des destinataires et ce contenu. Le consommateur d'`engagement` ne connaît aucune règle de
Guide Négo : une **branche générique** écrit un avis `in_app` par destinataire pour tout événement qui
porte une charge `notification` (forme ci-dessous), après `canal_autorise`. Il n'appelle rien de
`negotiation`, ne lit aucune de ses tables. Les courriels partent de `negotiation`, par sa file de
travaux (R9).

Charge commune (dans `contracts`) :

```text
notification: { type_code, recipients: [uuid], title: i18n, body: i18n, link_path,
                subject: {schema, table, id}, group_key | null, replace: bool, variables }
```

Types semés dans `engagement.notification_types` (la clé du type est l'`event_type`) :
`negotiation.meeting.changed`, `negotiation.report.published`, `negotiation.network_meeting.published`,
`negotiation.report.decided` — `module_code = 'negotiation'`, `{in_app}`, `normal`.

Regroupement : `group_key = '<type>:<id>:<jour>'` ; `replace: true` remplace titre, corps et variables
de la ligne non lue par l'état final (option ajoutée à `notifications::ecrire`) ; `report.decided`
sans clé.

**La cloche filtre par origine, génériquement** : `GET /notifications` gagne `module`, comparé à
`notification_types.module_code`, une donnée qui existe déjà. Rien de Guide Négo n'est écrit dans
`engagement`.

**Écarté** : faire lire à `engagement` les tables ou les fonctions de `negotiation` — une dépendance
d'un module vers un autre (principes II à IV).

## R9 — Le courriel : un par session et par fenêtre, relu au moment de partir

**Décision** : `negotiation` compose ses courriels (patron de 0b, `mail.rs`), mais **le travail relit la
base** au moment d'envoyer, contrairement à 0b : c'est l'état final qu'il doit dire (FR-029). À chaque
changement, `negotiation` pose **un travail par destinataire** (`negotiation.session_change_email`, clé
`email:<cible>:<fenêtre>:<personne>`, fenêtre = tranche fixe de 10 min, `run_at` = fin de la tranche) :
les changements d'une même tranche tombent dans un seul courriel, et un échec ne renvoie pas aux autres.
Les destinataires viennent des mêmes fonctions de `negotiation`. Le travail relit l'état courant, **ne compte que les signalements publiés (`published_at`)**, vérifie
l'accord (R10) et envoie, gardé par le `GardedMailer`. Heures avec le fuseau de la COP. Réunion non
annoncée : même travail, clé sur la réunion.

**Écarté** : faire envoyer le courriel par `engagement` — il ne compose aujourd'hui que les rappels,
et `negotiation` compose déjà ses courriels (0b).

## R10 — L'accord « Notifications » (écart 40, tranché le 25/09)

**Décision** : `identity.consents`, `purpose = 'guide_nego_notifications'`, version = celle de la
politique de confidentialité servie par `kernel::legal` (la clé qu'emploie 0c pour « À propos ») ; **aucune ligne = allumé** (allumé par défaut, décidé) ; chaque
bascule écrit une ligne. `negotiation` l'écrit en SQL, comme `programme` le fait déjà
(`programme/src/repo/consents.rs`). Routes `GET`/`PUT /negotiation/me/notifications`. L'interrupteur
vit sur « À propos », avec la phrase de la maquette ; l'écart 40 se met à jour dans `05-design.md`.

## R11 — Le réglage par thématique : une colonne sur le suivi

**Décision** : `theme_subscriptions.notify_changes boolean NOT NULL DEFAULT false`. Route
`PUT /negotiation/me/themes/notifications` (`{codes}` : les thématiques allumées, parmi celles
suivies) ; `GET /negotiation/me/themes` rend le drapeau, **et son empreinte l'inclut** (sinon un
autre appareil reçoit `304` et garde l'ancien réglage). Quitter une thématique l'éteint avec elle.

**Écarté** : `engagement.notification_preferences` — elle n'a pas de dimension thématique ; l'étendre
toucherait un modèle partagé pour un seul usage.

## R12 — Le centre de notifications réutilise les routes d'`engagement`

`GET /notifications`, `POST /notifications/read` existent ; **`GET /notifications` gagne un filtre
`module`** (par `notification_types.module_code`, R8) : la cloche de Guide Négo ne compte pas les avis
du site. Côté application : `composables/api/notifications.ts`
(nouveau, pour ne pas grossir `useApi.ts`), `useGnNotifications` (garde `notifications`, 50 dernières,
heure de lecture), marquer lu par la file, **une intention par notification** (`lu:<id>`) — la file garde une intention
par clé, un lot écraserait le précédent. La cloche vit dans l'emplacement `action` de
`GnEntete`, déjà là.

## R13 — L'entrée de validation dans l'application

« Signalements » s'ouvre depuis « Ressources », visible si `can_validate_reports`. Écran
`pages/guide-nego/validation/signalements.vue` sur le patron de 11-validation 1a et 1f ;
`GnMessageEphemere` porte « Annuler » (six secondes). Lecture en ligne seulement ; sans réseau, l'écran
le dit.
