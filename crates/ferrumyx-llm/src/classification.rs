//! Data classification for LLM routing.
//! See ARCHITECTURE.md §8.2 and §8.3

/// Data classification levels for prompt content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataClass {
    /// Published literature, public database records.
    Public,
    /// Ferrumyx-generated scores, hypotheses, KG facts.
    Internal,
    /// Proprietary or unpublished experimental data.
    Confidential,
}

impl DataClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataClass::Public       => "PUBLIC",
            DataClass::Internal     => "INTERNAL",
            DataClass::Confidential => "CONFIDENTIAL",
        }
    }
}

/// Scans prompt content and returns the highest data class detected.
pub struct DataClassifier {
    /// Regex-like patterns that indicate INTERNAL content.
    internal_patterns: Vec<String>,
    /// Patterns that indicate CONFIDENTIAL content.
    confidential_patterns: Vec<String>,
}

impl Default for DataClassifier {
    fn default() -> Self {
        Self {
            internal_patterns: vec![
                // SMILES strings (rough pattern: contains ring notation, bonds)
                r"[A-Za-z0-9@\[\]=#/\\+\-()%]{15,}".to_string(),
                // Composite score values
                "composite_score".to_string(),
                "S_adj".to_string(),
                "target_score".to_string(),
            ],
            confidential_patterns: vec![
                "CONFIDENTIAL".to_string(),
                "proprietary_assay".to_string(),
                "unpublished".to_string(),
            ],
        }
    }
}

impl DataClassifier {
    pub fn classify(&self, prompt: &str) -> DataClass {
        // Check confidential first (highest priority)
        for pattern in &self.confidential_patterns {
            if prompt.to_lowercase().contains(&pattern.to_lowercase()) {
                return DataClass::Confidential;
            }
        }

        // Check internal
        for pattern in &self.internal_patterns {
            if prompt.contains(pattern.as_str()) {
                return DataClass::Internal;
            }
        }

        DataClass::Public
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_classification() {
        let clf = DataClassifier::default();
        let prompt = "What is KRAS G12D and why is it important in pancreatic cancer?";
        assert_eq!(clf.classify(prompt), DataClass::Public);
    }

    #[test]
    fn test_confidential_classification() {
        let clf = DataClassifier::default();
        let prompt = "Analyse this CONFIDENTIAL assay result: IC50 = 12nM";
        assert_eq!(clf.classify(prompt), DataClass::Confidential);
    }
}
