"""
Ferrumyx SciSpacy NER Service
FastAPI server exposing biomedical named entity recognition via SciSpacy.

Endpoints:
  POST /ner       — Extract entities from text
  GET  /health    — Health check (model load status)
  GET  /models    — List loaded models
"""

import time
import logging
from typing import Optional
from functools import lru_cache

import spacy
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("ferrumyx.ner")

app = FastAPI(
    title="Ferrumyx NER Service",
    description="Biomedical NER via SciSpacy — supports en_core_sci_lg and en_ner_bc5cdr_md",
    version="0.1.0",
)

# ── Model registry ────────────────────────────────────────────────────────────

MODELS = {
    "sci_lg":  "en_core_sci_lg",
    "bc5cdr":  "en_ner_bc5cdr_md",
}

_loaded: dict[str, spacy.language.Language] = {}

def load_model(alias: str) -> spacy.language.Language:
    if alias not in _loaded:
        model_name = MODELS.get(alias)
        if not model_name:
            raise ValueError(f"Unknown model alias: {alias}. Available: {list(MODELS)}")
        logger.info(f"Loading model: {model_name}")
        _loaded[alias] = spacy.load(model_name)
        logger.info(f"Model loaded: {model_name}")
    return _loaded[alias]

# Pre-load default model on startup
@app.on_event("startup")
async def startup():
    try:
        load_model("sci_lg")
        load_model("bc5cdr")
        logger.info("All NER models loaded successfully.")
    except Exception as e:
        logger.error(f"Model load failed: {e}")

# ── Request / Response schemas ────────────────────────────────────────────────

class NerRequest(BaseModel):
    text: str = Field(..., max_length=50_000, description="Text to process")
    model: str = Field("sci_lg", description="Model alias: sci_lg | bc5cdr")

class EntitySpan(BaseModel):
    text: str
    label: str
    start: int
    end: int
    normalized_id: Optional[str] = None

class NerResponse(BaseModel):
    entities: list[EntitySpan]
    model: str
    elapsed_ms: float
    n_entities: int

# ── Endpoints ─────────────────────────────────────────────────────────────────

@app.post("/ner", response_model=NerResponse)
async def extract_entities(req: NerRequest) -> NerResponse:
    """Extract biomedical named entities from text."""
    t0 = time.perf_counter()

    try:
        nlp = load_model(req.model)
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))

    doc = nlp(req.text)

    entities = []
    for ent in doc.ents:
        # Try to get a normalized ID from entity linker (if loaded)
        normalized_id = None
        if hasattr(ent._, "kb_ents") and ent._.kb_ents:
            normalized_id = ent._.kb_ents[0][0]  # top UMLS CUI

        entities.append(EntitySpan(
            text=ent.text,
            label=ent.label_,
            start=ent.start_char,
            end=ent.end_char,
            normalized_id=normalized_id,
        ))

    elapsed_ms = (time.perf_counter() - t0) * 1000

    logger.info(f"NER [{req.model}] {len(entities)} entities in {elapsed_ms:.1f}ms "
                f"(text_len={len(req.text)})")

    return NerResponse(
        entities=entities,
        model=MODELS.get(req.model, req.model),
        elapsed_ms=round(elapsed_ms, 2),
        n_entities=len(entities),
    )


@app.get("/health")
async def health():
    """Health check — returns loaded model status."""
    return {
        "status": "ok",
        "loaded_models": list(_loaded.keys()),
        "available_models": list(MODELS.keys()),
    }


@app.get("/models")
async def models():
    """List available model aliases and their spacy model names."""
    return {
        "models": {alias: name for alias, name in MODELS.items()},
        "loaded": list(_loaded.keys()),
    }
