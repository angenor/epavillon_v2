# 06 — Après le MVP

> Ce qui suit les six étapes de la [feuille de route](04-roadmap.md), dans l'ordre. Le modèle de données couvre déjà presque tout ; il reste à écrire le code, et à tenir les règles de confiance.

## 6 — Échanges

**But** : remplacer à terme le groupe WhatsApp des négociatrices — [ADR-014](adr/014-whatsapp-est-remplace-a-terme.md).

- Canaux par thématique et par promotion, annonces de l'IFDD, canal réservé du réseau des négociatrices, conversations privées avec une experte, une négociatrice ou un administrateur.
- **Questions aux experts** : la réponse d'un expert peut rejoindre la FAQ, qui nourrira l'assistant.
- **Proposer un document** en deux gestes, depuis une conversation ou depuis le partage du téléphone — ce qui se perd aujourd'hui dans WhatsApp arrive dans l'« arrivée » du corpus.
- Annuaire du réseau : trouver la coordonnatrice de sa thématique, écrire à une aînée.

**Préalables** : la coquille Capacitor et les notifications poussées, sans lesquelles personne ne quitte WhatsApp ; la session par jeton.

**Parades** : commencer par ce que WhatsApp ne fait pas ; ne fermer le groupe que si 80 % de ses membres sont actifs dans l'application pendant une session ; dire sans détour que l'IFDD héberge et modère — ce n'est pas le chiffrement de WhatsApp.

## 7 — Assistant IA

**But** : une réponse en français qui cite son document et sa page, ou sa vidéo et sa minute.

- Il ne lit que la **référence**, validée par un expert — [ADR-011](adr/011-un-corpus-a-deux-etages.md). Les sources sont des documents, des transcriptions, la FAQ ; d'autres natures pourront s'ajouter.
- Chaque source porte un état ; chaque réponse a son bouton « Dépassé ou faux » ; un expert pose des notes de correction — [ADR-012](adr/012-toute-source-porte-un-etat.md).
- Il sait dire « Je ne sais pas », propose alors de poser la question à un expert, et ne conseille jamais la position d'un pays.
- **Il se juge sur pièces** : 30 à 50 vraies questions dont les experts connaissent la réponse, rejouées à chaque changement de modèle ou de corpus.
- Un quota par personne, déjà prévu par le modèle, borne la dépense.
- Service Python interne, OpenRouter — [ADR-004](adr/004-rust-en-facade-python-au-sidecar.md), [ADR-005](adr/005-openrouter-et-embedding-versionne.md).

**Préalable** : le correctif de la dimension des vecteurs, et des transcriptions horodatées — à relire par sondage, les sigles et les accents trompant la transcription automatique.

## 8 — Formations et quiz

- Les seize modules de formation en vidéo, avec leur transcription horodatée : on ouvre une vidéo à la minute citée par l'assistant.
- **Quiz** à choix multiple et vrai ou faux, générés par l'IA à la demande d'un administrateur, depuis un document ou un enregistrement. Un quiz est aussi un résumé : il dit ce qu'il faut retenir.
- **Aucun quiz n'est publié sans relecture d'un expert**, et chaque question garde son passage source — [ADR-013](adr/013-aucun-quiz-publie-sans-relecture.md).
- Une négociatrice peut en régénérer un pour elle — questions reformulées, ou portant sur d'autres passages. Il reste privé, marqué « Non relu », et peut être proposé à la relecture.
- Les documents qui circulent pendant une session peuvent donner des quiz en français, une fois promus en référence.

## 9 — Restitutions — à confirmer sur le terrain

- Une aide **privée** à la rédaction du compte rendu du jour, prérempli avec les sessions suivies.
- Un partage par **cercle** choisi — ma délégation, mon groupe de négociation, tout le réseau — [ADR-015](adr/015-restitutions-privees-partage-par-cercle.md).
- Une synthèse automatique par thématique, tirée des seules restitutions partagées, et marquée comme telle.

Aucune source publique ne décrit ce besoin : il se vérifie par entretiens à la première COP, avant d'écrire une ligne.

## Idées en réserve

| Idée | Pourquoi elle attend |
|---|---|
| Résumer en français un projet de texte en anglais | Un texte de négociation se joue au mot et au crochet près : seulement comme « aide à la lecture — le texte anglais fait foi » |
| Capter les courriels de la CCNUCC par point de l'ordre du jour | Un essai de la COP30, sans garantie de suite ; à éprouver en coulisse avant de s'y appuyer |
| Capter automatiquement le groupe WhatsApp | Techniquement fermé ; le partage volontaire vers l'application le remplace |
| Marrainage entre une nouvelle et une aînée | Très demandé dans les enquêtes ; suppose les Échanges et l'annuaire |
| Exporter son agenda vers le calendrier du téléphone | Simple, mais après les sessions de négociation |
| Biodiversité et désertification | Le modèle a déjà ses espaces ; aucune structure à changer, du contenu à produire |
