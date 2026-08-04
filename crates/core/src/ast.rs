//! Renderer-facing AST. Produced per block by `parse::parse_block`; consumed by
//! display adapters. Deliberately small and stable — this is the core/display seam.

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    SoftBreak,
    HardBreak,
    Code(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link {
        text: Vec<Inline>,
        url: String,
    },
    Image {
        alt: String,
        url: String,
    },
    /// Math content; `display == true` for block math.
    Math(String, bool),
    Html(String),
    FootnoteRef(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// `Some(checked)` for task-list items.
    pub checked: Option<bool>,
    pub children: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        children: Vec<Inline>,
    },
    Code {
        lang: String,
        text: String,
    },
    BlockQuote(Vec<Block>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<ListItem>,
    },
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
        aligns: Vec<Alignment>,
    },
    ThematicBreak,
    Html(String),
    FootnoteDefinition {
        label: String,
        children: Vec<Block>,
    },
}

/// One parsed block: the AST of a single markdown block string.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ast {
    pub children: Vec<Block>,
}

impl Ast {
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}
