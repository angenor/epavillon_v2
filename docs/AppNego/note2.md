je dois maintenant construire l'application du guide du négociateur.
L'ojectif principale des de mettre à disposition des négociatrics.teurs les documents utils(guides négociation, résumé des guides négociation, notes technique et bien d'autres documents). On doit pouvoir charger un document de sorte à pouvoir lire hors connexion et aussi ajouter d'autre au favorie.
Comme plus, on pourrait rendre accessible aussi les programmations des activités au sein du pavillon de la francophonie en cours et passé et la possibilité de s'y inscrire et de consulter tous les détailles.
une chose très importante aussi: mettre en place un FAQ qui sera très utile pour les nouvelles négociatrices et meme les anciennes vu que ca sera alimenté en continue.
Les négociatrices suivent des thématiques. elles peuvent suivent 1 ou plusieurs thématiques.
Actuellement nous avons un groupe whatsApp pour les négociatrices. C'est bon pour échanger mais on ne peut pas customiser à notre gout, on aimerait donc pouvoir creer un outils propre. celà nous permettra par exemple de regrouper des document partagé dans le groupe et de les vectoriser pour une meilleur recherche.
Je veux aussi y intégrer un chatbot qui se basera sur les documents qu'on aura sélectionné pour repondre à certaines questions des négociatrices ou meme sur la transcription de certains session de formation. pour chaque reponse, l'IA pourra donner le nom/lien, page, lien de la video et l'interval de minute concernée. Il peut arrivé que certaines références des ducuments et meme des vidéos de formations soient obsolet ou faux, il va falloire trouver un mécanisme pour gerer ca. les documents négos et enregistrement vidéo seront des source mais on pourra en déninir d'autres.
Les documents et formations peuvent concerner 0 ou plusieurs thématiques.
L'admin doit pouvoir générer un quiz basé sur des documents, des enregistrements pour que les négociatrice puisse s'exercer à le faire. une négociatrice aussi peu regerer un autre quiz(dans ce cas, ca sera reformulations des questions et/ou quiz basé sur d'autres éléments non pris en compte par les quiz existants). Bien sur la génération de quiz sera basé sur un agen IA qui le fera.
je viens d'échanger avec une négociatrice et je lui ai posé cette question:
- Moi: Je voulais savoir, c'est quoi la première difficulté pour une négociatrice quand elle se rend à la COP
- Sa reponse:
```
--Non connaisse des programmes par certaines, surtout les nouvelle négociatrice. ;
--L’Anglais;
--Faible encadrement  de la délégation au niveau de certains points focaux de l’a CCNUCC etc
--financement pour la participation des points focaux genre et Changement climatique
--parfois elle doivent faire des restitution chaque jours(ou tous les 2 jours) auprès de leur groupe pays/ministère de l'environnement.(je me dit qu'elle pourrait partager aussi leur restitution dans l'app pour alimenter ou aider d'autres personne. avec les recoupement, un agent IA pourrait en deduire des chose interressants)
--une nouvelle negociatrice doit se rapprocher du lead de son groupe et participer aux activité du groupe.
```

il me semble que pour le programme on peut s'incrire quelque part et le recevoir par email ainsi que les modifs. On pourrait faire ensorte d'inscrire l'email du epavillon de sorte que les document recus doit automatiqment rengé par un agent IA ainsi que le programme sur la plateforme pours les sessions de négociation, tu me dira. De plus pendant les négociations, plusieurs documents important peuvent passer, ca serait bien les capter pour en faire des quiz en francais. Les document partagé dans le groupe pourrait etre capté aussi. Mais il ne faut pas tout capter non plus par ne pas trop parasiter avec de fausse info les reponse de l'IA. C'est pourquoi le chois des doc et transcription pour les reponse sera manuel ou je sais pas trop, tu me diras.

Les négociatrice pourrons aussi contribuer à la mise à jours du programme: exemple: une personne se rend compte qu'une réunion est annulé, elle le signale dans l'application et l'admin est notifié. L'admin seule valide parce que c,est une action critique qui peut induire en erreur tout un continent. 

je veux qu'on utilise la meme base de données que le epavillon, en effet, il y aura plusieurs point commun avec le epavillon. exemple: document negos, session de négociations, réunion de la francophonie.
Mais le coté IA et vectorisation, embed de document et conversation devra etre gerer par un microservice fastAPI/Python

Pour les compte utilisateurs, un utilisateur du epavillon doit etre distingué à l'authentification à un utilisateur de l'APP mobile meme si toutes ses infos, historique etc sont rataché: dit moi si c'est une bonne idée.

pour l'app mobile, on peut peux-etre creer une dossier @frontend/app/pages/app_nego/ avec ses composant dans @frontend/app/components/app_nego/ , developpé pour etre porté plus tard en capacitor.


on doit developper module par ```module qui met à disposition les documents```, ensuite, on attaque le module pour ```afficheer les programmes des 'Sessions de négociation' , le module pour afficher le programme des 'réunions de la francophonie'(certaine réunions de la francophonie pourront etre certaines activité de la francophonie ou pas)```, ensuite, on attaque le module pour le chat(groupe de conversation, conversation privé avec les experts ou avec une negociatrice, un admin), 


Je participe chaque années à la cop donc je serai aussi sur terrain en personne . Mais je quitte rarement le stand de l'OIF parce que je fais parti de l'équipe technique.

Pour les quiz généré par l'IA, ca sera des QCM/QCD, l'IA generera les question et bien sur leur reponse. Mais comme ca reste une IA, elle peut alluciner donc sont quiz pourrait etre examiné et valiser par 1 ou plusieur expert ou un amin avant d'etre mise à la disponisition des négociatrice. Je trouve que le quiz est une bonne idée parce que celà constitue une sorte de résumé, pour un document ou une vidéo.




