# A13 bis : Envoi de courriels du back-office

> Extrait de la [progression](../../PROGRESSION.md). Demande urgente du commanditaire du 28/10, sans prompt dans
> [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) : l'équivalent de l'envoi de courriels de la v1, « similaire
> mais mieux développé, pas aussi désordonné ».

**État** : livré le 28/10 (modèle, API, worker, écrans) ; éprouvé au navigateur sur données d'exemple, pas contre l'API ni vers le serveur réel.

---

## Ce que faisait la v1 (relevé du 28/10)

Code : `epavillonvue/src/views/admin/EmailManager.vue`, `components/email/EmailManagerModal.vue`, `composables/useEmailSender.js`,
`supabase/functions/send-email/`, relais Laravel `laravel/email_polivalent/`. `send-flexible-email/` est vide, rien ne l'appelle.

- **Destinataires** : saisie libre en À, Cc, Cci (autocomplétion sur les comptes, collage séparé par virgules) ; ajouts
  rapides par événement (« tous les participants » : soumissionnaires, inscrits, panélistes) ou par activité
  (soumissionnaire, panélistes, inscrits) ; trois filtres groupés (activités datées, approuvées, en évaluation), en Cci.
  Liste construite dans le navigateur, dédoublonnée par `Set`. Ni rôles, ni liste de suppression.
- **Copie** : Cc et Cci, **pas d'adresse de réponse**. Cc par défaut selon l'écran (deux adresses personnelles de l'équipe,
  ou `epavillonclimatique@francophonie.org`).
- **Sémantique de copie** : **un message par destinataire À, chacun portant tous les Cc et Cci**. Pour N destinataires,
  l'équipe en copie reçoit N copies (exception : plus de cinq À sans variable et sans Cci, le relais passe les À en Cci
  et n'envoie qu'un message).
- **Contenu** : objet, corps en texte brut (`textarea`, `nl2br`), une trentaine de variables `{nom}` (destinataire,
  événement, activité, dates), modèles codés en dur (une douzaine). Aperçu sans les vraies valeurs, **pas d'essai**,
  pas de pièce jointe. Signature et pied de page ajoutés d'office.
- **Envoi** : synchrone depuis le navigateur, un appel par destinataire en parallèle, délai dépassé compté comme succès,
  aucune cadence. **Aucun historique** (l'onglet était « bientôt disponible »).

## Ce qui a été livré

| Couche | Où | Quoi |
|---|---|---|
| Modèle | `docs/database/110_engagement.sql` § 12 | `engagement.mailings` (brouillon, critères, adresses libres, Cc, Cci, réponse, auteur, dates), `engagement.mailing_recipients` (destinataires figés, valeurs des variables, critères d'origine, état), `engagement.mailing_audience()` (la liste, bornée à l'édition, dédoublonnée sur l'adresse, liste de suppression signalée), réglage `engagement.mailing.team_cc` ([]), sans limite d'envoi, permission `engagement.mailing.send` (rôle `admin`) |
| Migration | `ops/migrations/2026-10-28-envoi-de-courriels.sql` | rejouable, additive ; jouée deux fois en local |
| Noyau | `kernel/src/mail.rs` | `Copies` (Cc, Cci, réponse) et `Mailer::send_with_copies`, servie par le SMTP ; un transport qui ne sait pas la porter la refuse. Aucun courriel livré ne change |
| Garde d'envoi | `engagement/src/mail.rs` | les copies traversent la garde ; trace et liste de suppression sur le destinataire principal |
| API | `engagement/src/routes/mailings.rs`, `service/mailings.rs`, `repo/mailings.rs`, `domain/mailing.rs` | dix routes sous `/admin/mailings` : historique, brouillon (créer, lire, modifier, supprimer), liste avant envoi, essai, envoi, arrêt, duplication. Deux codes : `ENGAGEMENT_MAILING_NOT_DRAFT` (409), `ENGAGEMENT_MAILING_EMPTY` (422) |
| Worker | `engagement/src/jobs/send_mailing.rs` | `engagement.send_mailing` (file « email ») : lots de dix messages enchaînés sans attente, sans limite d'envoi ; `engagement.send_mailing_test` |
| Écrans | `pages/admin/courriels/index.vue`, `[id].vue`, `components/admin/mailings/` (7), `utils/mailing.ts`, `composables/api/admin-mailings.ts`, `types/admin-mailings.ts`, `mocks/admin-mailings.ts` | historique filtrable ; brouillon en quatre étapes ; fiche d'un envoi parti |
| Traductions | `i18n/locales/{fr,en}/pages/admin.mailing.{common,list,compose,record}.json` | trois fichiers au plus par page |
| Tests | `engagement/tests/envois.rs` (4), tests unitaires du domaine et du travail, `frontend/tests/site/mailing.test.ts` (3) | liste dédoublonnée, adresse supprimée écartée, copie sur chaque message, variables, refus hors périmètre, variable inconnue refusée, envoi parti non modifiable, duplication |

## Fenêtre unique (28/10, refonte après usage en production)

Les quatre étapes sont remplacées par une fenêtre de rédaction à la Gmail, ouverte de partout (icône enveloppe de la barre du back-office, « Écrire un courriel » de l'historique, un brouillon de la liste) : `components/admin/mailings/Composer.vue`, `AddressField.vue` (champ à puces, adresses et groupes, « + Groupe » reprend l'éditeur de critères), `ComposerOptions.vue` (réponse à, édition, gabarit et aperçu, valeurs de secours, bouton Répondre, bouton d'action, essai), `stores/mail-composer.ts`, `UiModal` (`fullscreen-mobile`), `UiRichText` (`buttons` : « Insérer un bouton »). Brouillon créé à l'ouverture, enregistré à chaque pause de frappe, supprimé à la fermeture s'il est vide. Synthèse en direct sous les destinataires (nombre, écartés, sans compte, valeurs de secours, poids des pièces) ; alertes en ligne ; confirmation en un clic au-delà de 20 destinataires. Une variable inconnue n'empêche plus l'enregistrement, seulement l'envoi.

API : `engagement/src/domain/gabarits.rs` (trois gabarits en HTML de messagerie, 600 px, tableaux et styles en ligne, couleurs de la charte, logo `/logos/png/epavillon@4x.png`, remplacé le 29/10 par le symbole intégré ; liens `class="bouton"` rendus en boutons ; `mailto:` du bouton Répondre), `GET /admin/mailings/{id}/preview` (rendu par la fonction du worker), `without_account` et `fallback_count` dans la liste avant envoi, `event_id` modifiable sur un brouillon (gardé sur la nouvelle édition). Tests : `envois.rs` (7), gabarits et secours (unitaires).

Vu au navigateur sur données d'exemple (1280 et 375 px, plein écran sur mobile). Non vérifié : le rendu dans Outlook et Microsoft 365, l'aperçu contre l'API réelle.

## Pièces jointes (28/10, second commit)

| Couche | Où | Quoi |
|---|---|---|
| Modèle | `050_media.sql`, `110_engagement.sql` | colonne `max_total_byte_size` de `media.attachable_roles` et son contrôle dans `tg_validate_attachment()` ; ligne `engagement.mailings` / `attachment` (multiple, PDF, bureautique, texte, images ; 10 Mo par fichier, 10 Mo au total) |
| Garde | `media/src/domain/guards.rs`, `service/authz.rs`, `repo/cross.rs` | `Garde::EnvoiAdministre` : `engagement.mailing.send` sur l'édition de l'envoi, avec le périmètre ; dépôt toujours privé |
| Media | `service/upload.rs`, `service/attach.rs`, `repo/attachments.rs` | poids cumulé refusé à l'annonce, au dépôt et au rattachement : `MEDIA_TOTAL_TOO_LARGE` (413) |
| Noyau | `kernel/src/mail.rs` | `Copies` devient `Extras` (copie, copie cachée, réponse, pièces jointes) ; le SMTP compose un message `mixed` ; le relais HTTP refuse plutôt que de perdre |
| Engagement | `repo/mailings.rs`, `service/mailings.rs`, `jobs/send_mailing.rs`, `lib.rs` | les pièces jointes se lisent dans `media` ; le travail lit les octets par `kernel::storage::Entrepots` (comme `negotiation` lit ses PDF), une fois par lot ; essai compris ; une pièce pas prête (analyse) retient l'envoi et l'essai |
| Écrans | `components/admin/mailings/Attachments.vue`, `[id].vue`, `Review.vue`, `Record.vue`, `index.vue` | dépôt par `useDepotMedia` à l'étape Message (contrôle du type et du poids cumulé avant l'envoi du fichier), liste et poids total à la Vérification et sur la fiche, nombre dans l'historique ; « Dupliquer » rattache les mêmes objets à la copie |
| Tests | `media/tests/pieces_jointes.rs` (3), garde (unitaire), `engagement/tests/envois.rs` (pièce jointe sur chaque message, pièce pas prête), Mailpit (`kernel/tests/envoi_smtp.rs`), relais (unitaire) | |

Non vérifié : le dépôt au navigateur (les données d'exemple ne rattachent rien), et un message avec pièce jointe vers le serveur réel.

## En-tête, logo d'édition et gabarits de l'édition (29/10)

Demande du commanditaire, en-tête validé sur un autre courriel. Par un agent dans un worktree, contrôles ciblés.

| Couche | Où | Quoi |
|---|---|---|
| Modèle | `050_media.sql`, `110_engagement.sql`, `ops/migrations/2026-10-29-logo-edition-et-gabarits.sql` | rôle `logo` sur `event.events` (unique, `image/*`, 5 Mo, sans forme) ; `ck_mailings_template` admet `banniere_edition` et `institutionnel_edition` |
| Noyau | `kernel/src/mail.rs` | `Extras.inline_images` (`InlineImage` : identifiant, type, octets) ; `composer()` public rend le MIME (`alternative` texte puis `related` HTML et images, dans un `mixed` s'il y a des pièces jointes) ; le relais HTTP refuse une image intégrée |
| Gabarits | `engagement/src/domain/gabarits.rs`, `assets/logo-epavillon-symbole-inverse.png` | en-tête bleu nuit : symbole inversé intégré (`cid:logo-epavillon`, 894 octets, 42 x 40), « ePavillon » sur deux lignes comme la barre publique ; « Bannière de l'édition » (bannière 600 px cliquable vers `/evenements/<slug>`, bande blanche logo à gauche, nom et dates à droite, liseré) ; « Institutionnel + logo de l'édition » (logo sur tuile blanche à droite). Replis sur l'institutionnel sans bannière ou sans logo. Image choisie parmi les variantes JPEG ou PNG (`lg` 1600 pour la bannière, la plus petite d'au moins 120 px de haut pour le logo), jamais le WebP |
| Lecture | `engagement/src/repo/cross.rs` (`visuels_edition`), `jobs/send_mailing.rs` (`edition_de_lenvoi`) | nom, période dans le fuseau de l'édition, ville, slug, `media.attached_image()` pour `banner` et `logo` ; lue seulement pour les deux gabarits de l'édition |
| Aperçu | `service/mailings.rs` | le symbole cité en `data:` (`pour_apercu`), le reste identique au message |
| Écrans | `ComposerOptions.vue`, `admin.mailing.composer.json` (fr, en) ; `ImagesPanel.vue`, `EditionForm.vue`, `types/media.ts`, `admin.event.form.json` | deux gabarits de plus ; quatrième emplacement « Logo de l'édition » dans les visuels d'une édition, par `MediaImageField`, sans forme imposée |
| Essai | `engagement/tests/gabarits_essai.rs` (ignoré par défaut) | écrit trois `.eml` complets par le vrai code, images de l'édition intégrées pour l'essai |

Lot 101 (`ops/reprise-v1/lots/101-visuels-editions.sh` et `.sql`) : bandeau (photo de Belém, 16:9) et logo de la CdP30, logo de la CdP31. Répété sur une copie de `epavillon-20261028-204320` (migration du 28/10 puis celle du 29/10, deux fois) : 3 objets, 3 rattachements, 3 traitements en file, 3 événements d'outbox marqués publiés ; seconde passe refusée ; base jetable supprimée. Fichiers CdP30 copiés sous `sauvegardes/reprise-v1/fichiers/ancien-serveur/` (hors git). **Pas joué en production.**

Non vérifié : rendu réel dans Outlook et Microsoft 365 (les `.eml` d'essai sont à envoyer par le commanditaire) ; dépôt du logo au navigateur ; le rôle `banner` annonce un 32:9 et les bandeaux CdP30 et CdP31 sont des 16:9 (la forme ne se contrôle qu'à la pose, quand les dimensions sont connues : une nouvelle pose de ces fichiers depuis le back-office serait refusée).

## Retours sur les essais, gabarits personnalisés (29/10, second passage)

- **Mises en page** (codes définitifs) : `sobre`, `institutionnel`, `annonce`, `banniere` (image 600 x 200 puis bande blanche logo, nom, dates), `texte_sur_image` (nom et dates en blanc sur l'image ; image de fond CSS, rectangle VML pour Outlook bureau, aplat bleu nuit sans prise en charge ; voile comme `.scrim-fade-bottom`, facultatif), `institutionnel_logo` (logo de 56 px sur tuile blanche, marges de la bande réduites à 4 px : en-tête de 72 px comme sans logo). Sans image 3:1 ou sans logo : l'institutionnel.
- **Image de tête** : seulement un fichier 3:1 (`choisir_fond`) : la variante media `courriel` (1200 x 400, JPEG qualité 62, `media/domain/variants.rs`, `imaging::decliner`) ou un fond déposé déjà recadré. Le worker complète un objet déjà servable (variante manquante, sans nouvelle analyse) ; la migration met en file les bandeaux d'édition existants. Bannière CdP31 : 113 Ko au lieu des 416 Ko de `lg`.
- **Gabarits personnalisés** : `engagement.mail_templates` ; `GET/POST /admin/mail-templates`, `GET/PUT/DELETE /admin/mail-templates/{id}`, `GET …/{id}/preview` (`service/mail_templates.rs`) ; fond et logo rattachés par le module media (garde `GabaritDeCourriel`). Textes et images du gabarit remplacent ceux de l'édition (`send_mailing::apparence`). Page `/admin/courriels/gabarits` (`pages/admin/courriels/gabarits/index.vue`, `[id].vue`) : liste de l'édition et des communs, formulaire, `MediaImageField` (fond 3:1 imposé par la règle, 1200 px, JPEG ou PNG, qualité 70 ; logo 400 px), aperçu en direct rendu par le worker. `UiImageEditor` et `MediaImageField` gagnent `maxWidth`, `formats`, `defaultQuality`.
- **Fenêtre de rédaction** : modèles par défaut, puis gabarits de l'édition ; `mail_template_id` sur l'envoi (« Dupliquer » le garde) ; présélection quand `MailingAudience.suggested_mail_template_id` est rempli et que l'auteur n'a rien choisi.
- **Tests** : `engagement/tests/gabarits_personnalises.rs` (2), unitaires des gabarits, `media/tests/traitement.rs` (variante `courriel`, objet servable complété), garde (unitaire).
- **Essais** : quatre `.eml` sur la CdP31 (bannière basse, texte sur l'image, institutionnel avec logo, gabarit personnalisé), images intégrées pour l'essai ; la variante `courriel` produite par `cargo run -p media --example declinaison_courriel`.

Non vérifié : rendu Outlook (VML compris) et Microsoft 365 ; la page des gabarits au navigateur ; le dépôt d'un fond par l'éditeur borné.

## Le parcours

1. **Historique** (`/admin/courriels`) : les envois de l'édition choisie, filtrables par état (onglets) et par objet ou
   auteur ; « Écrire un courriel » crée tout de suite un brouillon enregistré.
2. **Destinataires** : quatre familles nommées qui s'additionnent (déposants et contacts des dossiers, intervenants,
   inscrits, équipe de l'édition), chacune avec ses filtres (état du dossier, état de l'inscription, rôle, activité),
   plus des adresses libres.
3. **Message** : objet, corps dans `UiRichText` (le même éditeur que le reste du back-office), variables `{{prenom}}`,
   `{{nom}}`, `{{organisation}}`, `{{email}}`, `{{edition}}` à copier, et le bloc **Réponses** (copie proposée : l'adresse
   de qui écrit et `team_cc`, copie cachée, adresse de réponse), avec la phrase qui en explique l'usage.
4. **Vérification** : décompte de la base, nombre de messages remis au serveur (copies comprises), mises en garde chiffrées (adresses écartées,
   destinataires sans prénom, copies par adresse en copie), aperçu rendu « vu par » un destinataire au choix, essai à soi,
   liste complète avec recherche.
5. **Envoi** : confirmation, destinataires figés, file du worker. La fiche montre ensuite chaque destinataire et son état
   (en file, en cours, parti, échec avec le message du serveur, écarté, non envoyé), avec « Dupliquer » et « Arrêter ».

## Décisions et écarts

1. **Copie : la sémantique v1 est reprise**, un message par destinataire, chacun avec la copie. C'est ce qui fait voir
   les réponses à l'équipe quand le destinataire répond à tous. **Coût dit à l'écran** : 300 destinataires, 300 copies
   par adresse en copie. **Proposé sans l'imposer** : mettre l'équipe en adresse de réponse plutôt qu'en copie ; les
   réponses lui arrivent, et l'historique remplace la copie comme trace de ce qui est parti.
2. **Sans limite d'envoi, décision du commanditaire le 28/10.** Une cadence horaire (200 destinataires par heure) avait
   d'abord été posée, faute d'en trouver une dans le code ; elle est retirée du modèle, de l'API et des écrans. Les lots
   de dix s'enchaînent sans attente ; le marquage « en cours » avant l'expédition empêche un double envoi à la reprise,
   et un serveur injoignable rend le destinataire à la file. La limite horaire de l'ancien hébergement existe, de valeur
   inconnue (`DEPLOIEMENT.md` § 10) : un refus du serveur se lit dans l'état de chaque destinataire.
3. **Un envoi appartient à une édition** (`event_id` obligatoire) : règle n° 8 tenue par construction. Les adresses
   libres restent permises à un administrateur d'édition : il les tape lui-même, elles ne révèlent personne.
4. **Modèles : pas de modèles réutilisables**, la duplication d'un envoi passé en tient lieu. `message_templates` est
   lié aux types de notification et multilingue, trop lourd pour ce jalon.
5. **Pièces jointes** : ajoutées le 28/10 à la demande du commanditaire (voir plus haut). Pas de signature ajoutée : l'écran demande de signer.
6. **Critère par activité** : la liste des activités vient de `/admin/showcase/sessions`, qui ne rend que les séances
   **publiées** ; une activité non publiée ne se cible pas encore seule (tous les critères restent possibles sur
   l'édition entière).
7. **Le texte brut** est dérivé du HTML (paragraphes et listes gardés) ; `{{edition}}` est rendu en français.

## Vérifications

- **Au navigateur, sur les données d'exemple** (thème sombre, 1280 et 375 px, sans défilement horizontal) : historique,
  création d'un brouillon, les quatre étapes, aperçu « vu par », essai, confirmation, fiche d'un envoi parti, duplication.
  Corrigé en chemin : la page ne se remontait pas en passant d'un envoi à sa copie (`key` de page), un en-tête d'aperçu se
  coupait au milieu d'un mot, une case vide dans les compteurs à 375 px.
- **Mailpit** relit un message parti avec copie, copie cachée et adresse de réponse (`kernel/tests/envoi_smtp.rs`).
- **Base** : `mailing_audience()` sur la base locale (édition CdP31 de la reprise), frontières de modules conformes.

## Ce qui n'a pas été vérifié

- Le parcours au navigateur contre l'API réelle (API et worker non lancés : la base locale est partagée avec une autre
  session, dont le worker aurait pu prendre les travaux).
- L'envoi par le serveur SMTP de l'ancien hébergement, et sa limite horaire réelle.
- Seul le thème sombre a été regardé ; le thème clair suit les jetons.
