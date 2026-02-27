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
                    "Gene" => "badge-success",
                    "Disease" => "badge-danger",
                    "Chemical" => "badge-primary",
                    _ => "badge-outline",
                },
                e.entity_type, e.text, e.confidence
            )
        }).collect::<String>();
        
        format!(
            r#"
            <div class="result-section card mt-4">
                <div class="card-header">Recognized Entities Timeline</div>
                <div class="card-body">
                    <div class="highlighted-text">{}</div>
                </div>
                
                <div class="card-header mt-4 text-muted border-top border-glass" style="font-size:0.9rem;">Entity Extraction Catalog ({} items):</div>
                <div class="table-container p-0">
                    <table class="table">
                        <thead><tr><th>Classification</th><th>Recognized Substring</th><th>Model Confidence</th></tr></thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
            </div>
            "#,
            highlighted, r.entities.len(), entity_list
        )
    }).unwrap_or_default();
    
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>NER Engine — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.1">
    <style>
        .highlighted-text {{ 
            background: var(--bg-surface); 
            padding: 1.5rem; 
            border-radius: 8px; 
            line-height: 2.2;
            font-size: 1.05rem;
            color: var(--text-main);
            border: 1px solid var(--border-glass);
            font-family: 'Inter', sans-serif;
        }}
        .entity-gene {{ background: rgba(16, 185, 129, 0.2); color: #10b981; padding: 2px 6px; border-radius: 6px; font-weight: 600; border: 1px solid rgba(16, 185, 129, 0.4); }}
        .entity-disease {{ background: rgba(239, 68, 68, 0.2); color: #ef4444; padding: 2px 6px; border-radius: 6px; font-weight: 600; border: 1px solid rgba(239, 68, 68, 0.4); }}
        .entity-chemical {{ background: rgba(59, 130, 246, 0.2); color: #3b82f6; padding: 2px 6px; border-radius: 6px; font-weight: 600; border: 1px solid rgba(59, 130, 246, 0.4); }}
        .entity-other {{ background: rgba(156, 163, 175, 0.2); color: #9ca3af; padding: 2px 6px; border-radius: 6px; border: 1px solid rgba(156, 163, 175, 0.4); }}
        textarea {{ font-family: 'Outfit', sans-serif; font-size: 1.1rem; }}
        .example-btn {{ background: var(--bg-surface); border: 1px solid var(--border-glass); color: var(--text-muted); cursor: pointer; border-radius: 6px; padding: 0.5rem; transition: var(--transition-fast); text-align: left; width: 100%; }}
        .example-btn:hover {{ border-color: var(--brand-blue); color: var(--text-main); }}
    </style>
</head>
<body>
<div class="app-container">
    {}
    <main class="main-content">
        <div class="page-header">
            <div>
                <h1 class="page-title">
                    <svg width="20" height="20" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
                    Named Entity Recognition Engine
                </h1>
                <p class="text-muted">High-performance extraction of gene, disease, and chemical entities from unstructured scientific literature</p>
            </div>
        </div>
        
        <div class="grid-2 align-start" style="grid-template-columns: 300px 1fr; gap: 2rem;">
            <div class="d-flex flex-column gap-3">
                <div class="card">
                    <div class="card-header text-muted">Knowledge Base Stats</div>
                    <div class="card-body d-flex flex-column gap-2" style="font-size: 0.95rem;">
                        <div class="d-flex justify-between"><span>Gene Vocabulary:</span> <strong style="color:var(--text-main)">{}</strong></div>
                        <div class="d-flex justify-between"><span>Disease Tokens:</span> <strong style="color:var(--text-main)">{}</strong></div>
                        <div class="d-flex justify-between"><span>Chemical Graphs:</span> <strong style="color:var(--text-main)">{}</strong></div>
                        <div class="d-flex justify-between mt-2 pt-2 border-top border-glass"><span>Total Pre-computed Patterns:</span> <strong class="text-gradient">{}</strong></div>
                    </div>
                </div>
                
                <div class="card">
                    <div class="card-header text-muted">Example Corpi</div>
                    <div class="card-body d-flex flex-column gap-2 p-3">
                        <button class="example-btn" onclick="setExample(0)">EGFR + NSCLC Dynamics</button>
                        <button class="example-btn" onclick="setExample(1)">KRAS + PDAC Progression</button>
                        <button class="example-btn" onclick="setExample(2)">BRCA + PARP Inhibitors</button>
                    </div>
                </div>
            </div>
            
            <div class="d-flex flex-column">
                <div class="card input-section">
                    <form method="POST" action="/ner/extract" class="d-flex flex-column gap-3">
                        <div>
                            <label class="form-label" style="font-size: 1.1rem; color: var(--text-main);">Input Document Stream</label>
                            <textarea name="text" id="textInput" class="form-control mt-2" rows="6" placeholder="Enter biomedical text to initiate entity extraction sequence...">{}</textarea>
                        </div>
                        <div class="d-flex justify-end">
                            <button type="submit" class="btn btn-primary px-4 py-2">Execute Extraction Algorithm</button>
                        </div>
                    </form>
                </div>
                
                {}
                
                {}
            </div>
        </div>
    </main>
    
    <script src="/static/js/main.js"></script>
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
</div>
</body>
</html>"##,
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
