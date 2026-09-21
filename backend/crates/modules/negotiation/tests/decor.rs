//! Le décor du harnais tient, et le module est bien monté sur une base réelle.
//!
//! Ce test ne prouve aucune règle métier — les récits s'en chargent. Il prouve
//! que `commun/` compile et que ce qu'il sème existe : sans lui, le harnais ne
//! serait compilé par personne avant le premier test du récit suivant, et une
//! colonne mal nommée ne se verrait qu'alors.

mod commun;

use commun::{decor, etat_du_code, Bac};

#[tokio::test]
async fn le_decor_pose_un_espace_un_code_et_ses_comptes() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    assert_ne!(d.space_id, d.autre_space_id);
    assert_eq!(
        etat_du_code(&bac, d.code_id).await,
        "active",
        "un code sans quota, sans échéance et non révoqué est actif"
    );
}
