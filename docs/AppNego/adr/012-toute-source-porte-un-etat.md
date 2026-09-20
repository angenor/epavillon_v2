# ADR-012 — Toute source porte un état, et se corrige par une note

**Statut** : accepté — 18/09/2026

## Contexte

Une référence de document ou un passage de vidéo de formation peut devenir dépassé, ou se révéler faux. Un document se remplace ; une vidéo ne se corrige pas.

## Décision

Chaque source porte un **état** — à classer, valide, à vérifier, dépassée, retirée —, sa session d'origine, et un « remplacée par ». Quatre gestes la tiennent à jour :

1. Toute réponse, toute citation, toute entrée de FAQ a un bouton **« Dépassé ou faux »**, qui alimente la file des experts.
2. Un expert pose une **note de correction** sur une page, ou sur un intervalle de vidéo ; l'assistant la cite avec la source.
3. L'assistant ne lit que le valide ; s'il s'appuie sur une source ancienne, il le dit et la date.
4. À chaque nouveau cycle de négociation, les sources du précédent repassent **« à vérifier »**.

## Conséquences

- Le modèle a déjà `supersedes_id` sur les documents et le motif « dépassé » sur les retours ; l'état complet et les notes de correction s'ajoutent au SQL.
- Les citations restent figées dans la conversation : on sait toujours ce qui a été dit, et sur quelle version.
- La relecture annuelle est une charge d'experts, à prévoir.
