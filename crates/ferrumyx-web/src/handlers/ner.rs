//! NER entity extraction page — real-time entity recognition demo.

use axum::{
    extract::State,
    response::{Html, IntoResponse, Json},
    Form,
};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;
use ferrumyx_ner::trie_ner::TrieNer;

#[derive(Deserialize)]
pub struct NerForm {
    pub text: String,
}

#[derive(Serialize)]
pub struct NerResult {
    pub text: String,
    pub entities: Vec<EntityResult>,
}

#[derive(Serialize, Clone)]
pub struct EntityResult {
    pub text: String,
    pub entity_type: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
}

#[derive(Serialize)]
pub struct NerStats {
    pub gene_count: usize,
    pub disease_count: usize,
    pub chemical_count: usize,
    pub total_patterns: usize,
}

/// GET /ner — Show NER demo page
pub async fn ner_page(State(_state): State<SharedState>) -> Html<String> {
    Html(render_ner_page(None, None))
}

/// POST /ner/extract — Extract entities from text
pub async fn ner_extract(
    State(_state): State<SharedState>,
    Form(form): Form<NerForm>,
) -> Html<String> {
    let ner = TrieNer::with_embedded_subset();
    let extracted = ner.extract(&form.text);
    
    let entities: Vec<EntityResult> = extracted.into_iter().map(|e| EntityResult {
        text: e.text,
        entity_type: format!("{:?}", e.label),
        start: e.start,
        end: e.end,
        confidence: e.confidence,
    }).collect();
    
    let result = NerResult {
        text: form.text.clone(),
        entities,
    };
    
    Html(render_ner_page(Some(result), None))
}

/// GET /api/ner/stats — Get NER database stats
pub async fn api_ner_stats() -> impl IntoResponse {
    let ner = TrieNer::with_embedded_subset();
    let stats = ner.stats();
    
    Json(NerStats {
        gene_count: stats.gene_count,
        disease_count: stats.disease_count,
        chemical_count: stats.chemical_count,
        total_patterns: stats.total_patterns,
    })
}

/// POST /api/ner/extract — API endpoint for entity extraction
pub async fn api_ner_extract(
    Json(payload): Json<NerForm>,
) -> impl IntoResponse {
    let ner = TrieNer::with_embedded_subset();
    let extracted = ner.extract(&payload.text);
    
    let entities: Vec<EntityResult> = extracted.into_iter().map(|e| EntityResult {
        text: e.text,
        entity_type: format!("{:?}", e.label),
        start: e.start,
        end: e.end,
        confidence: e.confidence,
    }).collect();
    
    Json(NerResult {
        text: payload.text,
        entities,
    })
}

fn render_ner_page(result: Option<NerResult>, error: Option<String>) -> String {
    let ner = TrieNer::with_embedded_subset();
    let stats = ner.stats();
    
    let entity_html = result.as_ref().map(|r| {
        let mut highlighted = r.text.clone();
        let mut offset = 0i32;
        
        // Sort entities by start position
        let mut sorted_entities = r.entities.clone();
        sorted_entities.sort_by_key(|e| e.start);
        
        for entity in sorted_entities {
            let start = (entity.start as i32 + offset) as usize;
            let end = (entity.end as i32 + offset) as usize;
            
            let color_class = match entity.entity_type.as_str() {
                "Gene" => "entity-gene",
                "Disease" => "entity-disease",
                "Chemical" => "entity-chemical",
                _ => "entity-other",
            };
            
            let replacement = format!(
                r#"<span class="{}" title="{} (confidence: {:.2})">{}</span>"#,
                color_class, entity.entity_type, entity.confidence, &highlighted[start..end]
            );
            
            highlighted.replace_range(start..end, &replacement);
            offset += replacement.len() as i32 - (entity.end - entity.start) as i32;
        }
        
        let entity_list = r.entities.iter().map(|e| {
            format!(
                r#"<tr><td><span class="badge {}">{}</span></td><td>{}</td><td>{:.2}</td></tr>"#,
                match e.entity_type.as_str() {
                    "Gene" => "bg-success",
                    "Disease" => "bg-danger",
                    "Chemical" => "bg-primary",
                    _ => "bg-secondary",
                },
                e.entity_type, e.text, e.confidence
            )
        }).collect::<String>();
        
        format!(
            r#"
            <div class="result-section">
                <h4>Highlighted Text</h4>
                <div class="highlighted-text">{}</div>
                
                <h4>Entities Found ({}):</h4>
                <table class="table table-sm">
                    <thead><tr><th>Type</th><th>Text</th><th>Confidence</th></tr></thead>
                    <tbody>{}</tbody>
                </table>
            </div>
            "#,
            highlighted, r.entities.len(), entity_list
        )
    }).unwrap_or_default();
    
    format!(
        r##"
<!DOCTYPE html>
<html>
<head>
    <title>Ferrumyx — NER Demo</title>
    <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
    <style>
        body {{ padding: 2rem; background: #f8f9fa; }}
        .container {{ max-width: 1200px; }}
        .stats-card {{ background: white; border-radius: 8px; padding: 1rem; margin-bottom: 1rem; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .input-section {{ background: white; border-radius: 8px; padding: 1.5rem; margin-bottom: 1rem; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .result-section {{ background: white; border-radius: 8px; padding: 1.5rem; margin-top: 1rem; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .highlighted-text {{ 
            background: #f8f9fa; 
            padding: 1rem; 
            border-radius: 4px; 
            line-height: 1.8;
            font-size: 1.1rem;
        }}
        .entity-gene {{ background: #d4edda; padding: 2px 4px; border-radius: 3px; font-weight: bold; }}
        .entity-disease {{ background: #f8d7da; padding: 2px 4px; border-radius: 3px; font-weight: bold; }}
        .entity-chemical {{ background: #cce5ff; padding: 2px 4px; border-radius: 3px; font-weight: bold; }}
        .entity-other {{ background: #e2e3e5; padding: 2px 4px; border-radius: 3px; }}
        textarea {{ font-family: monospace; }}
    </style>
</head>
<body>
    {}
    <div class="container">
        <h2>🔍 Named Entity Recognition Demo</h2>
        <p class="text-muted">Extract genes, diseases, and chemicals using dictionary-based NER</p>
        
        <div class="row">
            <div class="col-md-3">
                <div class="stats-card">
                    <h5>Database Stats</h5>
                    <div class="d-flex justify-content-between"><span>Genes:</span> <strong>{}</strong></div>
                    <div class="d-flex justify-content-between"><span>Diseases:</span> <strong>{}</strong></div>
                    <div class="d-flex justify-content-between"><span>Chemicals:</span> <strong>{}</strong></div>
                    <div class="d-flex justify-content-between"><span>Patterns:</span> <strong>{}</strong></div>
                </div>
                
                <div class="stats-card">
                    <h5>Example Texts</h5>
                    <button class="btn btn-sm btn-outline-primary w-100 mb-2" onclick="setExample(0)">EGFR + NSCLC</button>
                    <button class="btn btn-sm btn-outline-primary w-100 mb-2" onclick="setExample(1)">KRAS + PDAC</button>
                    <button class="btn btn-sm btn-outline-primary w-100" onclick="setExample(2)">BRCA + Ovarian</button>
                </div>
            </div>
            
            <div class="col-md-9">
                <div class="input-section">
                    <form method="POST" action="/ner/extract">
                        <div class="mb-3">
                            <label class="form-label">Input Text</label>
                            <textarea name="text" id="textInput" class="form-control" rows="6" placeholder="Enter biomedical text to extract entities...">{}</textarea>
                        </div>
                        <button type="submit" class="btn btn-primary">🔍 Extract Entities</button>
                    </form>
                </div>
                
                {}
                
                {}
            </div>
        </div>
    </div>
    
    <script>
        const examples = [
            "EGFR T790M mutations confer resistance to gefitinib and erlotinib in non-small cell lung cancer patients. Osimertinib is a third-generation EGFR TKI effective against T790M-positive NSCLC.",
            "KRAS G12D mutations drive pancreatic adenocarcinoma progression. Combination therapy with MEK inhibitors and chemotherapy shows promise in preclinical models.",
            "BRCA1 and BRCA2 mutations increase risk of ovarian cancer. PARP inhibitors like olaparib are effective in BRCA-mutant tumors."
        ];
        
        function setExample(index) {{
            document.getElementById('textInput').value = examples[index];
        }}
    </script>
</body>
</html>
        "##,
        NAV_HTML,
        stats.gene_count,
        stats.disease_count,
        stats.chemical_count,
        stats.total_patterns,
        result.as_ref().map(|r| r.text.clone()).unwrap_or_default(),
        entity_html,
        error.map(|e| format!(r#"<div class="alert alert-danger">{}</div>"#, e)).unwrap_or_default()
    )
}
