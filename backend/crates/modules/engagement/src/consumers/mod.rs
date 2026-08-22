//! Les consommateurs d'outbox du module.
//!
//! **Leurs noms entrent dans `platform.inbox_events`** : ils identifient le
//! consommateur pour la garde d'idempotence, et les renommer ferait rejouer
//! **tout** l'historique — c'est-à-dire, ici, remettre en file tous les rappels
//! jamais programmés.

pub mod notifications;
pub mod reminders;
