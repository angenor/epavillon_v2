//! La forme lisible, telle que la décrit `contracts/forme-lisible.md`.

use serde::Serialize;

#[derive(Serialize)]
pub struct DocumentReading {
    pub id: String,
    pub version: String,
    pub mode: &'static str,
    pub page_count: usize,
    pub outline: Vec<OutlineEntry>,
    pub pages: Vec<ReadingPage>,
}

#[derive(Serialize, Clone, Debug)]
pub struct OutlineEntry {
    pub title: String,
    pub level: u8,
    pub page_index: usize,
    pub children: Vec<OutlineEntry>,
}

#[derive(Serialize)]
pub struct ReadingPage {
    pub index: usize,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub blocks: Vec<Block>,
}

#[derive(Serialize, Clone, Debug)]
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
    /// `text` est une extension proposée par l'essai : le texte de la zone,
    /// pour qu'un tableau reste cherchable (voir la conclusion).
    Origin {
        reason: &'static str,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        text: Vec<Span>,
    },
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Span {
    pub text: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub term: bool,
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
