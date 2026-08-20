//! Négociation de la locale contre `reference.locales`, repli sur le français.
//!
//! Aucune résolution de texte du modèle côté Rust : la locale négociée est
//! passée à `platform.t()` dans les requêtes, jamais recopiée dans un fichier
//! de traduction.

use sqlx::PgPool;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Locales {
    codes: Vec<String>,
    default: String,
}

impl Locales {
    pub async fn load(pool: &PgPool) -> Result<Self> {
        let rows = sqlx::query!(
            "SELECT code, is_default FROM reference.locales WHERE is_active ORDER BY sort_order"
        )
        .fetch_all(pool)
        .await?;

        let default = rows
            .iter()
            .find(|r| r.is_default)
            .map(|r| r.code.clone())
            .unwrap_or_else(|| "fr".to_owned());

        Ok(Self {
            codes: rows.into_iter().map(|r| r.code).collect(),
            default,
        })
    }

    pub fn default_code(&self) -> &str {
        &self.default
    }

    pub fn codes(&self) -> &[String] {
        &self.codes
    }

    /// `Accept-Language: en-GB;q=0.9, fr;q=0.8` → la première locale active,
    /// par ordre de préférence décroissante ; le français sinon.
    ///
    /// `q=0` est un REFUS (RFC 7231 § 5.3.5), pas une préférence faible : la
    /// langue est écartée. Un facteur illisible écarte l'étiquette entière
    /// plutôt que de la promouvoir à la priorité maximale.
    pub fn negotiate(&self, accept_language: Option<&str>) -> String {
        let Some(header) = accept_language else {
            return self.default.clone();
        };

        let mut souhaits: Vec<(f32, &str)> = header
            .split(',')
            .filter_map(|part| {
                let mut morceaux = part.split(';');
                let etiquette = morceaux.next()?.trim();
                if etiquette.is_empty() || etiquette == "*" {
                    return None;
                }
                let q = match morceaux.find(|m| m.trim().starts_with("q=")) {
                    Some(m) => m
                        .trim()
                        .strip_prefix("q=")?
                        .parse::<f32>()
                        .ok()
                        .filter(|q| (0.0..=1.0).contains(q))?,
                    None => 1.0,
                };
                if q == 0.0 {
                    return None;
                }
                Some((q, etiquette))
            })
            .collect();
        souhaits.sort_by(|a, b| b.0.total_cmp(&a.0));

        for (_, etiquette) in souhaits {
            if let Some(code) = self.resolve(etiquette) {
                return code;
            }
        }
        self.default.clone()
    }

    fn resolve(&self, etiquette: &str) -> Option<String> {
        let exact = self
            .codes
            .iter()
            .find(|c| c.eq_ignore_ascii_case(etiquette));
        if let Some(code) = exact {
            return Some(code.clone());
        }
        let primaire = etiquette.split('-').next()?;
        self.codes
            .iter()
            .find(|c| c.eq_ignore_ascii_case(primaire))
            .cloned()
    }
}
