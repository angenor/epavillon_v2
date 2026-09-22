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
}

impl MyThemes {
    /// L'empreinte de **l'état**, pas de sa représentation.
    pub fn empreinte(&self) -> String {
        empreinte_des_codes(self.themes.iter().map(|t| t.code.as_str()))
    }
}

/// `ThemesPayload` — la liste **entière** des codes suivis, jamais un delta.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemesPayload {
    pub codes: Vec<String>,
}

/// Codes triés, dédoublonnés, joints ; 16 octets de `token_hash` en
/// hexadécimal, entre guillemets comme l'exige l'en-tête `ETag`.
pub fn empreinte_des_codes<'a>(codes: impl IntoIterator<Item = &'a str>) -> String {
    let mut codes: Vec<&str> = codes.into_iter().collect();
    codes.sort_unstable();
    codes.dedup();
    let octets = kernel::crypto::token_hash(&codes.join("\n"));
    let mut hexa = String::with_capacity(34);
    hexa.push('"');
    for octet in &octets[..16] {
        hexa.push_str(&format!("{octet:02x}"));
    }
    hexa.push('"');
    hexa
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
}
