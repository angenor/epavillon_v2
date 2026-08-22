//! **L'annonce, le dépôt, et la déduplication.**
//!
//! # Trois choses que ce fichier ne fait pas, et c'est délibéré
//!
//! 1. **Il n'émet rien et n'enfile rien.** Insérer une ligne dans
//!    `media.assets` déclenche `media.tg_enqueue_processing()`, qui met le
//!    traitement en file **et** émet l'annonce de dépôt. Un geste de plus
//!    produirait deux traitements par fichier, et le doublon ne se verrait qu'en
//!    production (FR-020). Un test compte les lignes plutôt que de relire ce
//!    commentaire.
//! 2. **Il ne débite aucun quota.** Les compteurs appartiennent aux
//!    déclencheurs et à la réconciliation.
//! 3. **Il ne revérifie aucun invariant de la base.** Le contrôle de capacité
//!    en est un emploi prévu, pas une garde parallèle : `has_storage_capacity()`
//!    est la fonction que le modèle décrit comme « contrôle opposable au
//!    téléversement », et le refus final reste celui de `tg_enforce_quota`.
//!
//! # Le tempo du dépôt, et pourquoi il est dans cet ordre
//!
//! Les métadonnées arrivent **avant** le fichier. C'est ce qui permet de refuser
//! un type, un poids ou un droit **sans avoir lu un octet** — sur un fond vidéo
//! de deux cents mégaoctets, la différence n'est pas théorique.
//!
//! Ensuite seulement le flux est lu, **vers une clé temporaire**, en calculant
//! son empreinte au passage. À la fin :
//!
//! - empreinte déjà connue d'un objet vivant → le temporaire est **supprimé**,
//!   et l'objet existant est rendu. Aucun second octet n'est conservé, et le
//!   second déposant ne consomme **aucun** quota (R10, écart n° 128) ;
//! - sinon → le temporaire est **renommé** vers la clé définitive, et la ligne
//!   est écrite.
//!
//! L'empreinte n'est jamais acceptée du client sans être recalculée : celle
//! qu'il annonce sert seulement à lui épargner l'envoi.

use kernel::error::{ApiError, ErrorCode, Result};
use kernel::RequestContext;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::asset::{Asset, QuotaSnapshot, UploadVerdict};
use crate::domain::{keys, rules};
use crate::repo::{assets, quotas};
use crate::service::authz::{self, Porteuse};
use crate::state::MediaState;
use crate::storage::FluxOctets;

/// Ce qu'une annonce déclare — `UploadDeclaration`.
///
/// **Aucun octet.** L'annonce est une question, pas une tentative : elle n'écrit
/// rien, ne réserve ni espace, ni clé, ni identifiant (FR-016).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UploadDeclaration {
    pub filename: String,
    pub mime_type: String,
    pub byte_size: i64,
    /// L'entité que le fichier illustrera. Facultative : un objet peut être
    /// déposé sans rôle visé, et rattaché plus tard.
    pub owner_schema: Option<String>,
    pub owner_table: Option<String>,
    pub owner_id: Option<Uuid>,
    pub role: Option<String>,
    /// L'empreinte, **si le client sait la calculer**. Elle évite le transfert
    /// entier quand le contenu est déjà connu. Le contrat du front ne la porte
    /// pas encore ; la route l'accepte quand même (FR-011).
    pub checksum_sha256: Option<String>,
}

/// Les métadonnées d'un dépôt réel — les champs qui précèdent le fichier dans
/// le corps composite.
#[derive(Debug, Clone, Default)]
pub struct MetadonneesDepot {
    pub filename: String,
    pub mime_type: String,
    /// Le poids **annoncé**. Sert à refuser avant de lire ; le poids qui compte
    /// est celui réellement reçu.
    pub byte_size: Option<i64>,
    pub owner_schema: Option<String>,
    pub owner_table: Option<String>,
    pub owner_id: Option<Uuid>,
    pub role: Option<String>,
    pub alt_text: Option<serde_json::Value>,
    pub caption: Option<serde_json::Value>,
    pub credit: Option<String>,
    pub license_code: Option<String>,
    pub visibility: Option<String>,
}

// -----------------------------------------------------------------------------
// L'annonce
// -----------------------------------------------------------------------------

/// Le verdict que rendrait le dépôt. **N'écrit rien.**
///
/// Tous les refus sortent en **200** avec leur code : ce sont des réponses, pas
/// des erreurs. Le seul refus qui sorte en erreur est celui du **droit** — parce
/// qu'il ne se distingue pas d'une entité inexistante, et qu'il n'a donc rien à
/// dire de plus.
pub async fn annoncer(
    state: &MediaState,
    acteur: Uuid,
    declaration: &UploadDeclaration,
) -> Result<UploadVerdict> {
    let porteuse = porteuse_de(
        declaration.owner_schema.as_deref(),
        declaration.owner_table.as_deref(),
        declaration.owner_id,
    );
    let organisation = match porteuse {
        Some(p) => authz::exiger_le_droit(state.pool(), acteur, p).await?,
        None => None,
    };

    let bucket = assets::bucket_par_defaut(state.pool()).await?;

    // **L'empreinte d'abord** (FR-012) : si le contenu est déjà là, rien d'autre
    // n'a d'importance — ni le poids, ni le quota, puisque rien ne sera écrit.
    if let Some(empreinte) = declaration.checksum_sha256.as_deref() {
        if let Some(existant) =
            assets::par_empreinte_en_lecture(state.pool(), empreinte, &bucket).await?
        {
            let objet = assets::par_id(state.pool(), existant).await?;
            return Ok(UploadVerdict {
                accepted: true,
                code: None,
                field: None,
                message: None,
                existing_asset: objet.map(Box::new),
                quota: None,
            });
        }
    }

    if let Some(refus) = refus_de_regle(
        state,
        porteuse,
        declaration.role.as_deref(),
        &declaration.mime_type,
        declaration.byte_size,
    )
    .await?
    {
        return Ok(refus);
    }

    if let Some(refus) = refus_de_quota(state, organisation, declaration.byte_size).await? {
        return Ok(refus);
    }

    Ok(UploadVerdict {
        accepted: true,
        code: None,
        field: None,
        message: None,
        existing_asset: None,
        quota: None,
    })
}

// -----------------------------------------------------------------------------
// Le dépôt
// -----------------------------------------------------------------------------

/// Ce que le dépôt rend : l'objet, et le fait qu'il existait déjà.
#[derive(Debug)]
pub struct ResultatDepot {
    pub asset: Asset,
    /// Vrai quand le contenu était déjà connu. **C'est un succès**, pas un
    /// refus : l'écran affiche l'image, et rien n'a été écrit une seconde fois.
    pub deduplique: bool,
}

/// Le dépôt en un seul geste. Les métadonnées ont déjà été lues ; le flux, non.
pub async fn deposer(
    state: &MediaState,
    ctx: &RequestContext,
    acteur: Uuid,
    metadonnees: MetadonneesDepot,
    flux: FluxOctets,
) -> Result<ResultatDepot> {
    let porteuse = porteuse_de(
        metadonnees.owner_schema.as_deref(),
        metadonnees.owner_table.as_deref(),
        metadonnees.owner_id,
    );
    let organisation = match porteuse {
        Some(p) => authz::exiger_le_droit(state.pool(), acteur, p).await?,
        None => None,
    };

    // **Le texte alternatif est exigé AVANT de lire un octet** (R9, écart
    // n° 129). `ck_assets_alt_text_required` interdit à une image d'atteindre
    // l'état servable sans lui : accepter le dépôt produirait un objet bloqué en
    // traitement pour toujours, et un emplacement vide inexplicable.
    if metadonnees.mime_type.starts_with("image/") && !porte_un_texte(&metadonnees.alt_text) {
        return Err(ApiError::new(ErrorCode::MediaAltTextRequired).field("alt_text"));
    }

    // Les trois refus qui n'exigent aucun octet. Ici ils sortent en **erreur** et
    // non en verdict : le client a choisi d'envoyer, il n'a pas posé de question.
    if let Some(refus) = refus_de_regle(
        state,
        porteuse,
        metadonnees.role.as_deref(),
        &metadonnees.mime_type,
        metadonnees.byte_size.unwrap_or(0),
    )
    .await?
    {
        return Err(erreur_du_verdict(refus));
    }
    if let Some(annonce) = metadonnees.byte_size {
        if let Some(refus) = refus_de_quota(state, organisation, annonce).await? {
            return Err(erreur_du_verdict(refus));
        }
    }

    let bucket = assets::bucket_par_defaut(state.pool()).await?;
    let jeton = Uuid::now_v7();
    let cle_temporaire = keys::cle_temporaire(jeton);

    // Le flux est mesuré **au passage** : empreinte et poids réel sortent de la
    // même lecture, sans que le fichier tienne jamais en mémoire (FR-017).
    let mesure = crate::service::stream::Mesure::nouvelle(state.config().media.max_upload_bytes);
    let flux_mesure = mesure.envelopper(flux);

    let ecrits = match state
        .storage()
        .put_stream(&cle_temporaire, &metadonnees.mime_type, flux_mesure)
        .await
    {
        Ok(ecrits) => ecrits,
        Err(erreur) => {
            // **Rien ne traîne** : ce qui a été reçu est retiré du stockage, et
            // aucune description n'est écrite (FR-017, T056).
            let _ = state.storage().delete(&cle_temporaire).await;
            return Err(mesure.erreur_ou(erreur));
        }
    };

    let (empreinte, octets) = mesure.resultat();
    let nettoyer = |erreur: ApiError| async {
        let _ = state.storage().delete(&cle_temporaire).await;
        erreur
    };

    if octets == 0 || octets != ecrits {
        return Err(nettoyer(
            ApiError::new(ErrorCode::MediaUploadIncomplete)
                .field("file")
                .detail(format!("{octets} octets mesurés, {ecrits} écrits")),
        )
        .await);
    }
    // Le poids annoncé n'est pas une promesse : s'il diffère de ce qui arrive,
    // c'est le flux qu'on refuse, pas la déclaration (FR-017).
    if let Some(annonce) = metadonnees.byte_size {
        if annonce != octets as i64 {
            return Err(nettoyer(
                ApiError::new(ErrorCode::MediaUploadIncomplete)
                    .field("file")
                    .detail(format!("{annonce} octets annoncés, {octets} reçus")),
            )
            .await);
        }
    }

    let mut tx = state.db().write(ctx).await?;

    // **La déduplication, et c'est ici qu'elle se joue.**
    if let Some(existant) = assets::par_empreinte(&mut tx, &empreinte, &bucket).await? {
        tx.rollback().await?;
        let _ = state.storage().delete(&cle_temporaire).await;
        let asset = assets::par_id(state.pool(), existant)
            .await?
            .ok_or_else(|| ApiError::internal("objet dédupliqué introuvable après lecture"))?;
        return Ok(ResultatDepot {
            asset,
            deduplique: true,
        });
    }

    let instant = assets::maintenant(state.pool()).await?;
    let asset_id = Uuid::now_v7();
    let cle = keys::cle_objet(instant, asset_id, &metadonnees.filename);

    let objet = assets::NouvelObjet {
        bucket: bucket.clone(),
        object_key: cle.clone(),
        checksum_sha256: empreinte,
        mime_type: metadonnees.mime_type.clone(),
        byte_size: octets as i64,
        original_filename: Some(metadonnees.filename.clone()),
        owner_person_id: Some(acteur),
        owner_organization_id: organisation,
        visibility: metadonnees
            .visibility
            .clone()
            .unwrap_or_else(|| "public".to_owned()),
        alt_text: metadonnees.alt_text.clone(),
        caption: metadonnees.caption.clone(),
        credit: metadonnees.credit.clone(),
        license_code: metadonnees.license_code.clone(),
    };

    // L'objet est déplacé **avant** l'écriture : une description qui pointerait
    // vers une clé absente serait pire qu'un objet orphelin.
    //
    // **Un échec de déplacement efface le temporaire tout de suite**, et ne le
    // laisse pas « à la purge » : la purge ne ramasse que ce qui est **décrit en
    // base**, et un temporaire orphelin ne l'est pas — il resterait sur le
    // disque pour toujours, ce qui est exactement le défaut de la v1 que ce
    // schéma corrige.
    if let Err(erreur) = state.storage().rename(&cle_temporaire, &cle).await {
        let _ = state.storage().delete(&cle_temporaire).await;
        return Err(ApiError::from(erreur));
    }

    let ecriture = assets::ecrire(&mut tx, &objet).await;
    let asset_id = match ecriture {
        Ok(id) => id,
        Err(erreur) => {
            tx.rollback().await?;
            // Le refus de la base — quota atteint, entre autres — laisse
            // l'objet sur le stockage : on le retire, puis on traduit.
            let _ = state.storage().delete(&cle).await;
            return Err(traduire(erreur, state, organisation).await);
        }
    };

    let asset = assets::par_id_dans(&mut tx, asset_id)
        .await?
        .ok_or_else(|| ApiError::internal("objet introuvable juste après son écriture"))?;
    tx.commit().await?;

    Ok(ResultatDepot {
        asset,
        deduplique: false,
    })
}

// -----------------------------------------------------------------------------
// Les refus, et leur forme
// -----------------------------------------------------------------------------

fn porteuse_de<'a>(
    schema: Option<&'a str>,
    table: Option<&'a str>,
    id: Option<Uuid>,
) -> Option<Porteuse<'a>> {
    match (schema, table, id) {
        (Some(owner_schema), Some(owner_table), Some(owner_id)) => Some(Porteuse {
            owner_schema,
            owner_table,
            owner_id,
        }),
        _ => None,
    }
}

fn porte_un_texte(valeur: &Option<serde_json::Value>) -> bool {
    valeur.as_ref().is_some_and(|v| {
        v.as_object().is_some_and(|o| {
            o.values()
                .any(|t| t.as_str().is_some_and(|s| !s.trim().is_empty()))
        })
    })
}

/// Le type et le poids, contre la ligne de la table blanche visée.
///
/// **Sans rôle visé, aucune règle ne s'applique** : un objet peut être déposé
/// puis rattaché plus tard, et c'est le rattachement qui l'éprouvera.
async fn refus_de_regle(
    state: &MediaState,
    porteuse: Option<Porteuse<'_>>,
    role: Option<&str>,
    mime_type: &str,
    byte_size: i64,
) -> Result<Option<UploadVerdict>> {
    let (Some(p), Some(role)) = (porteuse, role) else {
        return Ok(None);
    };

    let regle = sqlx::query!(
        r#"SELECT r.allowed_mime_prefixes AS "prefixes!", r.max_byte_size
             FROM media.attachable_roles r
            WHERE r.owner_schema = $1 AND r.owner_table = $2
              AND r.role = $3::text::media.attachment_role
              AND r.is_active"#,
        p.owner_schema,
        p.owner_table,
        role
    )
    .fetch_optional(state.pool())
    .await?;

    let Some(regle) = regle else {
        return Ok(Some(verdict_refuse(
            ErrorCode::MediaRoleNotDeclared,
            Some("role"),
            None,
        )));
    };

    if !rules::type_accepte(mime_type, &regle.prefixes) {
        return Ok(Some(verdict_refuse(
            ErrorCode::MediaMimeNotAllowed,
            Some("file"),
            Some(format!(
                "type « {mime_type} » reçu pour le rôle « {role} » ; accepté : {}",
                regle.prefixes.join(", ")
            )),
        )));
    }

    if !rules::poids_accepte(byte_size, regle.max_byte_size) {
        return Ok(Some(verdict_refuse(
            ErrorCode::MediaTooLarge,
            Some("file"),
            Some(format!(
                "{byte_size} octets reçus pour le rôle « {role} » ; limite : {}",
                regle.max_byte_size.unwrap_or_default()
            )),
        )));
    }

    Ok(None)
}

/// L'espace restant, avant l'écriture. **Le refus porte ses trois chiffres.**
async fn refus_de_quota(
    state: &MediaState,
    organisation: Option<Uuid>,
    byte_size: i64,
) -> Result<Option<UploadVerdict>> {
    if quotas::a_la_place(state.pool(), organisation, byte_size).await? {
        return Ok(None);
    }

    let mut refus = verdict_refuse(ErrorCode::MediaQuotaExceeded, None, None);
    refus.quota = quotas::etat(state.pool(), organisation).await?;
    Ok(Some(refus))
}

fn verdict_refuse(code: ErrorCode, champ: Option<&str>, detail: Option<String>) -> UploadVerdict {
    UploadVerdict {
        accepted: false,
        code: Some(code.as_str().to_owned()),
        field: champ.map(str::to_owned),
        message: Some(detail.unwrap_or_else(|| code.message().to_owned())),
        existing_asset: None,
        quota: None,
    }
}

/// Le même refus, mais en erreur : sur un dépôt réel, le client n'a pas posé de
/// question — il a envoyé.
fn erreur_du_verdict(verdict: UploadVerdict) -> ApiError {
    let code = verdict
        .code
        .as_deref()
        .and_then(code_depuis_le_texte)
        .unwrap_or(ErrorCode::ValidationFailed);

    let mut erreur = ApiError::new(code);
    if let Some(champ) = verdict.field {
        erreur = erreur.field(champ);
    }
    if let Some(quota) = verdict.quota {
        erreur = erreur.with_quota(quota);
    }
    erreur
}

fn code_depuis_le_texte(texte: &str) -> Option<ErrorCode> {
    ErrorCode::ALL.iter().copied().find(|c| c.as_str() == texte)
}

/// **Le refus de quota de la base porte le MÊME code que le refus préalable**
/// (R14, écart n° 136).
///
/// `SQLSTATE 53100` est `disk_full`, un état d'erreur **système** : traduit
/// naïvement, il sortirait en 500 là où l'écran sait afficher trois chiffres. Et
/// faire porter deux codes différents au même refus selon qu'il vient d'avant ou
/// d'après l'écriture obligerait l'écran à traiter deux fois le même cas — la
/// course du cas limite n° 13 rend les deux atteignables.
async fn traduire(erreur: sqlx::Error, state: &MediaState, organisation: Option<Uuid>) -> ApiError {
    if kernel::pg_error::sqlstate(&erreur).as_deref() == Some("53100") {
        let mut api = ApiError::new(ErrorCode::MediaQuotaExceeded);
        if let Ok(Some(quota)) = quotas::etat(state.pool(), organisation).await {
            api = api.with_quota(quota);
        }
        return api;
    }
    ApiError::from(erreur)
}

/// **Les trois chiffres voyagent dans le MESSAGE**, et c'est un arbitrage.
///
/// Le corps d'erreur du noyau porte un code, un message et un champ — pas de
/// charge utile structurée, et lui en ajouter une modifierait le noyau pour un
/// seul cas d'un seul module. « L'espace est atteint » sans chiffre ne dit
/// pourtant pas quoi faire : supprimer un fichier, ou demander un relèvement.
///
/// Les chiffres sont donc écrits **dans le message français**, que l'écran
/// affiche tel quel. La forme structurée existe là où elle a sa place : dans le
/// verdict de la pré-vérification, qui sort en 200 et porte `quota`.
trait AvecQuota {
    fn with_quota(self, quota: QuotaSnapshot) -> Self;
}

impl AvecQuota for ApiError {
    fn with_quota(self, quota: QuotaSnapshot) -> Self {
        let message = format!(
            "L'espace de stockage de cette organisation est atteint : {} utilisés sur {}, il reste {}.",
            en_octets_lisibles(quota.used_bytes),
            en_octets_lisibles(quota.max_bytes),
            en_octets_lisibles(quota.remaining_bytes)
        );
        let detail = format!(
            "max_bytes={} used_bytes={} remaining_bytes={} used_files={} max_files={}",
            quota.max_bytes,
            quota.used_bytes,
            quota.remaining_bytes,
            quota.used_files,
            quota.max_files
        );
        ApiError {
            message,
            ..self.detail(detail)
        }
    }
}

/// Des octets en unités que l'on lit. Le séparateur décimal est la virgule, et
/// les unités sont celles de la base 1024 : c'est ce qu'affiche un système de
/// fichiers, et une organisation compare son quota à ce qu'elle voit ailleurs.
fn en_octets_lisibles(octets: i64) -> String {
    const UNITES: [(&str, i64); 4] = [
        ("Gio", 1024 * 1024 * 1024),
        ("Mio", 1024 * 1024),
        ("Kio", 1024),
        ("octets", 1),
    ];

    for (nom, seuil) in UNITES {
        if octets >= seuil {
            if seuil == 1 {
                return format!("{octets} octets");
            }
            let valeur = octets as f64 / seuil as f64;
            return format!("{valeur:.1} {nom}").replace('.', ",");
        }
    }
    "0 octet".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_octets_se_lisent_dans_lunite_qui_convient() {
        assert_eq!(en_octets_lisibles(0), "0 octet");
        assert_eq!(en_octets_lisibles(512), "512 octets");
        assert_eq!(en_octets_lisibles(5 * 1024 * 1024), "5,0 Mio");
        assert_eq!(en_octets_lisibles(5_368_709_120), "5,0 Gio");
    }

    /// Le refus doit **dire quoi faire** : sans chiffre, « l'espace est
    /// atteint » laisse la personne devant un écran muet.
    #[test]
    fn le_refus_de_quota_porte_ses_trois_chiffres() {
        let erreur = ApiError::new(ErrorCode::MediaQuotaExceeded).with_quota(QuotaSnapshot {
            max_bytes: 5_368_709_120,
            used_bytes: 5_368_709_000,
            remaining_bytes: 120,
            max_files: 5000,
            used_files: 4200,
        });

        assert_eq!(erreur.code, ErrorCode::MediaQuotaExceeded);
        assert!(erreur.message.contains("5,0 Gio"));
        assert!(erreur.message.contains("120 octets"));
        assert!(erreur.detail.is_some_and(|d| d.contains("used_files=4200")));
    }

    #[test]
    fn un_texte_alternatif_vide_ne_compte_pas() {
        assert!(!porte_un_texte(&None));
        assert!(!porte_un_texte(&Some(serde_json::json!({}))));
        assert!(!porte_un_texte(&Some(serde_json::json!({"fr": "   "}))));
        assert!(porte_un_texte(&Some(
            serde_json::json!({"fr": "Le logo du ROAC"})
        )));
    }
}
