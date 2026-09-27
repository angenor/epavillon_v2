# Points ouverts de Guide Négo

> Le détail de chaque point, déplacé tel quel de l'ancien suivi le 27/09. Le [point central](../progress.md) n'en garde qu'une ligne ; un point levé est rayé ici et retiré de là-bas.

## La réinitialisation du site atteint Guide Négo

**La réinitialisation générale du site atteint Guide Négo**, relevé le 24/09 à l'essai 1b : la préface de Tailwind, importée par `frontend/app/assets/css/main.css`, pose `* { box-sizing: border-box; margin: 0; padding: 0; border: 0 solid }` sur toute la page, application comprise — contraire à FR-026 de 0a (« aucune règle du site ne doit altérer l'application »). `assets/guide-nego/base.css` reprend `box-sizing: border-box` pour lui-même, ce qui la masquait. Elle décalait la couche de texte de pdf.js ; 1b s'en protège par une règle `content-box` sous `.pdfViewer` ([ADR-022](../adr/022-pdfjs-dans-le-client.md)). **Non corrigée à cette étape** : la borner demanderait de sortir Guide Négo de la couche de base de Tailwind, et de revoir tous ses composants, qui comptent sur elle.

## Levé — le périmètre d'administration des espaces de négociation

~~Le périmètre d'administration ne couvre pas les espaces de négociation.~~ **Tranché le 21/09 par le commanditaire** : pour Guide Négo, « son périmètre » veut dire **global**. Seul un administrateur de la plateforme entière tient les codes, les usages, les demandes et le mode d'admission ; la garde des douze routes est `negotiation.space.manage` **sur la portée globale** (`Requires<SpaceManage>`), et **surtout pas `RequiresAnyScope`** — le rôle `admin` porte cette permission et s'attribue aussi sur un événement, si bien qu'un administrateur d'une seule édition franchirait « n'importe quelle portée » alors qu'aucun espace de négociation n'est rattaché à une édition. **Aucune fonction de périmètre nouvelle**, aucun lien espace ↔ événement, aucun semis de rôle : `identity.administered_events()` reste ce qu'elle est. Corrigé dans `contracts/api-admin.md`, `spec.md` (FR-044 et SC-008), `plan.md` et T067, T070, T073. **Construit et éprouvé le 22/09** : les douze routes portent `Requires<SpaceManage>`, et `perimetre_url_forgee.rs` refuse toute garde plus large — y compris en relisant le code des trois fichiers de routes, commentaires retirés.

## Confier un espace de négociation à une personne qui n'administre pas la plateforme

**Confier un espace de négociation à une personne qui n'administre pas la plateforme** — un référent de COP, par exemple. Rien n'est construit pour cela, et c'est délibéré. Il faudrait le rôle `space_lead`, `negotiation.space.manage` sur la portée `negotiation_space`, et une fonction sœur d'`administered_events()` qui rende les espaces confiés. **Une spécification à part, si l'IFDD le demande.**

## Levé — `useApi.ts` près des mille lignes

~~**`frontend/app/composables/useApi.ts` est à 973 lignes**, le garde-fou est à mille : **vingt-sept lignes de marge**.~~ **Levé le 22/09, phase 1 de 0c** : `auth` vit dans `composables/api/auth.ts`, `useApi.ts` est à 895 lignes, les 14 appelants sont inchangés. La phase 5 n'y a posé que le montage du back-office de Guide Négo (neuf lignes), son code vivant dans `composables/api/admin-negotiations.ts`. Le bloc `auth` y est une constante : c'est le premier candidat à sortir dans `composables/api/auth.ts`, sans rien changer à ses appelants. **Le prochain bloc écrit en ligne franchit la limite** — la phase 7 devra sortir `auth` avant d'y toucher.

## Levé — R3, une adresse par écran et le français partout

~~R3 — une seule adresse par écran, et le français quel que soit le téléphone.~~ **Levé le 21/09**, vérifié en T001 au navigateur. La route non localisée s'affiche en `fr`, `/en/guide-nego` rend 404, et le cookie de langue du site reste à sa valeur — `plugins/guide-nego-langue.client.ts` le rétablit après le module i18n, qui le réécrivait sans condition. Consigné dans [research.md § R3](../../../specs/008-guide-nego-coquille/research.md).

## Les lignes « En attente » du journal de maquette

Les lignes « En attente » du journal de maquette : des données à fournir, pas du code.

## Les deux textes qui engagent

**Les deux textes qui engagent sont à fournir par l'IFDD** — politique de confidentialité et conditions d'utilisation, `fr` et `en`. Ils se déposent dans `backend/crates/kernel/src/legal/` (`etat: publie`, la date d'entrée en vigueur pour version) ; le mode d'emploi est dans `LISEZMOI.md` du même dossier. Aucun outil ne les écrit.

## L'hébergeur à nommer dans « À propos »

**L'hébergeur à nommer dans « À propos »** : le commanditaire le dira avant qu'on l'écrive. D'ici là, l'écran n'en nomme aucun.

## L'accord du secrétariat de la CCNUCC

**L'accord du secrétariat de la CCNUCC** sur la reprise du calendrier des sessions : demande en cours. L'étiquette de source n'en parlera qu'une fois l'accord obtenu.

## Une clé de traduction manquante sur le site

Écart de clés i18n **préexistant** au site, relevé le 21/09 en comparant les 105 fichiers : `pages/home.json` porte `aside.description` en anglais et pas en français. Sans effet sur Guide Négo, dont les 12 fichiers nouveaux concordent clé pour clé.

## « Réservé aux négociateurs » sur la page de maintenance du site

**« Réservé aux négociateurs » sur la page de maintenance du module Négociations du site** (`i18n/locales/fr/pages/maintenance.json`) — la forme du lexique de Guide Négo est « négociatrices et négociateurs ». Hors du périmètre de 0b, donc **non corrigé** : c'est un texte du site.

## Créer un compte ne consigne aucun consentement

**Créer un compte ne consigne aucun consentement**, constaté le 22/09 : aucune ligne dans `identity.consents` après une inscription, alors que le commentaire de `frontend/app/pages/auth/register.vue` dit l'API chargée de l'écrire. Seule l'inscription à une séance qui pose une question sensible en écrit un. Préexistant, hors de Guide Négo : **à trancher par le commanditaire** — consigner l'accord aux conditions à la création du compte, ou corriger le commentaire.

## La mise en ligne de 0a, 0b et 0c

**La mise en ligne de 0a, 0b et 0c est préparée, pas exécutée** — [§ 15 de DEPLOIEMENT.md](../../DEPLOIEMENT.md) : trois migrations dans l'ordre, répétition sur une copie de la production, vérifications du site, puis la recette sur téléphones réels en une séance. Le commanditaire l'exécute.

## Les objets désignés par une colonne sans être rattachés

**Les objets désignés par une colonne sans être rattachés** — `media.find_orphan_assets()` ne regardait que `media.attachments`. Le registre `media.asset_references` (étape 1) corrige le cas des documents de Guide Négo, **seuls déclarés**. Restent à déclarer, par leur module : `programme.proposal_documents.asset_id` et les colonnes d'objet de `publication`, `training`, `live`, `engagement`, `programme.sessions`. Sans effet visible tant qu'aucun écran ne liste les orphelins ; **avant d'en construire un**, les déclarer.

## Les restitutions et leur partage par cercle

Les restitutions et leur partage par cercle : proposés, à confirmer par entretiens.

## Étape 1 — limites connues de l'extraction

**Étape 1 — limites connues de l'extraction**, relevées au contre-examen et laissées : un filigrane ou une bande latérale répétés hors des marges deviennent une figure sur chaque page, et le document passe « tel quel » ; un fond de couleur pleine page dessiné en tracé (et non en image) masque le filet des notes, qui restent dans le texte. Le vrai guide n'a ni l'un ni l'autre.

## Étape 1 — le jeu d'exemple des documents

**Étape 1 — le jeu d'exemple des documents** ne simule ni les permissions (tout est permis hors ligne, aucun 401 ni 403) ni les images de page et le PDF (chemins relatifs, 404 sans API).

## Étape 1 — ce que `check-api-contract` ne vérifie pas

**Étape 1 — `check-api-contract`** ne compare pas les verbes, ne lit pas une réponse annoncée à deux formes (« DocumentLibrary, ou DocumentTextHits avec q ») et ignore les constructeurs de chemin (image, PDF). Et rien ne vérifie que `backend/.sqlx` est à jour : `cargo sqlx prepare --check` gagnerait à entrer dans `check-back`.

## Étape 1 — relevés de la recette du back-office (23/09)

**Étape 1 — relevés de la recette du back-office (23/09), hors de Guide Négo, non corrigés** : les dates des listes prennent le fuseau du serveur au rendu, puis celui du navigateur (écart d'hydratation, hérité des codes) ; un expert voit aussi les entrées générales du back-office, qui lui répondent « accès refusé ». Les deux autres — l'élision de « heure d'Antalya » et la page rechargée qui passait par la connexion — sont corrigés sur `main`.

## Étape 1 — relevés de la phase 8

**Étape 1 — relevés de la phase 8, laissés** : « Ajouté » d'un lien ne porte que la date, dans le fuseau du téléphone — la maquette dit « Hier, 22:40 — heure d'Antalya », mais la liste ne sert pas le fuseau de la COP ; une panne de la recherche dans le texte passe pour « aucun trouvé dans le texte », et les documents trouvés par le texte disparaissent le temps d'une frappe ; les spécimens de la planche des documents portent des types en i18n, comme ceux de 0a leurs thématiques ; « Tout se lit sans connexion », sous-titre de Ressources, ne vaut plus pour un document non téléchargé.

## Étape 1 — un refus de l'API compté comme une panne

**Étape 1 — `useGnLecture` compte un refus de l'API comme une panne de réseau** (`noterEchec`) : un 401 ou un 403 sur une lecture fait passer toute l'application hors connexion. Préexistant (0a) ; contourné pour les favoris seuls. Et **une route publique ne dit pas qu'un jeton a expiré** : la phase 8 le déduit côté téléphone (`lireEnPersonne`) ; un signal de l'API serait plus sûr.

## Étape 1 — la recette des réservés (T102)

**Étape 1 — la recette des réservés de la phase 11 n'est pas faite** (T102, étapes 1, 4 et 5 du [quickstart § 5](../../../specs/011-guide-nego-documents/quickstart.md)) : aucun compte de recette n'a l'accès négociateur, et l'ouvrir en base a été refusé à l'agent — c'est une attribution de droit. Il suffit d'un compte admis par un code d'invitation du back-office de 0b ; le document « Résumé réservé — recette » attend en base. Le chemin est couvert sans navigateur depuis la phase 7 (`effacerLesReserves`, la génération des réservés).

## Étape 1 — relevés de la phase 13

**Étape 1 — relevés de la phase 13, laissés** : un filtre de thématique venu d'une adresse, sur une thématique qu'aucun document ne cite, s'affiche en code brut (« Thématique : gender ») — le vocabulaire servi ne porte que les valeurs citées ; sans API et sans rien de gardé, un navigateur neuf voit « pas encore ouvert », par la règle de 0a (ce qui n'a jamais été lu ouvert reste fermé) ; la recette a trouvé `localhost:8080` pris par un Adminer en IPv6 et le port 3000 par un autre projet — l'API écoute sur `127.0.0.1`, et le navigateur tente `::1` d'abord.

## Étape 1 — relevés de la phase 12

**Étape 1 — relevés de la phase 12, laissés** : la signature dit « expert IFDD » quel que soit l'expert — la qualité vient du rôle, et le modèle ne porte pas d'accord (« experte ») ; un extrait sélectionné qui commence sur une puce de liste (« • … ») ne se retrouve pas et la note va en tête de page ; sur la fiche d'un document, l'expert voit les thématiques et le choix du PDF actifs, sans bouton d'enregistrement — l'API refuse, mais l'écran ne le dit qu'au refus (hérité de la phase 6).

## Étape 1 — relevés de la phase 11

**Étape 1 — relevés de la phase 11, laissés** : `/guide-nego` sans barre finale est hors de la portée du service worker (`/guide-nego/`, l'adresse de départ de l'application installée) — tapée dans un navigateur, elle ne s'ouvre pas sans réseau ; « utilisés par Guide Négo » compte toute l'origine, site compris quand ils partagent un domaine ; la place d'une copie dépubliée se rend à la relecture de la liste, pas avant.

## Étape 1 — relevés de la phase 10

**Étape 1 — relevés de la phase 10, laissés** : la section d'un passage est celle de sa page, pas de sa position — un passage au-dessus d'un titre porte la sous-partie qui s'ouvre après lui ; une recherche de deux lettres très courantes (« de ») surligne des milliers d'occurrences à l'ouverture du premier passage ; la ligne de page de la barre, devenue bouton, fait 32 px comme la maquette, sous les 48 de la règle ; un terme anglais touchable reste une cible en ligne, sous 44 px (exception des cibles dans le texte).

## Étape 1 — relevés de la phase 9

**Étape 1 — relevés de la phase 9, laissés** : en « tel quel », la place réservée est celle d'une page A4 — une page à l'italienne glisse un peu quand son image arrive ; la recette sur Android et iPhone réels reste à faire, comme pour 0a à 0c.

## « Au nom de {nom} » n'élide pas

**« Au nom de {nom} »** (`guide-nego.demande.json`) n'élide pas devant un nom qui commence par une voyelle (« Au nom de Aïssatou »). Même défaut que « heure de Antalya », sur un nom propre : `zoneElides()` en donne la règle.

## Étape 1 — deux contre-examens interrompus (23/09)

**Étape 1 — deux contre-examens interrompus le 23/09**, pour tenir le délai : la relecture des renforts de tests de la phase 5 (six lots sur sept) et celle des corrections de la lecture PDF (pages tournées, pages qui ne se chargent pas). Les tests passent ; à relancer si l'on veut la même assurance qu'ailleurs.

## Deux tests de `identity` sensibles à la concurrence

`make check-safe` : deux tests Rust de `identity` (`toute_ecriture_laisse_son_auteur`) ont échoué une fois le 21/09 pendant que l'API locale et un serveur de développement tournaient, puis passé seuls et dans la porte complète, API arrêtée. Aucun Rust n'avait été touché. À surveiller : ils semblent sensibles à une activité concurrente sur la base.

## `make check-safe` en fin de cycle seulement

`make check-safe` ne se lance plus qu'à la fin d'un cycle de travail, demandé le 21/09 : la porte complète (typecheck Nuxt, build et tests de tous les crates) coûte plusieurs minutes. Entre-temps : `npm run check:guide-nego`, `test:guide-nego`, `typecheck`.

## Levé — le groupe Réunions de la Francophonie dans la recherche globale

~~**À la fusion de l'étape 2 : groupe Réunions de la Francophonie dans la recherche globale (`reunionsTrouvees`), clé `pas-encore` réduite au Pavillon**~~ **Fait le 27/09** (T029 de l'étape 4).

## Étape 5 — rien n'écrit `live.streams`

**Étape 5 — rien n'écrit `live.streams`** : sans outil de saisie (le back-office du direct, qui appartient à l'ePavillon), la rediffusion d'une activité ne paraîtra pas en production. La recette l'a posée en SQL ; le § 15 de `DEPLOIEMENT.md` donne la requête pour la séance sur téléphones.

## Étape 5 — le back-office du site n'édite pas les formulaires d'inscription

**Étape 5 — le back-office du site n'édite pas les formulaires d'inscription** : une activité à formulaire complet (question de plus que le pays) ne se prépare aujourd'hui qu'en base.

## Levé le 27/09 — étape 5, T021, le scénario qui clôt le MVP

**Étape 5 — T021, le scénario qui clôt le MVP** (quickstart § 5 de `specs/017-guide-nego-pavillon/`), reporté : il se joue désormais (étape 2 et 016 fusionnées dans 017 le 27/09), sur la version construite à 360 px.

## Deux fuseaux sur un même écran

**Deux heures d'un même écran ne se lisent pas dans le même fuseau**, relevé par dev2 le 27/09 à la recette de l'étape 5, non corrigé : un téléphone réglé sur un autre fuseau que celui de la COP affiche « lu à » à son heure, mais « validé à » à l'heure de la COP.
