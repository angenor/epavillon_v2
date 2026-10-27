# PACO : relevé de la v1 et voie de mise en ligne dans la v2

> Relevé du 27/10/2026, en lecture seule, sur le dépôt v1 (`epavillonvue/`) et le schéma de production extrait le même jour (`sauvegardes/reprise-v1/schema-public-v1.sql`, archive `supabase-v1-20261027-0419.dump`). Décision D9 (`ecarts.md`) : PACO n'est pas repris dans le modèle v2 ; le bloc est posé tel quel, apparence et données, et **reste branché sur Supabase** (§ 6). Recommandation : servir le build v1 lui-même par Apache.

## 1. La prochaine séance : jeudi 29 octobre 2026, 14 h 00 à 15 h 30 GMT

Session 10 (édition `10` en base), « Évaluation d'impact des programmes d'adaptation climatique », sous-titre « Les essais contrôlés randomisés pour mesurer l'effet réel des programmes ». Seule séance marquée `completed: false` dans `usePacoWebinarData.js` ; intervenants « À confirmer » (PIK Potsdam, LARDES Université de Parakou, un troisième sans organisation). **1 002 inscrits** à cette édition dans l'archive du 27/10.

Les inscriptions ferment seules 30 minutes après la fin (15 h 30 + 30 min, soit 16 h 00 GMT). Le bouton « Rejoindre » n'ouvre Teams qu'à partir d'une heure avant le début (13 h 00 GMT), sinon une fenêtre propose « Rejoindre quand même ».

**Depuis le 26/10, `https://epavillonclimatique.francophonie.org/paco` et `/paco/join` tombent sur la 404 de la v2** : aucune page, aucune redirection ne les sert. Or c'est le lien des rappels envoyés aux inscrits (`/paco`) et des anciennes confirmations (`/paco/join`).

## 2. Ce que fait PACO

Un cycle mensuel de webinaires (Teams) de l'IFDD avec la GIZ, l'IKI, le BMU et le PIC, un dernier jeudi du mois, 14 h 00 à 15 h 30 GMT. Trois pages, sans compte pour le public :

| Adresse v1 | Vue | Garde | Rôle |
|---|---|---|---|
| `/paco` | `views/paco/PacoWebinar.vue` | aucune | onglets des dix séances ; séance terminée : présentation et rediffusion YouTube ; séance à venir : présentation, compte à rebours, formulaire d'inscription, puis écran « Rejoindre » (lien Teams) |
| `/paco/join` | `views/paco/PacoJoinGateway.vue` | aucune | passerelle des anciens courriels : inscrit (localStorage) vers Teams, sinon vers `/paco` |
| `/paco/admin` | `views/paco/PacoAdmin.vue` | `requiresAuth`, `requiresRole: ['paco', 'admin', 'super_admin']` | statistiques, courbe des inscriptions, répartition par canal d'acquisition, liste paginée (100), recherche, filtre par séance et par type, export CSV, suppression, marquage « rattrapé », rappel par courriel, toast en temps réel |

**Inscription sans compte** : prénom, nom, adresse, genre, tranche d'âge, ville, pays, statut professionnel, organisation (masquée pour « sans emploi »), consentement à l'enregistrement (obligatoire), canal d'acquisition. Cascade qui ne bloque jamais la personne : RPC `register_paco_quick`, sinon RPC `register_paco_fallback`, sinon trace locale seule. Dans tous les cas, l'écran passe à « Rejoindre » et le navigateur retient l'inscription (`localStorage`).

**Identification ensuite** : uniquement le `localStorage` du navigateur (`paco_registered_session_<édition>`, `paco_registration_data_session_<édition>`). Une personne connectée à Supabase est aussi vérifiée en base, chemin mort en v2 (le public n'a plus de session Supabase).

**Même origine qu'avant** : la v1 était servie sur `epavillonclimatique.francophonie.org`, la v2 l'est aujourd'hui. Le `localStorage` des inscrits de la session 10 est donc toujours là ; **garder exactement les mêmes clés** les reconnaît sans réinscription.

### Fichiers v1 réellement utilisés

Chaîne vivante (ce que la vraie migration devra reprendre) :
- Vues : `PacoWebinar.vue` (150 l.), `PacoJoinGateway.vue` (51), `PacoAdmin.vue` (469).
- Composants publics (`components/paco/`) : `PacoSessionTabs` (240), `PacoSession1` (97, rediffusion), `PacoSession2` (71, à venir), `PacoPresentation` (219), `PacoBanner` (36), `PacoStatusBadge` (84), `PacoPanelistCard` (59), `PacoPartnerLogos` (34), `PacoShareButton` (142), `PacoCountdown` (117), `PacoQuickRegister` (385), `PacoJoinSection` (401).
- Composants d'administration : `PacoStatsCards` (183), `PacoRegistrationChart` (222, amCharts 5), `PacoReferralSourceChart` (178, amCharts 5), `PacoRegistrantTable` (439).
- Composables (`composables/paco/`) : `constants.js`, `usePacoWebinarData.js` (324, **toutes les séances en dur**), `referralSources.js`, `usePacoRegistration.js` (605, dont la moitié morte), `usePacoStats.js` (442), `usePacoCsvExport.js` (99).
- Hors dossier : `useSupabase.js`, `useCountries.js` (table `countries` de Supabase), `useAuth.js`, `useSEO.js`, `useEmailModal.js` et `EmailManagerModal.vue` + `SimpleEmailSender.vue` (1 520 l.) pour le rappel.
- Traductions : `locales/fr/paco.json` et `locales/en/paco.json` (780 l., 67 Ko chacun), plus `common.cancel` et `common.close`.
- Images : 31 fichiers de `public/images/` (bannières des séances, photos d'intervenants ; `paco_10.jpg` pour la prochaine) et 5 logos de `public/logos/logo_paco/`. Tous présents, 3,6 Mo au total environ.

Code mort (à laisser à la migration) : `PacoActivityRegister`, `PacoEmailCheck`, `PacoEmailVerification`, `PacoLoginForm`, `PacoRegisterForm` (aucun import), `usePacoEmail.js` (n'est importé que par un composant mort), et dans `usePacoRegistration.js` : `checkEmailExists`, `registerForPaco`, `insertDemographicData`, `finalizePacoRegistration`, `savePendingRegistration` et voisines. Sections i18n associées : `emailCheck`, `login`, `register`, `activityRegister`, `verifyEmail`, `success`. Fonction Edge `send-paco-email` jamais appelée. `PacoLoginForm` pourra servir de base à la connexion des administrateurs lors de la migration.

Ailleurs dans la v1, sans équivalent à recréer : carte « Prochain webinaire » de l'accueil (`UpcomingActivities.vue`), exclusion de l'événement PACO des listes (`EventsSection`, `UpcomingEventWidget`), rôle `paco` dans la gestion des rôles.

## 3. Dépendances Supabase

Client unique `createClient(VITE_SUPABASE_URL, VITE_SUPABASE_ANON_KEY)`.

| Appel | Objet | Qui | Existe en prod | Ce que permet la sécurité |
|---|---|---|---|---|
| `rpc('register_paco_quick', …13 paramètres)` | fonction, `SECURITY DEFINER` | visiteur | oui | `GRANT` à `anon` : écrit `activity_registrations` + `paco_demographic_data` hors RLS |
| `rpc('register_paco_fallback', …6)` | fonction, `SECURITY DEFINER` | visiteur | oui | `GRANT` à `anon` |
| `rpc('track_paco_join_click', p_user_id, p_email, p_session_edition)` | fonction, `SECURITY DEFINER` | visiteur | oui | `GRANT` à `anon`, premier clic seulement |
| `rpc('increment_activity_view_count', activity_uuid)` | fonction, `SECURITY DEFINER` | visiteur | oui | `GRANT` à `anon` (compteur de vues uniques) |
| `from('countries').select('*')` | table | visiteur | oui | politique « Countries are publicly viewable » |
| `from('activity_registrations')` select, count, delete, update `recovered_at` | table | admin | oui | SELECT, UPDATE, DELETE réservés à `paco`, `admin`, `super_admin` actifs (`user_roles`) ; un visiteur ne lit que ses propres lignes (`user_id = auth.uid()`), donc rien |
| jointure `paco_demographic_data`, `countries`, `users` | tables | admin | oui | `paco_demographic_data` : mêmes trois rôles ; `users` : lisible publiquement hors bloqués et suspendus |
| `from('activities').select('activites_view_count')` | table | admin | oui | l'activité PACO est `approved`, donc lisible publiquement |
| `channel('paco-registrations')` sur `activity_registrations` | Realtime | admin | **à vérifier** | la publication `supabase_realtime` n'est pas dans l'archive ; si la table n'y est pas, pas de toast, le reste marche |
| `functions.invoke('send-email')` (rappel groupé) | Edge | admin | oui | exige un `super_admin` pour un envoi groupé ; un `paco` seul reçoit 403 (copie ou CCI refusées) ; relaie vers le serveur de courriel de l'ancien hébergement |
| `auth.signInWithPassword` + lecture `user_roles` | Auth | admin | oui | `user_roles` lisible publiquement pour les lignes actives |

Identifiants en dur, aussi codés dans les fonctions SQL : événement `00000000-0000-4000-a000-00000000e001`, activité `00000000-0000-4000-a000-00000000a002`. Contraintes que le formulaire doit respecter : genre `male|female`, âge `under_35|over_35`, statut `employed|student|unemployed|entrepreneur`, consentement `true`, canal parmi six valeurs, précision ≤ 120 caractères.

Volumes au 27/10 : 14 241 inscriptions PACO (sessions 1 à 10), dont 219 de secours ; 3 comptes portent le rôle `paco`.

**Authentification** : le public n'en a pas besoin. L'administration exige une session Supabase **et** un rôle `paco`, `admin` ou `super_admin` actif et non expiré, lu dans `user_roles` (garde du routeur v1, et les politiques RLS refont le contrôle côté base).

## 4. Données en dur

- **Séances** (`usePacoWebinarData.js`) : dix entrées `{ edition, date, startTime, endTime, timezone: 'GMT', completed, coverImage, replayUrl, panelists, partners }`. Ordre d'affichage ≠ numéro d'édition (les éditions 5 et 6 sont les sessions 1 et 2, d'où `paco.tabs.session<édition>`). Toutes à 14:00–15:30, interprétées en UTC (`…Z`).

| Affichée | Édition | Date | Rediffusion |
|---|---|---|---|
| Session 1 | 5 | 29/01/2026 | YouTube |
| Session 2 | 6 | 26/02/2026 | YouTube |
| Session 3 | 1 | 26/03/2026 | YouTube |
| Session 4 | 2 | 30/04/2026 | YouTube |
| Session 5 | 3 | 28/05/2026 | YouTube |
| Session 6 | 4 | 25/06/2026 | YouTube |
| Session 7 | 7 | 30/07/2026 | YouTube |
| Session 8 | 8 | 27/08/2026 | YouTube |
| Session 9 | 9 | 24/09/2026 | YouTube |
| **Session 10** | **10** | **29/10/2026** | à venir |

- **Lien Teams** : une seule constante `PACO_TEAMS_LINK` (`constants.js`) pour toutes les séances, commentée « À configurer par l'organisateur ». À faire confirmer pour le 29/10.
- **Adresses** : `https://epavillonclimatique.francophonie.org/paco` (partage, SEO, corps du rappel), `/paco/join` (anciennes confirmations).
- **Courriels** : rappel adressé à `epavillonclimatique@francophonie.org`, copie à une adresse personnelle du développeur v1, inscrits en CCI ; aide à l'inscription vers une adresse de l'IFDD avec la même copie. Texte du rappel en français dans `PacoAdmin.vue` (installer Teams, venir 30 min avant).
- **Textes** : titres, contextes, dates et heures affichées (`dateLabel`, `timeLabel`) dans les traductions, une section par séance. Date réelle et libellé sont donc écrits deux fois : à garder d'accord.
- **Couleurs** du graphique des canaux, en hexadécimal (`referralSources.js`).

## 5. Variables d'environnement (noms seulement)

Le build v1 lit : `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY` (indispensables), `VITE_APP_URL` (liens des courriels, repli sur l'origine), et pour le seul assistant IA : `VITE_ANTHROPIC_API_KEY`, `VITE_OPENAI_API_KEY`, `VITE_OPENROUTER_API_KEY`, `VITE_OPENROUTER_CHAT_MODEL`, `VITE_USE_OPENROUTER`, `VITE_AI_CHAT_MAX_TOKENS`, `VITE_AI_CHAT_TEMPERATURE`, `VITE_AI_CHUNK_SIZE`, `VITE_AI_CHUNK_OVERLAP`.

**Danger** : tout `VITE_*` est écrit en clair dans le JavaScript servi. Le `.env.local` de la v1 porte des clés d'IA, et le `dist/` local du 26/10 en contient une en clair dans un fichier `assets/*.js`. **Ne jamais déposer ce `dist/` tel quel** ; reconstruire avec les seules variables Supabase et `VITE_APP_URL`, les clés d'IA vides. Ces clés ont été publiques tant que la v1 était en ligne : à révoquer chez leurs fournisseurs.

Fonctions Edge (secrets Supabase, inchangés) : `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY`, `LARAVEL_POLIVALENT_EMAIL_URL`, `SUPABASE_CUSTOM_AUTH_LARAVEL_KEY`, `PACO_TEAMS_LINK` (fonction morte).

## 6. Consigne : rien ne change

Précision du commanditaire (27/10) : ni l'apparence (la v1 à l'identique, pas de reprise au style de la v2), ni les données (Supabase tel quel). Le bloc est posé brut ; sa vraie migration viendra plus tard. Deux voies.

### Voie A : servir l'application v1 elle-même, par Apache

La v1 est une SPA Vite. Sa page `/paco` n'est pas une page isolée : elle se rend **dans la coquille de la v1** (`App.vue` : barre de navigation, pied de page, barre latérale, fenêtre de courriel groupé, alerte de mise à jour). Servir le build v1 est la seule façon d'avoir exactement cela.

**Ce que le build demande.** Base Vite par défaut (`/`) ; routeur `createWebHistory(import.meta.env.BASE_URL)`, donc `/`. Les chemins de la v1 sont ceux du domaine : `/paco` reste `/paco`, sans rien changer à la configuration. Changer la base (`--base=/paco-v1/`) ne servirait à rien : le routeur passerait sous `/paco-v1/paco`, et les images de PACO sont des chaînes `'/images/…'` écrites dans le JavaScript, que Vite ne réécrit pas.

**Ce que le build sert, et les collisions avec Nuxt.** Chemins à la racine utilisés par la page et sa coquille :

| Chemin v1 | Usage | Pris par la v2 ? |
|---|---|---|
| `/assets/*` | JS et CSS hachés | non (Nuxt sert `/_nuxt/`) |
| `/images/*` | bannières et intervenants PACO | non |
| `/logos/logo_paco/*`, `/logos/<fichier>` (logos de la barre) | logos | `/logos/` existe en v2, mais seulement en sous-dossiers `png/` et `svg/` : aucun fichier commun |
| `/fonts/*` | police Maverick (`fonts.css`) | non |
| `/favicon.ico`, `/logo-ifdd.png`, `/version.json` | icône, partage, sondage de mise à jour toutes les N minutes | non (la v2 a `/favicon.svg`) |
| `/paco`, `/paco/join`, `/paco/admin` | entrées | non (404 aujourd'hui) |
| `/login`, `/403` | connexion des administrateurs PACO, refus | non : la v2 sert `/connexion` et `/en/login`, jamais `/login` sans préfixe |
| `/forgot-password`, `/reset-password` | mot de passe oublié v1 (lien Supabase vers `…/reset-password`) | non : la v2 sert `/mot-de-passe-oublie`, `/nouveau-mot-de-passe` et leurs versions `/en/…` |

Aucune collision de fichier. Seule réserve : ces chemins deviennent ceux de la v1 tant que PACO y vit ; les redirections des anciennes adresses v1 (D11) devront les éviter.

**Configuration Apache** (vhost 443 de `epavillonclimatique.francophonie.org`, `ops/apache-epavillon.conf`, avant `ProxyPass /`, même procédé que l'exclusion `/v2` déjà en place) :

```apache
# PACO : l'application v1 telle quelle, le temps de sa vraie migration (D9).
ProxyPassMatch "^/(paco(/.*)?|login|403|forgot-password|reset-password)$" !
ProxyPassMatch "^/(assets|images|fonts|logos/logo_paco)/" !
ProxyPassMatch "^/(logos/[^/]+|favicon\.ico|logo-ifdd\.png|version\.json)$" !
AliasMatch "^/(paco(/.*)?|login|403|forgot-password|reset-password)$" /var/www/paco-v1/index.html
AliasMatch "^/((assets|images|fonts|logos)/.*|favicon\.ico|logo-ifdd\.png|version\.json)$" /var/www/paco-v1/$1
<Directory "/var/www/paco-v1">
    Require all granted
    Options -Indexes
    AllowOverride None
    <FilesMatch "\.html$">
        Header set Cache-Control "no-cache, must-revalidate"
    </FilesMatch>
</Directory>
```

`AllowOverride None` neutralise le `.htaccess` copié de `public/` (il renverrait tout chemin inconnu vers la v1). `/logos/[^/]+` ne prend que les fichiers au premier niveau : `/logos/png/…` et `/logos/svg/…` continuent vers Nuxt. Dépôt : `apachectl configtest && systemctl reload httpd` (comme `./deploy.sh vhost`). Les vidéos, `events/` et le reste du `dist/` (86 Mo) n'ont pas à être copiés.

**Ce qui arrive quand PACO mène ailleurs dans la v1.** Le routeur v1 navigue sans recharger : un clic sur le logo, un lien de la barre ou du pied de page (accueil, événements, programmations…) **affiche la page v1 correspondante**, lue sur Supabase, formulaires compris. Un site fantôme de la v1, accessible depuis `/paco`. Un rechargement sur cette adresse tombe ensuite sur Nuxt (404 ou page v2). Deux réponses :
- **recommandé** : sur une branche de build `paco-isole` du dépôt v1, une garde de quelques lignes dans `src/router/index.js` : toute route hors `paco*`, `login`, `403`, `forgot-password`, `reset-password` part en navigation complète (`window.location.assign(to.fullPath)`), donc chez la v2. Invisible à l'écran : rien ne change de l'apparence ni des données, seule la sortie du bloc ;
- strictement tel quel, sans garde : accepté, mais la v1 entière reste joignable par clics et écrit encore dans Supabase.

**Connexion des administrateurs PACO.** `/paco/admin` exige une **session Supabase** (`requiresAuth`) et un rôle actif `paco`, `admin` ou `super_admin` dans `user_roles`. Sans session, la garde envoie vers `/login?redirect=/paco/admin`, page de connexion v1 rendue dans la SPA (et servie aussi au rechargement grâce à l'`AliasMatch`), puis retour à `/paco/admin`. Comptes et mots de passe : ceux de **Supabase**, pas ceux de la v2. La session est gardée dans le `localStorage` du domaine : un administrateur déjà connecté à la v1 avant le 26/10 l'est peut-être encore. Le bouton « Notifier la page » (fenêtre de courriel de la coquille) marche à l'identique, pour un `super_admin` seulement, comme en v1. La connexion Google ou Microsoft (`/auth/callback`) n'est pas couverte : mot de passe seulement.

**Build.** Depuis le dépôt v1, à partir de l'état servi en production le 26/10 : l'arbre de travail a 13 fichiers modifiés non commités (fenêtre de courriel, `useOrganizationForm`), présents dans le `dist/` du 26/10. Les committer sur `paco-isole` avant de construire, pour savoir ce qui tourne. `npm ci && npm run build` avec un fichier d'environnement réduit à `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`, `VITE_APP_URL` ; vérifier qu'aucune clé d'IA ne reste dans `dist/assets/`. Copier `index.html`, `assets/`, `images/`, `fonts/`, `logos/`, `favicon.ico`, `logo-ifdd.png`, `version.json` dans `/var/www/paco-v1/` sur le serveur.

**Effort** : deux à trois heures, essais compris. Faisable aujourd'hui, sans toucher au code de la v2.

### Voie B : recopier les fichiers Vue dans Nuxt

« Rien ne change » n'y est **pas tenable** ; ce qui bougerait malgré tout :
- **La coquille** : la page v1 vit sous la barre de navigation, le pied de page et la barre latérale de la v1. Dans Nuxt, elle prendrait la mise en page de la v2 (autre barre, autre pied) ou une mise en page nue. Recopier la coquille, c'est recopier `AppNavBar`, `FooterSection`, `SideBar` et leurs stores (authentification, rôles, notifications, recherche) : une bonne part de la v1.
- **Styles globaux** : les deux sont en Tailwind 4 (pas d'écart v3/v4), mais la v2 redéfinit `--font-sans`, `--font-display` et `--shadow-lg`, et charge ses jetons et sa base (`main.css`, `design-tokens.css`). Police, ombres et fond changent sous les mêmes classes. La v1 apporte sa police Maverick et ses utilitaires `ifdd-*` (`main.css` v1), absents de la v2.
- **Mode sombre** : la v1 suit une classe `.dark` posée par son bouton ; la v2 suit `data-theme` et la préférence du système. Les classes `dark:` de l'administration réagiraient autrement.
- **Langue** : vue-i18n direct et choix gardé par la v1, contre `@nuxtjs/i18n` avec préfixe d'adresse (`/en/paco`) et cookie de la v2. Le sélecteur de langue de la barre v1 disparaît.
- **Composants partagés** : Font Awesome enregistré globalement, amCharts, `useAuth`, la garde du routeur, et pour « Notifier la page » la fenêtre de courriel de la v1 (`EmailManagerModal`, `SimpleEmailSender` : 1 680 lignes, stores `emailModal` et `auth`).
- **Rendu** : SSR de Nuxt à couper pour `localStorage` et la session Supabase ; hydratation et middlewares globaux de la v2 (dont `maintenance`, qui fermerait `/paco`).
- **Effort** : deux jours, pour une copie approchée et non identique, dans un code qui enfreint toutes les conventions de la v2 (appels hors `useApi()`, JavaScript, couleurs en dur, données en dur).

Le détail fichier par fichier de cette voie, utile pour la vraie migration, est au § 2 (chaîne vivante et code mort).

### Recommandation : voie A

C'est la seule qui respecte « rien ne change » à la lettre (le même code, le même build, la même base), elle se fait en quelques heures avant jeudi, elle ne touche pas au code de la v2, et elle se retire en supprimant cinq lignes Apache et un dossier. Avec la garde de sortie sur la branche `paco-isole`, pour que la v1 entière ne revive pas derrière `/paco`.

## 7. Points de vigilance

- **Délai** : séance le 29/10 à 14 h 00 GMT, bouton « Rejoindre » ouvert à 13 h 00. Mise en ligne le 28/10 au plus tard.
- **Lien Teams** : constante unique « À configurer », à faire confirmer pour la session 10.
- **Après la séance** : passer la session 10 en `completed: true` avec sa `replayUrl`, ajouter la session 11 (données et traductions), reconstruire, recopier. Comme en v1.
- **Clés** : build sans clé d'IA ; révoquer celles déjà publiées par la v1. Supabase garde ses clés (D9).
- **Supabase doit rester actif** : en offre gratuite, un projet se met en pause après une semaine sans activité. Vérifier l'offre.
- **Courriel de rappel** : relayé par le serveur de courriel de l'ancien hébergement (`LARAVEL_POLIVALENT_EMAIL_URL`) ; sa résiliation le coupe, comme les courriels de la v2.
- **Realtime** : vérifier que `activity_registrations` est publiée, sinon pas d'alerte en direct dans l'administration.
- **Courriel personnel en copie** des rappels et de l'aide à l'inscription : à faire trancher.
- **Bogue v1 conservé** : `/paco/join` appelle la vérification locale sans numéro de séance et renvoie donc toujours vers `/paco`. Sans gravité (la page `/paco` reconnaît l'inscrit) ; tel quel par consigne.
- **Mode maintenance de la v2** : sans effet en voie A (Apache sert PACO avant Nuxt).

## 8. À tester (voie A)

1. `/paco`, `/paco/join`, `/paco/admin`, `/login` : rendus par la v1, au rechargement comme par clic.
2. `/`, `/connexion`, `/logos/png/…`, `/_nuxt/…`, `/api/…` : toujours la v2.
3. Session 10 ouverte par défaut, compte à rebours, pays chargés, images et logos présents, police Maverick.
4. Inscription d'essai (adresse de test) : lignes dans `activity_registrations` (`session_edition = 10`) et `paco_demographic_data`, écran « Rejoindre » ; supprimée ensuite depuis l'administration.
5. Navigateur d'un inscrit avant le 26/10 : reconnu sans réinscription (même origine, mêmes clés `localStorage`).
6. « Rejoindre » plus d'une heure avant : fenêtre « pas encore commencé ».
7. Un clic sur un lien de la barre v1 : avec la garde, la v2 se charge ; sans elle, noter ce qui s'affiche.
8. Administration avec un compte `paco` : statistiques, liste, CSV, filtres ; « Notifier la page » refusé, accepté pour un `super_admin`.
9. `grep` des préfixes de clés d'IA dans `/var/www/paco-v1/assets/` : aucun résultat.
10. Téléphone 375 px, thèmes clair et sombre, français et anglais.
