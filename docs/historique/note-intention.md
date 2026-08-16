Nous somme actuellement sur un serveur apach et allons migrer sur un VPS ce qui nous donne de grandes possibilité de stack.

# Module Cop/evement
C'est le module le plus important.
Chaque années l'Orgnisation internationnal de la francophonie à travers son organe subsidiaire IFDD organise, participe à certains grands évènements.
- Cop climat: généralement en Novembre, l'organisatiion international de la francophonie met en place un stand pour accueillir les propositions de ses etats menbres, c'est un lieu de partage entres organisations francophone et entre francophone et le monde entier
- Il y'a aussi Cop biodiversité
- Cop désertification
Contrairement à la Cop climat, les autres cop n'ont pas forcément droit à un stand puis défilé d'organisation. Parfois oui et parfois non. Quand ya pas de stand on ne fait pas d'appel à proposition. L'IFDD envoie juste son representant dans le pays généralement pour participer aux session.
- Il ya aussi des évenements spontané(activité unique organisé par l'IFDD zoom ou teams) et parfois périodique(multi activités avec meme theme ou theme différent organisé par l'IFDD zoom ou teams) qui n'ont rien à avoir les cops, on peut les mettre dans la section autre de la page programmation.
- Les activités sont organisé sur zoom ou team, il arrives qu'on embed le direct de la session youtube(ou autre service) sur la plateforme(pour les cop climat c'est obligatoire)

Le workflow simplifé se passe ainsi:
- l'OIF/IFDD lance un appel à proposition de projet pour un evenement(cop30 climat etc)(ca suppose qu'on a créé l'evenement sur la plateforme),
- une organisations, pays, ong etc soumettent leurs propositions(activitées)
- l'IFDD fait le trie et sélectionne les projets interressants et règle les conflit de chevauchement des crénaux(vue-cal)
- les projets soumis sont visualisé dans le backoffice, L'équipe de l'IFDD ayant au moins le role de révisionniste peut alors
- l'organisation est notifié de la rétension ou non de son projet(il peut aussi faire le suivis en temps reel dans son espace peersonnel)
- lorqu'un projet est retenu, il est programmée pour etres affiché publiquement sur notre plateforme


# Espace Admin
- Tableau de bord
affiche le nombre total d'utilisateur, inscription par jour etc
- Liste des activitée
states: notre de soumission par jour, activités approuvé, rejeté etc
Cette rubrique affiche toutes les activitées soumis par les organisation, pays, ong etc. Clique sur une activité ramène sur une page de détail contenant toutes les informations sur l'activité.
D'ailleurs, c'est sur ces pages Que les révisionnistes pourrons noter les activités, voir les notes attribué par les autres révisionniste, la moyenne des notes, commenter l'activité. Il s'agit d'une page assez importante parce que c'est là qu'on analyse le projet, décide de le retenir ou pas.
- Liste des utilisateurs
Affiche la liste des utilisateurs et toutes leur infos, role, organisation, activité soumis
- Liste des organisations
Liste des organisations, leurs activités, leur membres sur la plateforme, nombre d'activités validé, ratio
- Liste des évènements(Cop, etc)
exemple, Cop30 climat, Cop25 Biodiversité
- Role
- Emails
- Messages d'Incidents
Il arrive qu'en plein live, ou avant le live il arrive des incident. Problème technique, retard, annulation, il faut donc afficher un message pour informer les téléspectateurs.
- Gestion des Directs YouTube
lorsqu'une activité est programmée dans le backoffice, on lui cree automatiquement un lien zoom via l'api de zoom pour ses panéliste et l'audience à distence.
Ensuite pendant l'activité, on fait du liveStreaming sur YouTube, on recupere ainsi l'ID pour embed sur la plateforme
- Gestion des newsletters(hors périmetre pour l'instant)
- Outils(les doivent etre developper de facons modulaire, il peut nous arriver dans les phases à venir de les déployer sous un autre lien à part)
Outils de sondage(création de sondage indépendant ou après une session zoom ou physique)
Outils divers pour automatiser certaine taches de l'equipe, en réflexion
Agent IA pour creer des réunion et plein de choses que nous définirons(en réflexion)
plusieurs autres outils seront intégré au fur et à mésur



# Espace programmation

On voit les programmations de l'évènement l'année en cours avec possibilité de swicher pour voir pour les autres années.
Chaque evenement contient un emble d'activité et ce sont les activités soumis par les organisations et validée par l'équipe de l'IFDD/OIF.
Pour l'affichage des activités validées, il existe une vue gride et une vue calendrier(avec vue-cal, couleur en fonction de l'état: pas encore commencé, terminé, annulé, etc)
lorsqu'on clique sur un évènements, on voit sa page de détails, on peut s'y inscrire. Après inscription(ca apparait dans notre google agenda et équivalent apple(phase 2+)), selon ce qu'à programé l'admin pour l'évènement dont c'est l'activité, on peut recevoir des emails de rappel(2/1 jours, 1h, 30m cumulé) avant l'activité en question.
les presonnes inscrites sont inscrit aussi à zoom à travers l'API


# Espace Publications
- Les organisations peuvent publier des articles. Vu que nous avont un espace de stockage assez limité, il seront autorisé à publier à une certaines organisations ne seront autorisé à publier qu'une fois  par semaine, d'Autres plusieurs fois par semaine, par mois par jours. bref pour chaque organisation, il faudra pourvoir le nombre/fréquence de publication dans dans le backOffice

# Espace Négociations
C'est un espace où on publie:
- Sessions de négociation
- Documents d'aide(lien ou fichier uploadé directement)
- Réunions Francophonie(session avec lien zoom)
- groupes d'echange temps reels comme whatsApp, Groupe par thématique et parfois par promotion, bref, on va creer dynamiquement.
- Outils pour aiders les négociateurs: agent IA + RAG pour repondre aux questions selon les document(en réflexion)
cet espace est réservé aux personnes ayant le role négociateurs
plusieurs autres outils seront intégré au fur et à mésur


Intégration de google agenda(phase 2+)


# Nouvelle stack
- Nuxt(pour le référencement SEO)
- Ruxt Actix web(pour la performaance)
- PostgresSQL
- Garage(API S3), en local d'abord et migration amazon dans les phase future

L'ancienne version contenais beaucous de défaut par exemple, 2 personnes qui ont crée deux fois la meme organisations et on voit les 2 organisation sur la plateforme sans possibilité de fusionner. Bien sur nous avons mis un mecanisme de recherche d'organisation avant creation lorsqu'il n'existe pas mais certains recherchaient par nom complet tandis que d'autres par cygle et ca creaient des doublons.

