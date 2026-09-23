//! Ce que la lecture du PDF rend, avant toute règle : des segments de texte avec
//! leur police et leur cadre, les objets graphiques, les signets. Rien ici ne
//! dépend de PDFium : les règles s'éprouvent sur des pages construites à la main.

/// Un rectangle, l'origine en haut à gauche, comme on lit.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cadre {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Cadre {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn largeur(&self) -> f32 {
        self.x1 - self.x0
    }

    pub fn hauteur(&self) -> f32 {
        self.y1 - self.y0
    }

    pub fn unir(&mut self, autre: &Cadre) {
        self.x0 = self.x0.min(autre.x0);
        self.y0 = self.y0.min(autre.y0);
        self.x1 = self.x1.max(autre.x1);
        self.y1 = self.y1.max(autre.y1);
    }

    /// Les deux cadres se touchent, à `marge` près.
    pub fn proche(&self, autre: &Cadre, marge: f32) -> bool {
        self.x0 - marge <= autre.x1
            && autre.x0 - marge <= self.x1
            && self.y0 - marge <= autre.y1
            && autre.y0 - marge <= self.y1
    }

    pub fn contient(&self, x: f32, y: f32, marge: f32) -> bool {
        x >= self.x0 - marge && x <= self.x1 + marge && y >= self.y0 - marge && y <= self.y1 + marge
    }
}

/// Des caractères consécutifs de même police, même corps et même ligne de base.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub texte: String,
    pub police: String,
    pub italique: bool,
    pub taille: f32,
    /// Ligne de base, depuis le haut de la page.
    pub base: f32,
    pub cadre: Cadre,
    /// Le segment finit sur un saut de ligne engendré par PDFium.
    pub fin_de_ligne: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenreObjet {
    Chemin,
    Image,
    Forme,
    Degrade,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Objet {
    pub genre: GenreObjet,
    pub cadre: Cadre,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageBrute {
    /// À partir de 1, dans l'ordre du fichier.
    pub indice: usize,
    /// L'étiquette que le PDF déclare, s'il en déclare.
    pub etiquette: Option<String>,
    pub largeur: f32,
    pub hauteur: f32,
    pub segments: Vec<Segment>,
    pub objets: Vec<Objet>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Signet {
    pub titre: String,
    pub niveau: u8,
    pub page_indice: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentBrut {
    pub pages: Vec<PageBrute>,
    pub signets: Vec<Signet>,
}
