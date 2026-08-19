# API — suivi des prompts

> Extrait de la [progression](../PROGRESSION.md). Les prompts sont dans [PROMPTS_DEVELOPPEMENT.md](../PROMPTS_DEVELOPPEMENT.md).

| Prompt | Module | État | Notes |
|--------|--------|------|-------|
| B0 | Constitution Spec Kit | ⬜ | |
| B1 | Socle + Identité | ⬜ | **Cinq obligations relevées en écrivant A12** : (1) `identity.role.assign` se vérifie SUR LA PORTÉE VISÉE (`has_permission(acteur, 'identity.role.assign', scope_type, scope_id)`) — un compte détaché sur la COP31 ne peut attribuer que là ; (2) **retirer exige le même droit qu'attribuer, sur la même portée**, sans quoi un administrateur détaché retirerait un rôle global qu'il n'aurait jamais pu accorder ; (3) le paramètre `granted` des écritures de rôle DISPARAÎT — l'API lit sa propre session, un client qui déclare ses droits n'est pas un contrôle d'accès ; (4) la file RGPD ne se filtre pas par édition et exige la portée globale ; (5) `anonymize_person()` ne répond qu'à une demande d'EFFACEMENT — l'exécuter sur un export détruirait l'identité de qui ne demandait qu'une copie |
| B2 | Organisations | ⬜ | **Quatre obligations relevées en écrivant A11** : (1) les choix de champ de la fusion sont un `UPDATE` de la fiche CIBLE, à faire AVANT `org.merge_organizations()` et dans la MÊME transaction ; (2) la lecture des organisations s'ouvre sur `org.organization.read` quelle que soit sa portée, la liste étant filtrée sur les éditions administrées, tandis que la fusion exige `org.organization.merge` en portée GLOBALE ; (3) `mv_organization_scorecard` est matérialisée — soit l'API rend son âge, soit `score_confiance` se relit depuis `org.organizations` ; (4) `org.compute_trust_score()` n'est appelée par aucun trigger : une tâche différée ou un trigger reste à décider |
| B3 | Événements | ⬜ | |
| B4 | Propositions | ⬜ | |
| B5 | Sessions | ⬜ | |
| B6 | Média + Engagement | ⬜ | **Le module `live` n'a AUCUN prompt d'API**, alors que l'écran A13 en consomme trois fonctions (`event_incidents`, `publish_incident`, `unpublish_incident`) et une table. Les rattacher ici est le moins mauvais choix — c'est le prompt des services transverses —, mais c'est une décision à prendre, pas un fait acquis. **Trois obligations relevées en écrivant A13** : (1) `live.incident.publish` se vérifie SUR L'ÉDITION visée, jamais globalement ; (2) publier et enregistrer sont deux écritures distinctes, et republier remet `unpublished_at`, `unpublished_by` et `unpublish_reason` à NULL comme le fait `publish_incident()` ; (3) les cibles offertes au formulaire sont celles de l'édition administrée — journées, séances, organisations qui y animent —, sans quoi une URL forgée viserait la journée d'une autre COP |
| B7 | Raccordement du front | ⬜ | |
