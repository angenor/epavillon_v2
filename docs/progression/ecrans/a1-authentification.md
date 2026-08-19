# A1 — Authentification

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 17/08. 5 pages, 6 composants, 1 layout, 1 store, 1 composable, 2 middlewares, 12 fichiers de traduction (6 × 2 locales). Un seul emplacement réservé sans implémentation : le second facteur (`accounts.mfa_enabled_at`, un compte simulé le déclenche). La connexion fédérée a été écrite puis **retirée** le 17/08 sur décision du commanditaire. Trois obligations d'API relevées (n° 18 à 20)

---

## Écarts relevés en écrivant l'authentification (A1, 17/08)

Trois points, tous de la même famille : **le modèle porte les colonnes, mais pas les règles qui les remplissent**. Ce n'est pas un défaut — une durée de validité ou un seuil de verrouillage sont des réglages d'exploitation, pas des invariants de données ; les inscrire en base les rendrait modifiables par une migration seulement. Ils deviennent donc des exigences du prompt **B1**, écrites ici pour ne pas être redécouvertes.

**Aucune modification du SQL n'a été nécessaire pour cet écran** — `030_identity.sql` prévoyait tout : la séparation personne / compte, le verrouillage, le second facteur, les jetons à usage unique et leurs cinq finalités.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **18** | **Le verrouillage a ses colonnes, pas son seuil.** `accounts.failed_attempts` et `locked_until` existent, mais rien ne dit au bout de combien d'échecs on verrouille, ni pour combien de temps, ni qui remet le compteur à zéro | `030` § 2 | L'écran affiche « réessayez dans quelques minutes » sans pouvoir donner de durée : il ne la connaît pas. Les mocks retiennent 5 échecs et 12 minutes, valeurs inventées | **Reporté au prompt B1.** Seuil et durée dans un réglage d'exploitation, pas dans le code ; la réponse `locked` peut alors porter `until`, que l'écran sait déjà rendre |
| **19** | **La durée de validité d'un jeton n'est écrite nulle part.** `one_time_tokens.expires_at` est `NOT NULL` et sans valeur par défaut : chaque appelant décide | `030` § 2 | Deux liens de finalités différentes pourraient vivre des durées différentes sans que personne l'ait décidé. Les mocks retiennent 48 h, sans plus de fondement | **Reporté au prompt B1** : une durée par finalité (`token_purpose`), déclarée au même endroit. La vérification d'adresse et la réinitialisation n'appellent pas la même : l'une se suit dans la journée, l'autre dans l'heure |
| **20** | **Rien n'exige qu'une adresse soit vérifiée pour se connecter.** `people.email_verified_at` est nullable et `person_status` ne connaît pas d'état « en attente de vérification » — un compte non vérifié est un compte `active` comme un autre | `030` § 1 | L'interface a tranché seule : mot de passe correct **et** `email_verified_at IS NULL` → connexion refusée, avec proposition de renvoyer le lien. C'est défendable, mais ce n'est écrit dans aucun fichier du modèle | **Reporté au prompt B1**, à confirmer : la règle appartient à l'API, qui seule peut la tenir. Ne PAS ajouter un statut de personne pour cela — l'état est déjà porté par la date, et deux sources pour un même fait divergent toujours |

---

## Ce qui a été vérifié le 17/08 sur l'authentification, et comment

Les parcours d'authentification ne se prouvent pas au rendu statique : ils se jouent au clic. Tout ce qui suit a été exercé **dans un navigateur réel**, sur le serveur de développement.

| Contrôle | Résultat |
|---|---|
| Les six issues de connexion, une par une | **Toutes rendues.** Mot de passe faux → « Connexion impossible » ; **adresse inexistante → le MÊME message, au caractère près** ; compte verrouillé (Ouédraogo) → « Compte temporairement verrouillé » ; second facteur (Perret) → écran « Validation en deux étapes » ; personne suspendue (Lambert) → « Compte suspendu » ; adresse non vérifiée (Traoré) → alerte et bouton de renvoi. Connexion réussie (Bakayoko) → redirection vers l'accueil |
| Les trois refus de jeton, pour les deux usages | Vérification d'adresse : valide → « Adresse vérifiée », périmé → « Ce lien a expiré », déjà utilisé → « Cette adresse a déjà été vérifiée ». Réinitialisation : les trois mêmes, **plus un jeton inventé** (`?token=jeton-inconnu`) → « Ce lien n'est pas valide » |
| Inscription de bout en bout | Formulaire rempli, compte créé, redirection vers la vérification, **« Un lien de vérification a été envoyé à awa.diallo@example.org »** affiché avec l'adresse saisie |
| **Le rebours de 60 secondes** | Bouton désactivé et libellé décomptant (« Renvoyer le lien (46 s) ») ; **repris 62 s plus tard : libellé « Envoyer de nouveau », `disabled = false`**. Le rebours se termine réellement, il n'est pas décoratif |
| Réinitialisation complète | Jeton valide → formulaire ; mot de passe de 10 signes refusé (« au moins 12 caractères ») ; mot de passe long accepté → « Mot de passe modifié » |
| Réponse invariable du mot de passe oublié | Adresse inconnue → « **Si** un compte existe pour personne.inconnue@example.org… ». Aucune autre formulation possible : l'API n'a pas d'autre issue à rendre |
| État « déjà connecté » | `/connexion` en étant connecté → « Vous êtes déjà connecté », **le compte est nommé** (Aminata Bakayoko et son adresse) ; la déconnexion ramène au formulaire |
| **Exigences et robustesse du mot de passe** — reprises le 17/08 après l'arbitrage du commanditaire | Les quatre conditions sont affichées **avant toute saisie** (« 8 caractères au moins · Une majuscule · Une minuscule · Un caractère spécial (facultatif) »). `abcdefgh` → « Conditions non remplies » ; `Abcdefgh` → conforme mais **« Faible »** (suite de lettres) ; `Azerty12` → conforme et **« Faible »** (mot trop courant) ; `Belem2027` → « Moyen » ; `Cop31-Belem-Pavillon` → « Solide ». À l'envoi : `sansmajuscule` refusé avec « au moins 8 caractères, dont une majuscule et une minuscule » ; `Kayes2027` accepté |
| Fédération d'identité | **Retirée des écrans le 17/08** — plus aucune trace dans le HTML rendu ni dans les fichiers de traduction. Seul l'ENUM de la base la prévoit encore |
| Tri des pays dans la langue affichée | fr : Belgique, **Bénin**, Brésil, Burkina Faso… — l'accent est rangé là où on le cherche. en : Belgium, Benin, Brazil… La liste est triée à l'affichage, pas dans la base |
| Les cinq écrans en français et en anglais | 10 URL, **200** partout, et **zéro clé brute** — balayage du HTML rendu sur `auth.*`, `auth-form.*`, `common.*`, `validation.*` |
| Les URL sont bien celles des deux langues | `/connexion`, `/inscription`, `/verification-adresse`, `/mot-de-passe-oublie`, `/nouveau-mot-de-passe` et leurs équivalents `/en/…` ; **`/auth/login` répond 404** — le chemin de fichier n'est pas exposé |
| 375 px, thème sombre | **Aucun défilement horizontal** (`scrollWidth == clientWidth == 375`), fond `#231F20` — le noir de charte —, **logo blanc effectivement servi** (`ifdd-horizontal-blanc.svg` visible, la variante grise masquée) |
| États des champs et accessibilité | `autocomplete="username"` et `current-password` sur la connexion, `new-password` ailleurs ; `role="meter"` avec `aria-valuetext` sur l'indicateur ; messages d'erreur en `role="alert"` ; le champ « compte concerné » de la réinitialisation est en **lecture seule et focalisable**, et porte le `username` que les gestionnaires de mots de passe attendent |
| `make check-front` | Vert — `nuxt typecheck` à 0 erreur, construction complète |
| Aucun fichier de code applicatif > 1000 lignes | Le plus long des fichiers créés est `app/pages/auth/verify-email.vue` (252 lignes) |

**Un défaut de composant trouvé en chemin, et corrigé.** Le champ en lecture seule de l'écran de réinitialisation affichait « Compte concerné **facultatif** » : `UiFormField` marque l'obligation, et affiche « facultatif » partout ailleurs — y compris là où il n'y a rien à décider. La mention est désormais retirée sur un champ `readonly`.
