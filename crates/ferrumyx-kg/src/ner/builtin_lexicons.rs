/// Built-in lexicon hints for chemical compounds, pathways, and cell lines.
///
/// These constants serve as the single source of truth for built-in lexicon terms
/// used by both the extraction module and the NER (trie-based) module to prevent
/// maintenance drift between duplicate definitions.
///
/// The terms are lowercase to match the expected input format in both modules.
pub const CHEMICAL_HINTS: &[&str] = &[
    "erlotinib",
    "gefitinib",
    "osimertinib",
    "afatinib",
    "sotorasib",
    "adagrasib",
    "trametinib",
    "selumetinib",
    "imatinib",
    "dasatinib",
    "nilotinib",
    "palbociclib",
    "ribociclib",
    "abemaciclib",
    "olaparib",
    "niraparib",
    "talazoparib",
    "cisplatin",
    "carboplatin",
    "paclitaxel",
    "docetaxel",
    "bevacizumab",
];

/// Built-in short lexicon for common pathways.
pub const PATHWAY_HINTS: &[&str] = &[
    "mapk pathway",
    "pi3k pathway",
    "jak stat pathway",
    "m tor pathway",
    "wnt pathway",
    "nf kb pathway",
    "tgf beta pathway",
    "hedgehog pathway",
    "ras pathway",
    "erk signaling pathway",
    "akt signaling pathway",
    "egfr signaling pathway",
    "vegf pathway",
];

/// Built-in short lexicon for common cell lines.
pub const CELL_LINE_HINTS: &[&str] = &[
    "hela",
    "hek293",
    "a549",
    "h1975",
    "pc9",
    "panc1",
    "mia paca 2",
    "bxpc3",
    "ht29",
    "hct116",
];
