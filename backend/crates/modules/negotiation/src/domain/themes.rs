//! Les thématiques qu'une personne suit — **des codes, jamais des libellés**.
//!
//! Le corps ne porte que des codes, et c'est ce qui rend l'empreinte sûre :
//! deux appareils de la même personne, l'un en français, l'autre en anglais,
//! doivent voir **la même empreinte pour un même état**. Les libellés viennent
//! de la route publique des termes, que le client lit de toute façon.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize)]
pub struct FollowedTheme {
    pub code: String,
    #[serde(with = "time::serde::rfc3339")]
    pub followed_at: OffsetDateTime,
}

/// `MyThemes` — ce que `GET /negotiation/me/themes` rend.
#[derive(Debug, Clone, Serialize)]
pub struct MyThemes {
    pub themes: Vec<FollowedTheme>,
    /// Les thématiques suivies dont on est prévenu des changements.
    pub notify: Vec<String>,
}

impl MyThemes {
    /// L'empreinte de **l'état**, pas de sa représentation — `notify` compris,
    /// sinon un autre appareil reçoit `304` et garde l'ancien réglage. Sans
    /// thématique allumée, elle reste celle d'avant l'étape 3b.
    pub fn empreinte(&self) -> String {
        let allumees: Vec<String> = self.notify.iter().map(|c| format!("!{c}")).collect();
        empreinte_des_codes(
            self.themes
                .iter()
                .map(|t| t.code.as_str())
                .chain(allumees.iter().map(String::as_str)),
        )
    }
}

/// `ThemesPayload` — la liste **entière** des codes suivis, jamais un delta.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemesPayload {
    pub codes: Vec<String>,
}

/// Codes triés, dédoublonnés, joints.
pub fn empreinte_des_codes<'a>(codes: impl IntoIterator<Item = &'a str>) -> String {
    let mut codes: Vec<&str> = codes.into_iter().collect();
    codes.sort_unstable();
    codes.dedup();
    kernel::empreinte::de(&codes.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::empreinte_des_codes;

    #[test]
    fn lordre_et_les_doublons_ne_changent_pas_lempreinte() {
        let a = empreinte_des_codes(["gender", "adaptation"]);
        let b = empreinte_des_codes(["adaptation", "gender", "adaptation"]);
        assert_eq!(a, b);
        assert_ne!(a, empreinte_des_codes(["adaptation"]));
        assert!(a.starts_with('"') && a.ends_with('"') && a.len() == 34);
    }

    #[test]
    fn lempreinte_change_avec_notify() {
        use super::{FollowedTheme, MyThemes};
        let mes = |notify: &[&str]| MyThemes {
            themes: vec![FollowedTheme {
                code: "finance".into(),
                followed_at: time::OffsetDateTime::UNIX_EPOCH,
            }],
            notify: notify.iter().map(|c| (*c).to_owned()).collect(),
        };
        assert_eq!(mes(&[]).empreinte(), empreinte_des_codes(["finance"]));
        assert_ne!(mes(&["finance"]).empreinte(), mes(&[]).empreinte());
    }
}
