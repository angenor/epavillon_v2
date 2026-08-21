//! Ce que ce module consomme de l'outbox.
//!
//! **Le premier consommateur d'un module métier dans ce dépôt** : la machinerie
//! vit dans le noyau depuis B1 — `EventConsumer`, `ConsumerRegistry`, `claim()`
//! et le relais qui réserve `(consommateur, événement)` avant d'appeler
//! `handle` — mais elle n'avait jamais servi ailleurs que pour la télémétrie.
//!
//! **La garde de rejeu n'est donc pas écrite ici** : le relais n'appelle pas un
//! consommateur deux fois pour le même événement. Le test qui rejoue l'annonce
//! mesure l'absence d'effet, pas la présence d'un code (research.md § R13).

pub mod publication;
