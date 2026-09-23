//! La forme lisible, telle que la décrit `contracts/forme-lisible.md` : une
//! grammaire close, sans HTML ni style libre. Elle se garde en base page par
//! page et se sert telle quelle au téléphone.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutlineEntry {
    pub title: String,
    pub level: u8,
    pub page_index: usize,
    pub children: Vec<OutlineEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    Heading {
        level: u8,
        spans: Vec<Span>,
    },
    Paragraph {
        spans: Vec<Span>,
    },
    ListItem {
        spans: Vec<Span>,
        depth: u8,
        marker: String,
    },
    Note {
        spans: Vec<Span>,
        mark: String,
    },
    /// Un tableau ou une figure : le lecteur renvoie à l'image de la page, et
    /// garde le texte de la zone, replié, pour qu'il reste cherchable.
    Origin {
        reason: OriginReason,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        text: Vec<Span>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginReason {
    Table,
    Figure,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    /// Un italique retenu comme terme anglais touchable ; implique `italic`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub term: bool,
}

impl Span {
    pub fn simple(texte: impl Into<String>) -> Self {
        Self {
            text: texte.into(),
            ..Self::default()
        }
    }
}

impl Block {
    pub fn spans(&self) -> &[Span] {
        match self {
            Block::Heading { spans, .. }
            | Block::Paragraph { spans }
            | Block::ListItem { spans, .. }
            | Block::Note { spans, .. } => spans,
            Block::Origin { text, .. } => text,
        }
    }

    pub fn texte(&self) -> String {
        self.spans().iter().map(|s| s.text.as_str()).collect()
    }
}

/// Le texte d'une page, blocs `note` et texte des blocs `origin` compris :
/// c'est lui que la recherche indexe, et rien ne se cherche qui ne s'affiche pas.
pub fn texte_de_page(blocs: &[Block]) -> String {
    blocs
        .iter()
        .map(Block::texte)
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_bloc_se_serialise_comme_le_contrat() {
        let bloc = Block::ListItem {
            spans: vec![
                Span::simple("Le "),
                Span {
                    text: "Global Stocktake".into(),
                    italic: true,
                    term: true,
                    ..Span::default()
                },
            ],
            depth: 0,
            marker: "•".into(),
        };
        let json = serde_json::to_value(&bloc).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "kind": "list_item",
                "spans": [{"text": "Le "}, {"text": "Global Stocktake", "italic": true, "term": true}],
                "depth": 0,
                "marker": "•"
            })
        );
        assert_eq!(serde_json::from_value::<Block>(json).unwrap(), bloc);
    }

    #[test]
    fn une_origine_sans_texte_nen_porte_pas() {
        let json = serde_json::to_value(Block::Origin {
            reason: OriginReason::Table,
            text: vec![],
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "origin", "reason": "table"})
        );
    }
}
