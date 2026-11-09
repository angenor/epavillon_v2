# A16 : Régie du jour

> Extrait de la [progression](../../PROGRESSION.md). Maquette validée : [`docs/maquettes/direction-condensee/regie.html`](../../maquettes/direction-condensee/regie.html).

**État** : ✅ construit le 01/11 (migration datée du 02/11), en production le 02/11 · réunion créée au démarrage, lien d'animateur en grand 02/11

---

## Ce qui a été livré

`/admin/regie`, entrée « Régie du jour » de la section Exploitation, direction condensée. L'équipe y pilote une journée de CdP : présence des organisateurs, annulation, direct démarré, terminé, repris, présentations des organisations à projeter, messages d'incident. **Chaque geste se défait** : bandeau « Défaire » juste après, journal ensuite, même plus tard et par un collègue.

- **Modèle** (`075` § 1 et § 9, migration `2026-11-02-regie`) : `programme.session_presence`, `programme.control_room_actions` (journal, `data` de quoi défaire, `undone_at`), colonnes d'annulation et bornes du direct sur `sessions` tenues par `tg_sessions_status_times`, taxonomie `session_cancellation_reason`, type d'avis `programme.session.restored`, signal `event:<id>:regie`.
- **API** (`programme`, `routes/regie.rs`) : `GET /admin/regie?event_id&day`, `POST /admin/regie/sessions/{id}/presence|cancel|live/start|live/end`, `POST /admin/regie/actions/{id}/undo`. Gardé par le périmètre puis `programme.session.schedule` sur l'édition. Codes `REGIE_LIVE_ALREADY_RUNNING`, `REGIE_SESSION_STATE_CHANGED`, `REGIE_RESUME_EXPIRED`, `REGIE_ACTION_ALREADY_UNDONE`, `REGIE_CANCELLATION_REASON_UNKNOWN`. `live` : `GET /admin/sessions/{id}/meeting/host` (lien d'animateur redemandé à Zoom, `no-store`).
- **Courriels différés** : l'annulation émet `programme.regie.cancellation_notice` ; `engagement.regie` pose `engagement.regie_notice` à `notify_at` (cinq minutes) ; le travail vérifie sous verrou partagé que le geste tient encore, puis pose un courriel par inscrit (clé `regie:<geste>:<personne>`). Rétablir après l'envoi émet `restoration_notice` : « finalement maintenue » à ces personnes seulement.
- **Zoom** : une séance annulée ne voit sa réunion annulée chez Zoom que cinq minutes plus tard (`live.cancel_session_meeting`), et seulement si elle l'est toujours : défaire garde la réunion et le lien.
- **Site** : `pages/admin/regie.vue`, quatorze composants `components/admin/regie/`, logique pure `utils/regie.ts` (12 tests), contrat `types/admin-regie.ts`, mocks hors ligne `mocks/admin-regie.ts`.

## Décisions prises en construisant

1. **Journal dans `programme`** : chaque geste écrit une séance. Les messages d'incident restent à `live` ; le journal les lit dans `live.incidents` (publié, retiré) sans les recopier, et « Défaire » les retire par la route existante.
2. **Présence en table, pas en ENUM** : trois états sans transition imposée. « Absent » ouvre l'annulation ; « pas venus » et « ont prévenu » notent l'absence.
3. **Annuler une séance, pas le dossier** : le dossier reste retenu. Une annulation venue du dossier (A8) n'a pas de geste au journal et ne se rétablit pas depuis la régie.
4. **Règle n° 4** : refus par l'API sous verrou consultatif (deux démarrages simultanés passent l'un après l'autre), titre du direct en cours dit seulement s'il est de la même édition. Le chemin Zoom (`programme.direct`) n'est pas bloqué : il suit ce que Zoom constate.
5. **Démarrer** seulement depuis `scheduled` ; **reprendre** (défaire la fin) seulement dans les quinze minutes après `live_ended_at`. Une annulation d'un direct le rétablit en `scheduled`.
6. **Présentations** : lues dans `programme.proposal_documents` (où l'espace organisation les dépose), tous les dossiers d'une séance commune ; le rôle `document` de `media.attachable_roles` n'est pas utilisé par le code.
7. **Message d'incident publié à part par le site** après l'annulation (portée séance, nature `cancellation`), retiré au rétablissement si la case est cochée : il garde son auteur et sa ligne au journal.
8. **Nouveau message** : portée séance ou toute la programmation ; la « page du direct » de la maquette n'a pas d'équivalent dans le modèle. Le texte anglais est prérempli par le modèle et repris du français s'il reste vide (l'API exige les deux).

## Retours après l'essai en production (02/11)

1. **Démarrer crée la réunion qui manque.** En production, aucune réunion n'existait (`live.meetings` vide) : toutes les activités demandent une inscription et la réunion ne naissait qu'à la première ; personne ne s'était inscrit. Le consommateur `live.sessions` (`programme.session.live`, ou toute séance relue en `live`) appelle `inscrits::assurer_reunion_du_direct` : réunion de l'activité avec ses inscrits déjà posés, ou réunion de la journée (`reconstruire_journee`), quel que soit `registration_required`. Pas de migration.
2. **Lien d'animateur en grand.** Encart du direct dans `NowPanel` : « Ouvrir Zoom (animateur) » en grand (nouvel onglet ouvert au clic, avant l'appel, contre les bloqueurs) ; « Création de la réunion Zoom en cours » avec témoin tant qu'elle n'est pas prête ; en échec, « Relancer la création » (`live.meeting.manage`). Démarrer une activité dont la réunion est déjà prête ouvre Zoom du même clic. L'écran relit au signal du poste et, faute de lui (sujet réservé à `live.meeting.manage`), toutes les 5 s tant que le lien manque ; un message annonce son arrivée.
3. **Direct terminé** : `REGISTRATION_SESSION_ENDED` (409) refuse l'inscription à une séance `completed`, ou passée hors direct (un direct qui déborde reste ouvert). Garder une activité terminée d'une journée ne crée plus d'inscription (`track_registrations::garder`) ; déjà gardée, elle le reste.

## Intervenants en ligne (08/11)

Rangée « Intervenants en ligne » sous chaque activité diffusée (`SpeakersRow`), trois fenêtres portées par `SpeakersGestures` : inscrire un intervenant (`SpeakersDialog` : déjà inscrits, « Afficher sur la page » pour ceux venus du lien, formulaire `speaker/OnlineSpeakerFields`, puis le lien Zoom à copier), renvoyer les liens Zoom (`ResendLinksDialog`), envoyer le lien d'inscription au porteur (`OwnerLinkDialog`, désactivé sans porteur). La carte se relit à la fermeture. Dates d'envoi en « le 12 novembre à 11:30 », au fuseau de l'édition annoncé dans l'en-tête du jour. Le lien Zoom à l'écran se redemande toutes les 3 s pendant 60 s (`utils/speaker-links.ts`, 409 tant que Zoom n'a pas répondu) ; passé ce délai, il part par courriel. Détail : [journal du 08/11](../journal/2026-11-08.md).

## Reste

- Le direct en cours au moment de la mise en ligne n'aura pas de réunion d'office (son événement est passé) : « Créer la réunion Zoom » dans l'encart du direct.
- À essayer avec Zoom : création d'une réunion dont l'heure de début est déjà passée (démarrage en retard), lien d'animateur obtenu dans la minute.

- L'avis à l'écran (`in_app`) de l'annulation part toujours tout de suite (consommateur `engagement.notifications` existant) ; seul le courriel attend.
- Les rappels annulés avec la séance ne sont pas remis quand on la rétablit.
- Une réunion Zoom annulée (plus de cinq minutes après) n'est pas recréée au rétablissement : la régie montre « Créer la réunion Zoom ».
- Heures des créneaux et du journal affichées sans fuseau ligne par ligne (le fuseau est dit dans l'horloge, l'en-tête de la journée et le bloc principal), comme la maquette : à confirmer.
