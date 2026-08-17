`agent-browser --headed`
`claude --dangerously-skip-permissions`

- lorsque la photo d'un intervenant est enregistré, il faut afficher au moins un apercu
- s'il ya un problème dans un onglet, il ne faut pas permettre de continuer, il faut désactiver le bouton suivant.
- un intervenant n'est pas obligé d'avoir un compte certe, il serait intérressant aussi de mettre en place un mécanisme pour limiter les doublons. Par exemple, en saisissant l'email le systhème peu faire une recherche pour voir si un tel intervenant ou utilisateur n'existe pas déjà dans la BD, dans ce ca, on peu le sélectionner directement, s'il a un compte et déjà renseigné son organisation, le système peu autodétecter ces infos et remplir les champs correspondant. Si le soumissionnaire sélectionne un profil déjà existant, il ne peu modifier aucune des infos de ce profil. Seul le profil(le user en question), s'il a un compte peu le modifier ou le super admin. Si c'est lui qui a créé l'intervenant(pas sélectionné un existant) alors il peu modifier les infos de ce intervenant si celui-ci n'a pas de compte sur la plateforme mais une fois l'activité validé, il ne peu plus modifier le compte d'un intervenant. N'empeche qu'il peu supprimer l'intervenant de son activité
- fonction et organisation doivent etre obligatoire ainsi que civilité
- Créneau souhaité et durée sont obligatoire
- le temps habituel c'est 60min. 2h30 max, 45min minimum. intervall d'heure admissible: 09h à 17h max(heure de fin)
- etape document à masquer pour l'intant, on aura pas le temps de lire les documents