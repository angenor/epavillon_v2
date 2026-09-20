# ADR-009 — La source officielle fait foi ; l'import se coupe seul

**Statut** : accepté — 18/09/2026

## Contexte

Le porteur a choisi d'importer automatiquement les sessions de négociation dès la COP31. Or la CCNUCC n'offre aucun flux documenté — un fichier JSON propre à chaque session, inconnu avant l'ouverture —, et ses conditions d'utilisation demandent un accord écrit pour reprendre le calendrier. La demande est en cours ; la direction est confiante.

## Décision

Le mécanisme d'import **se construit sans attendre l'accord**. Chaque session importée affiche son origine, son heure de dernière lecture et un lien vers l'original. L'état par défaut de l'écran est le lien vers le programme officiel ; l'import s'allume d'un interrupteur, et **se coupe seul** passé un seuil de lectures manquées.

## Conséquences

- Jamais une donnée périmée présentée comme fraîche : l'application dit qu'elle ne sait plus.
- Un titre traduit automatiquement est marqué ; l'anglais reste visible.
- Le mécanisme s'éprouve sur les données archivées de la COP30.
- Tant que l'accord n'est pas écrit, l'interrupteur reste une décision de la direction.
