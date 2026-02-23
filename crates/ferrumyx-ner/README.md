# ferrumyx-ner

Biomedical Named Entity Recognition using Candle (Rust-native, no Python/Docker dependencies).

## Features

- **Pure Rust** - No Python, no Docker, no external dependencies
- **GPU Acceleration** - CUDA support when available, CPU fallback
- **Multiple Architectures** - BERT, RoBERTa, XLM-RoBERTa, DeBERTa-v2
- **OpenMed Models** - 380+ specialized biomedical NER models
- **BIO Tagging** - Proper entity span extraction with confidence scores

## Supported Models

### Disease Detection
| Config | Model | F1 Score | Dataset |
|--------|-------|----------|---------|
| `NerConfig::diseases()` | OpenMed-NER-DiseaseDetect-BioMed-335M | 0.900 | BC5CDR-Disease |
| `NerConfig::diseases_large()` | OpenMed-NER-DiseaseDetect-SuperClinical-434M | 0.912 | BC5CDR-Disease |

### Pharmaceuticals & Chemicals
| Config | Model | F1 Score | Dataset |
|--------|-------|----------|---------|
| `NerConfig::pharmaceuticals()` | OpenMed-NER-PharmaDetect-SuperClinical-434M | 0.961 | BC5CDR-Chem |
| `NerConfig::chemicals()` | OpenMed-NER-ChemicalDetect-PubMed-335M | 0.954 | BC4CHEMD |

### Genomics & Genetics
| Config | Model | F1 Score | Dataset |
|--------|-------|----------|---------|
| `NerConfig::genomic()` | OpenMed-NER-GenomicDetect-SnowMed-568M | 0.998 | Gellus |
| `NerConfig::genome()` | OpenMed-NER-GenomeDetect-SuperClinical-434M | 0.901 | BC2GM |
| `NerConfig::proteins()` | OpenMed-NER-ProteinDetect-SnowMed-568M | 0.961 | FSU |
| `NerConfig::dna_proteins()` | OpenMed-NER-DNADetect-SuperClinical-434M | 0.819 | JNLPBA |

### Oncology
| Config | Model | F1 Score | Dataset |
|--------|-------|----------|---------|
| `NerConfig::oncology()` | OpenMed-NER-OncologyDetect-SuperMedical-355M | 0.899 | BioNLP 2013 CG |

### Anatomy & Species
| Config | Model | F1 Score | Dataset |
|--------|-------|----------|---------|
| `NerConfig::anatomy()` | OpenMed-NER-AnatomyDetect-ElectraMed-560M | 0.906 | AnatEM |
| `NerConfig::species()` | OpenMed-NER-SpeciesDetect-PubMed-335M | 0.965 | Linnaeus |

## Quick Start

```rust
use ferrumyx_ner::{NerModel, NerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load a disease NER model
    let model = NerModel::new(NerConfig::diseases()).await?;
    
    // Extract entities
    let entities = model.extract("Patient diagnosed with diabetes mellitus.")?;
    
    for entity in entities {
        println!("{}: '{}' (score: {:.2})", entity.label, entity.text, entity.score);
    }
    
    Ok(())
}
```

## Running Multiple Models in Parallel

```rust
use ferrumyx_ner::{NerModel, NerConfig};
use futures::future::join_all;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let configs = vec![
        NerConfig::diseases(),
        NerConfig::pharmaceuticals(),
        NerConfig::genomic(),
        NerConfig::oncology(),
    ];
    
    // Load all models in parallel
    let models: Vec<_> = join_all(
        configs.into_iter().map(|c| NerModel::new(c))
    ).await;
    
    // Extract entities with each model
    let text = "BRCA1 mutations increase breast cancer risk. Treatment with erlotinib showed efficacy.";
    
    for model in models.iter().filter_map(|m| m.as_ref().ok()) {
        let entities = model.extract(text)?;
        println!("Model: {}", model.model_id());
        for e in entities {
            println!("  {}: '{}' [score: {:.2}]", e.label, e.text, e.score);
        }
    }
    
    Ok(())
}
```

## Architecture Support

| Architecture | Implementation | Notes |
|--------------|----------------|-------|
| BERT | `bert::BertModel` | Most OpenMed models |
| RoBERTa | `bert::BertModel` | Uses BERT with different tokenizer |
| XLM-RoBERTa | `bert::BertModel` | Multilingual, uses BERT architecture |
| DeBERTa-v2 | `debertav2::DebertaV2NERModel` | Built-in NER support |

## Entity Types

The crate provides standardized entity types:

- `Disease` - Diseases, conditions, symptoms
- `Chemical` - Drugs, chemicals, compounds
- `Gene` - Genes, proteins, DNA sequences
- `Species` - Organisms, species names
- `Anatomy` - Body parts, tissues
- `Cancer` - Cancer types, tumor classifications
- `Other` - Other entity types

## Performance

Models are downloaded once and cached locally by Hugging Face Hub. First run includes download time (typically 500MB-1GB per model).

GPU acceleration is automatic when CUDA is available. CPU inference is also supported.

## Example Output

```
Text: The patient was diagnosed with non-small cell lung carcinoma.
Found 1 entities
  DISEASE: 'non-small cell lung carcinoma' [score: 0.96]

Text: BRCA1 mutations increase breast cancer risk.
Found 2 entities
  GENE: 'BRCA1' [score: 0.98]
  DISEASE: 'breast cancer' [score: 0.94]
```

## License

MIT
