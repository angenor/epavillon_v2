//! Les quatre valeurs dérivées d'une séance, et **le régime exact de chacune**
//! (research.md § R8, écarts n° 111 et n° 112).
//!
//! # Le piège du déclencheur de dérivation, en une phrase
//!
//! `tg_sessions_derive_fields()` ne complète que ce qui est **nul**, et il ne se
//! réveille que sur **cinq colonnes** — `room_id`, `starts_at`, `event_id`,
//! `is_streamed`, `broadcast_channel_id`. Tout ce qui n'est ni nul ni dans cette
//! liste lui échappe : une valeur envoyée **tient**.
//!
//! # Ce que l'écart n° 7 demande, et ce qu'il faut en retenir
//!
//! L'écart demande de refuser trois colonnes à l'écriture, dont le canal de
//! diffusion. **À la lettre, cela casserait une fonctionnalité livrée** :
//! `SessionBroadcastPayload` porte le canal depuis le 18/08, et le commentaire
//! du front dit pourquoi — « l'écran laisse le choix quand l'édition en a
//! plusieurs ». La consigne est donc tenue dans son **intention** : aucune
//! valeur envoyée n'est modifiée sans que la personne le sache.
//!
//! | Champ | Régime | Motif |
//! |---|---|---|
//! | `time_range` | refusé | `GENERATED ALWAYS` : PostgreSQL refuse déjà, mais par une erreur brute |
//! | `enforce_room_exclusivity` | refusé | Le déclencheur ne se réveille pas dessus : une valeur envoyée **tiendrait**, et ferait colorer un chevauchement matériel sur une salle virtuelle |
//! | `broadcast_channel_id` | accepté si la diffusion est **activée**, refusé si elle est **retirée** | Le déclencheur complète quand c'est nul ; la branche `ELSIF NOT NEW.is_streamed` **efface** — c'est là, et là seulement, qu'un choix disparaîtrait en silence |
//! | `event_day_id` | accepté, facultatif | Déduite quand elle n'est pas fournie ; voir `repo/sessions.rs` pour la mise à nul (R9) |

use kernel::error::{ApiError, ErrorCode, Result};

/// Le refus d'un champ dérivé, **en nommant le champ** : sans lui, l'écran ne
/// saurait pas quelle commande souligner.
fn refuser(champ: &str, motif: &str) -> ApiError {
    ApiError::with_message(ErrorCode::SessionDerivedField, motif).field(champ)
}

/// L'intervalle est une colonne engendrée : PostgreSQL refuse déjà, mais par un
/// message anglais portant un nom de colonne. Le refus nommé le remplace par une
/// phrase que l'écran peut afficher.
pub fn refuser_lintervalle() -> ApiError {
    refuser(
        "time_range",
        "Le créneau complet est déduit du début et de la fin : il ne se saisit pas.",
    )
}

/// L'exclusivité de salle est dérivée du caractère virtuel de la salle. Le refus
/// ne protège pas d'un écrasement — le déclencheur ne se réveille pas sur cette
/// colonne — mais d'une **valeur fausse durable**.
pub fn refuser_lexclusivite() -> ApiError {
    refuser(
        "enforce_room_exclusivity",
        "L'occupation du stand est déduite de la salle : elle ne se saisit pas.",
    )
}

/// Un canal désigné alors que la diffusion est **retirée**. C'est le seul cas où
/// la base efface une valeur choisie sans le dire.
pub fn refuser_le_canal_sans_diffusion() -> ApiError {
    refuser(
        "broadcast_channel_id",
        "La diffusion est retirée : aucun canal ne peut être désigné.",
    )
}

/// Le régime du canal, à l'écriture de la diffusion.
///
/// **Accepté quand la diffusion est activée** : le déclencheur ne pose le canal
/// par défaut que lorsque la colonne est nulle, il complète et n'écrase jamais.
/// **Refusé quand elle est retirée** : la branche d'effacement ferait disparaître
/// le choix en silence.
pub fn canal_a_lecriture(diffusee: bool, canal: Option<uuid::Uuid>) -> Result<Option<uuid::Uuid>> {
    match (diffusee, canal) {
        (false, Some(_)) => Err(refuser_le_canal_sans_diffusion()),
        // Le canal est laissé au déclencheur, qui pose celui de l'édition, à
        // défaut celui de la plateforme.
        (true, None) => Ok(None),
        (true, Some(id)) => Ok(Some(id)),
        (false, None) => Ok(None),
    }
}

/// Une référence inconnue, désactivée, ou appartenant à une autre édition.
///
/// **Un seul code pour les quatre** — salle, canal, journée, fil : l'écran
/// recharge ses listes, et c'est le champ qui lui dit laquelle.
pub fn reference_inconnue(champ: &str, quoi: &str) -> ApiError {
    ApiError::with_message(
        ErrorCode::SessionUnknownReference,
        format!("{quoi} n'existe pas, ou n'appartient pas à cette édition."),
    )
    .field(champ)
}

/// La fin doit être postérieure au début — `ck_sessions_period`.
///
/// Traduit **sur le champ de fin** : c'est celui que l'écran vient de bouger en
/// redimensionnant un bloc.
pub fn refuser_le_creneau() -> ApiError {
    ApiError::validation(
        "La fin d'une séance doit être postérieure à son début.",
        "ends_at",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_canal_choisi_est_retenu_quand_la_diffusion_est_activee() {
        let canal = uuid::Uuid::now_v7();
        assert_eq!(canal_a_lecriture(true, Some(canal)).unwrap(), Some(canal));
    }

    /// Sans canal, le déclencheur pose celui de l'édition : le service ne doit
    /// surtout pas en inventer un.
    #[test]
    fn sans_canal_le_service_laisse_faire_le_declencheur() {
        assert_eq!(canal_a_lecriture(true, None).unwrap(), None);
    }

    #[test]
    fn un_canal_designe_sans_diffusion_est_refuse_sur_son_champ() {
        let erreur = canal_a_lecriture(false, Some(uuid::Uuid::now_v7())).unwrap_err();
        assert_eq!(erreur.code, ErrorCode::SessionDerivedField);
        assert_eq!(erreur.field.as_deref(), Some("broadcast_channel_id"));
    }

    #[test]
    fn retirer_la_diffusion_sans_canal_passe() {
        assert_eq!(canal_a_lecriture(false, None).unwrap(), None);
    }

    #[test]
    fn les_deux_refus_nomment_leur_champ() {
        assert_eq!(refuser_lintervalle().field.as_deref(), Some("time_range"));
        assert_eq!(
            refuser_lexclusivite().field.as_deref(),
            Some("enforce_room_exclusivity")
        );
    }
}
