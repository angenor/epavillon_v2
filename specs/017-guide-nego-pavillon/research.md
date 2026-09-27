# Research — Guide Négo, Pavillon de la Francophonie (étape 5)

## R1 — Servir ce que la base a déjà, dans `programme`, par ajouts seulement (tranché le 26/09)

| Manque | Source déjà en base | Où l'ajouter | Public aujourd'hui ? |
|---|---|---|---|
| Rediffusion (lien, durée) | `live.streams` : disponible = (`kind = 'replay'`, non annulée) ou (`kind = 'live'` avec `replay_url`), `replay_available_at` nul ou passé ; **une seule par séance** par `LEFT JOIN LATERAL (… ORDER BY is_primary DESC, replay_available_at DESC LIMIT 1)` ; URL = `COALESCE(replay_url, watch_url, live.build_embed_url(provider, embed_id))` ; durée = `round(media.assets.duration_seconds)::int`, sinon `ended_at − started_at` de la ligne ou de son direct d'origine (`replay_of_id`) | colonnes ajoutées **à la fin** de la vue, mais la vue se **redéfinit dans `080_live.sql`** (chargé après `075` : `live.streams` n'existe pas encore quand `075` crée la vue) ; `075` garde un renvoi en commentaire | Oui : la diffusion l'est (`is_streamed`, canal) ; tranché le 26/09 |
| Langue | `programme.proposals.language_codes` par `proposal_id` (`LEFT JOIN` : nulle sans dossier) | colonne `language_codes` de la vue (dans `075`) | Oui : langue d'une activité annoncée ; tranché |
| Liste d'attente, inscription | `programme.sessions` (`waitlist_enabled`, `registration_required`, `registration_opens_at`, `registration_closes_at`), `count(*)` des `waitlisted` | colonnes de la vue | Oui : ce sont les conditions d'inscription d'une activité publiée |
| Empreinte | `listing_changed_at` et le corps | `ETag`/`304` sur `GET /schedule` et `GET /registrations/mine` (`kernel::empreinte`) | — |
| Noms des intervenantes | `programme.session_speakers.person_id` → `identity.people.display_name` | champ `display_name` **ajouté** aux objets `speakers` du détail public, par une lecture composée champ par champ (`jsonb_build_object`), **jamais** `to_jsonb(people)` ni `to_jsonb(org)` (courriels, téléphones) | La route publique sert déjà les intervenants (fonction, organisation, biographie) : le nom complète ce que son contrat annonce. **Signalé à l'orchestrateur**, avec la fuite existante (ci-dessous). |
| Noms des organisations | `programme.session_organizations.organization_id` → `org` | champs `name` (← `legal_name`), `acronym`, `country_code`, `country` (i18n) ajoutés, comme la vue | Oui : l'organisation porteuse l'est déjà dans la vue |

**Fuite existante, à trancher** : le détail public sert aujourd'hui `to_jsonb` de `session_speakers`, donc
`attended`, `confirmed_at` et `person_id`. Aucun écran du site ne l'appelle. Proposition : la lecture
publique dédiée ne les sert plus (exception à « ajouts seulement », sans effet sur le site) — question à
l'orchestrateur. Registre des lectures hors schéma de `programme` (`repo/cross/mod.rs`) : `live.streams`,
`media.assets` ajoutés.

**Jamais servis** : courriel, téléphone, adresse, pièces du dossier, notes d'évaluation — aucune colonne du
dossier de proposition autre que `language_codes`. Un test compare les clés servies avant et après : que
des ajouts.

**Le site** : `test:site` ne couvre pas le programme ; la preuve est donc `npm run typecheck`,
`check-api-contract`, un test du nombre de lignes de `/schedule` (identique avant et après) et **la page
`/programme` du site vérifiée au navigateur (bloquant)**. `make sqlx-prepare` après tout changement de
requête (l'image de production se construit hors ligne).

## R2 — Une lecture pour toute l'édition

`GET /schedule?event_id=` rend toute l'édition (sans plafond) : l'application la garde (`pavillon:<slug>`)
et en tire le jour, la veille, les jours suivants, la ligne de « Ma journée » et la cible de l'étiquette
(par `pavilion_session_id` → ligne → `slug`). Le détail par slug (`GET /events/{id}/sessions/{slug}`) se lit
à l'ouverture d'une fiche et se garde (`pavillon-activite:<édition>:<slug>` — un slug n'est unique que
par édition). Aucune route par identifiant. **Une bande des jours couvre toute l'édition**, jours passés
compris : les activités passées s'atteignent avec leur rediffusion ; « Hier — rediffusions » reste sur la
vue du jour.

## R3 — L'inscription : le mécanisme existant

`POST /sessions/{id}/registrations` (six issues en 200 : `registered`, `waitlisted`, `already_registered`,
`full`, `closed`, `not_open_yet`), `POST /registrations/{id}/cancel`, `GET /registrations/mine`. Aucune
référence client n'est nécessaire : l'unicité (personne, séance) rend `already_registered` au rejeu.

Hors connexion : intention `inscription-pavillon:<session_id>` portant l'état voulu (`inscrite` avec les
réponses, ou `annulee`) ; la dernière gagne ; une annulation qui suit une inscription pas encore partie
les annule toutes deux (rien ne part). Au départ d'une annulation, l'identifiant est celui de la ligne
de la séance **au statut autre que `cancelled`** dans `mes inscriptions` ; introuvable, ou `404` au rejeu →
succès ; **`RegistrationLocked` (422) est un refus** (la base a refusé), dit à l'écran. Activité sans
inscription, annulée ou reportée (`RegistrationNotAccepted`) : aucun bouton, état « Sans inscription »
ou l'état de l'activité.

## R4 — Le formulaire dans l'application (tranché le 26/09)

`GET /sessions/{id}/registration-form` (public, gardé avec la fiche). Les **seuls types de champ que le
modèle définit** (`RegistrationFormField`) se rendent, avec les composants de Guide Négo ; aucun type
nouveau. Le pays se préremplit depuis le profil de 0c : `identity.people.country_id` (uuid) converti en code ISO2
(`useGnPays` garde désormais `iso2`) ; profil sans pays → champ vide. Un champ sensible demande
`sensitive_data_consent`, comme le site. Un formulaire sans champ obligatoire, ou dont le seul est le
pays, s'envoie d'un geste.

## R5 — Jamais filtré par « mes thématiques »

Décision du 22/09 (spec 010, FR-011) : aucune règle de filtre ne s'applique au Pavillon, un test le garde.

## R6 — Le scénario qui clôt le MVP

Il traverse 0b (code), 1/1b (guide hors connexion), 2 (lexique), 3a (sessions de mes thématiques, « lu à »),
3b (signaler, valider, prévenir) : il ne se joue qu'une fois les étapes 2 et 4 fusionnées dans `main` et
`main` fusionnée dans cette branche (3a et 3b y sont déjà). La recette le joue alors sur la version construite ; avant, elle joue
tout le reste.
