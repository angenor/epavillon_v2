//! Les états d'un rappel, la consolidation d'un groupe, et ce que les écrans
//! lisent.
//!
//! # La consolidation est écrite DEUX fois, et c'est voulu
//!
//! Elle vit dans `engagement.session_reminder_schedule()`, qui est la source de
//! vérité et la seule que les deux lecteurs appellent. Elle est **reproduite
//! ici** pour que les tests puissent la prouver règle par règle sans écrire
//! trente lignes en base par cas — et un test confronte les deux écritures sur
//! le même jeu de lignes. Sans ce miroir, la règle ne serait éprouvable que par
//! des scénarios complets, où un cas sur quatre finirait par manquer.
//!
//! **L'écriture de référence reste le SQL.** Si les deux divergent, c'est le
//! miroir qui a tort.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

macro_rules! enumeration_texte {
    (
        $(#[$meta:meta])*
        $nom:ident { $( $variante:ident => $texte:literal ),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $nom { $( $variante, )* }

        impl $nom {
            pub fn as_str(self) -> &'static str {
                match self { $( Self::$variante => $texte, )* }
            }

            pub fn from_db(valeur: &str) -> Option<Self> {
                Some(match valeur {
                    $( $texte => Self::$variante, )*
                    _ => return None,
                })
            }
        }
    };
}

enumeration_texte! {
    /// `engagement.reminder_status`.
    ///
    /// **`Cancelled` revient à `Pending`**, et c'est la seule transition en
    /// arrière du module. Elle existe parce que `ux_scheduled_reminders_once`
    /// porte sur (séance, personne, canal, décalage) **sans condition d'état** :
    /// une ligne annulée existe toujours, `ON CONFLICT DO NOTHING` ne la
    /// ressuscite pas, et qui se désiste puis revient ne recevrait plus jamais
    /// rien — en silence (B6, R21).
    ReminderStatus {
        Pending => "pending",
        Queued => "queued",
        Sent => "sent",
        Skipped => "skipped",
        Cancelled => "cancelled",
    }
}

enumeration_texte! {
    /// `engagement.notification_channel`. Le canal `Push` est déclaré par le
    /// modèle et **n'a aucune implémentation** : le worker ne sait pas l'servir,
    /// et un rappel qui le viserait serait écarté avec son motif.
    NotificationChannel {
        InApp => "in_app",
        Email => "email",
        Push => "push",
    }
}

enumeration_texte! {
    /// `engagement.notification_criticality`. **`Critical` ignore les
    /// préférences** : sécurité du compte, annulation de séance, obligation
    /// légale. La préférence contraire est tout de même enregistrée, et la
    /// lecture le DIT à l'écran plutôt que de refuser l'écriture (FR-095).
    NotificationCriticality {
        Critical => "critical",
        Important => "important",
        Normal => "normal",
        Low => "low",
    }
}

/// Les motifs d'écart et d'annulation, tels que le modèle les nomme en
/// commentaire de `scheduled_reminders.skip_reason`.
///
/// Ce sont des chaînes libres en base : les déclarer ici évite qu'un
/// « session-cancelled » écrit à la main un jour ne se range jamais avec
/// « session_cancelled ».
pub mod motifs {
    /// L'adresse est sur la liste de suppression.
    pub const SUPPRIME: &str = "suppressed";
    /// La personne a coupé ce canal pour ce type d'avis.
    pub const CANAL_COUPE: &str = "channel_disabled";
    /// La séance a été annulée ou reportée.
    pub const SEANCE_ANNULEE: &str = "session_cancelled";
    /// L'inscription a été annulée, refusée ou basculée en liste d'attente.
    pub const INSCRIPTION_ANNULEE: &str = "registration_cancelled";
    /// La règle a été coupée ou supprimée.
    pub const REGLE_RETIREE: &str = "rule_removed";
    /// Le créneau a reculé et l'instant d'envoi est désormais derrière nous.
    /// **On ne rattrape pas** : c'est la règle que la fonction du modèle
    /// applique déjà à la création — « on ne réveille personne à 3 h du matin
    /// parce qu'un import a pris du retard ».
    pub const INSTANT_DEPASSE: &str = "schedule_passed";
    /// Le canal n'a pas d'expédition. `push` est déclaré par le modèle et
    /// n'existe nulle part ; `in_app` attend l'écran des notifications.
    pub const CANAL_NON_SERVI: &str = "channel_unsupported";
}

/// Une ligne du calendrier des rappels — `ReminderSlot` du contrat du front.
///
/// **Un nombre, jamais une liste nominative.** Une organisation n'administre
/// rien : elle a droit au nombre de destinataires de sa séance, pas à leur
/// identité. La garantie est portée par la signature de
/// `engagement.session_reminder_schedule()`, pas par la discipline d'un
/// appelant — et un test balaie la charge utile sérialisée entière pour
/// l'éprouver.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderSlot {
    /// En minutes, et rangé du plus lointain au plus proche.
    pub offset_before: i32,
    pub channel: String,
    #[serde(with = "time::serde::rfc3339")]
    pub scheduled_for: OffsetDateTime,
    /// L'état de la ligne **la moins avancée** du groupe.
    pub status: String,
    pub recipient_count: i64,
    /// Le motif dominant, et seulement quand le groupe est écarté ou annulé.
    pub skip_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub sent_at: Option<OffsetDateTime>,
}

/// Le calendrier d'une séance — la réponse de `GET /sessions/{id}/reminders`.
///
/// **`has_rule` n'est pas décoratif** : une liste vide se confond avec « tout
/// est parti » (FR-051). Les deux situations demandent des mots différents à
/// l'écran, et lui laisser deviner serait lui demander d'inventer.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SessionReminderSchedule {
    pub slots: Vec<ReminderSlot>,
    pub has_rule: bool,
}

/// La règle **applicable** à une séance — `ApplicableReminderRule`.
///
/// `origin` rend la non-cumulation **vérifiable de l'extérieur** : sans elle, un
/// administrateur ne peut pas distinguer une règle de séance à deux décalages
/// d'une règle d'édition qu'on aurait tronquée (FR-074).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicableReminderRule {
    pub rule_id: Uuid,
    /// `session` ou `event`.
    pub origin: String,
    /// L'identifiant de l'entité dont la règle vient.
    pub origin_id: Uuid,
    pub offsets: Vec<i32>,
    pub channels: Vec<String>,
    pub type_code: String,
    pub template_id: Option<Uuid>,
    pub is_active: bool,
}

/// Une règle de rappel — `ReminderRule` du contrat du front.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReminderRule {
    pub id: Uuid,
    pub event_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    /// En minutes, cumulés.
    pub offsets: Vec<i32>,
    pub channels: Vec<String>,
    pub type_code: String,
    pub template_id: Option<Uuid>,
    pub is_active: bool,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// L'état consolidé d'un groupe, **miroir Rust de la règle SQL**.
///
/// L'ordre est celui de la prudence : « parti » ne se dit pas tant qu'une
/// personne attend encore son courriel.
pub fn consolider(etats: &[ReminderStatus]) -> Option<ReminderStatus> {
    if etats.is_empty() {
        return None;
    }
    let compte = |vise: ReminderStatus| etats.iter().filter(|e| **e == vise).count();

    Some(if compte(ReminderStatus::Pending) > 0 {
        ReminderStatus::Pending
    } else if compte(ReminderStatus::Queued) > 0 {
        ReminderStatus::Queued
    } else if compte(ReminderStatus::Sent) > 0 {
        ReminderStatus::Sent
    } else if compte(ReminderStatus::Skipped) >= compte(ReminderStatus::Cancelled) {
        ReminderStatus::Skipped
    } else {
        ReminderStatus::Cancelled
    })
}

#[cfg(test)]
mod tests {
    use super::ReminderStatus::*;
    use super::*;

    #[test]
    fn une_seule_ligne_en_attente_retient_le_groupe_entier() {
        assert_eq!(consolider(&[Sent, Sent, Sent, Pending]), Some(Pending));
        assert_eq!(consolider(&[Sent, Queued]), Some(Queued));
    }

    #[test]
    fn un_groupe_entierement_parti_est_parti() {
        assert_eq!(consolider(&[Sent, Sent]), Some(Sent));
    }

    /// Un envoi réussi prime sur les écarts : une personne a bien reçu son
    /// courriel, et le dire « écarté » serait faux.
    #[test]
    fn un_envoi_reussi_prime_sur_les_ecarts() {
        assert_eq!(consolider(&[Skipped, Cancelled, Sent]), Some(Sent));
    }

    #[test]
    fn un_groupe_mort_prend_letat_majoritaire() {
        assert_eq!(consolider(&[Skipped, Skipped, Cancelled]), Some(Skipped));
        assert_eq!(
            consolider(&[Cancelled, Cancelled, Skipped]),
            Some(Cancelled)
        );
        // Égalité : « écarté » l'emporte, comme dans le SQL.
        assert_eq!(consolider(&[Skipped, Cancelled]), Some(Skipped));
    }

    #[test]
    fn un_groupe_vide_na_pas_detat() {
        assert_eq!(consolider(&[]), None);
    }
}
