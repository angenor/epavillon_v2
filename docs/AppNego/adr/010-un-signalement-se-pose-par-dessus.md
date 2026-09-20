# ADR-010 — Un signalement se pose par-dessus, une fois validé

**Statut** : accepté — 18/09/2026

## Contexte

Une négociatrice constate en salle qu'une réunion est annulée, déplacée, ou qu'un aparté se tient sans avoir été annoncé. Les réunions non annoncées ne figurent dans aucun programme : le réseau est la seule source. Mais une fausse information peut tromper tout un continent.

## Décision

Toute négociatrice peut signaler. **Seul un administrateur valide**, en un geste depuis son téléphone. Le signalement validé s'affiche dans un encart distinct, **par-dessus** la donnée officielle, sans jamais la modifier.

## Conséquences

- Rien ne s'affiche avant validation ; l'administrateur voit, en validant, ce que dit la source officielle à cet instant.
- Si la source officielle rattrape le signalement, l'encart devient inutile et se retire.
- Une réunion non annoncée est un objet à part, d'origine « réseau ».
- L'historique dit qui a signalé et qui a validé.
