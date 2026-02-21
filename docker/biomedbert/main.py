"""
Ferrumyx BiomedBERT Embedding Service
Batch text embedding with microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract.

Endpoints:
  POST /embed              — Embed a list of texts; return 768-dim vectors
  POST /embed_and_store    — Embed + UPDATE paper_chunks.embedding in Postgres
  GET  /health             — Health + model load status
"""

import os
import time
import logging
from typing import Optional

import numpy as np
import torch
import psycopg2
from pgvector.psycopg2 import register_vector
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field
from transformers import AutoTokenizer, AutoModel

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("ferrumyx.embed")

MODEL_NAME  = os.getenv("MODEL_NAME",  "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract")
BATCH_SIZE  = int(os.getenv("BATCH_SIZE", "32"))
MAX_SEQ_LEN = int(os.getenv("MAX_SEQ_LEN", "512"))
DB_URL      = os.getenv("DATABASE_URL", "postgresql://ferrumyx:ferrumyx_dev@postgres:5432/ferrumyx")

app = FastAPI(
    title="Ferrumyx Embedding Service",
    description="BiomedBERT-base batch embedding for paper chunks and entities",
    version="0.1.0",
)

# ── Model loading ─────────────────────────────────────────────────────────────

tokenizer: Optional[AutoTokenizer] = None
model: Optional[AutoModel] = None
device = "cuda" if torch.cuda.is_available() else "cpu"

@app.on_event("startup")
async def startup():
    global tokenizer, model
    logger.info(f"Loading BiomedBERT from {MODEL_NAME} on {device}...")
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
    model = AutoModel.from_pretrained(MODEL_NAME)
    model.eval()
    model.to(device)
    logger.info(f"BiomedBERT loaded (dim=768, device={device})")

# ── Embedding core ────────────────────────────────────────────────────────────

def mean_pool(token_embeddings: torch.Tensor, attention_mask: torch.Tensor) -> np.ndarray:
    """Mean pooling over non-padding tokens."""
    mask_expanded = attention_mask.unsqueeze(-1).expand(token_embeddings.size()).float()
    summed = torch.sum(token_embeddings * mask_expanded, dim=1)
    counts = torch.clamp(mask_expanded.sum(dim=1), min=1e-9)
    return (summed / counts).detach().cpu().numpy()

def embed_texts(texts: list[str]) -> np.ndarray:
    """Embed a list of texts into (N, 768) float32 numpy array."""
    all_embeddings = []
    for i in range(0, len(texts), BATCH_SIZE):
        batch = texts[i : i + BATCH_SIZE]
        encoded = tokenizer(
            batch,
            padding=True,
            truncation=True,
            max_length=MAX_SEQ_LEN,
            return_tensors="pt",
        )
        encoded = {k: v.to(device) for k, v in encoded.items()}
        with torch.no_grad():
            output = model(**encoded)
        vecs = mean_pool(output.last_hidden_state, encoded["attention_mask"])
        all_embeddings.append(vecs)
    return np.vstack(all_embeddings).astype(np.float32)

# ── Request / Response ────────────────────────────────────────────────────────

class EmbedRequest(BaseModel):
    texts: list[str] = Field(..., max_length=512, description="List of texts to embed")
    normalize: bool = Field(True, description="L2-normalize embeddings")

class EmbedResponse(BaseModel):
    embeddings: list[list[float]]
    model: str
    dim: int
    elapsed_ms: float
    n_texts: int

class EmbedAndStoreRequest(BaseModel):
    chunk_ids: list[str] = Field(..., description="List of paper_chunks UUIDs")
    texts: list[str] = Field(..., description="Corresponding chunk texts (same order)")
    normalize: bool = True

# ── Endpoints ─────────────────────────────────────────────────────────────────

@app.post("/embed", response_model=EmbedResponse)
async def embed(req: EmbedRequest) -> EmbedResponse:
    """Embed texts and return vectors."""
    if not model:
        raise HTTPException(503, "Model not loaded yet")
    if len(req.texts) == 0:
        raise HTTPException(400, "texts cannot be empty")

    t0 = time.perf_counter()
    vecs = embed_texts(req.texts)

    if req.normalize:
        norms = np.linalg.norm(vecs, axis=1, keepdims=True)
        norms = np.where(norms == 0, 1.0, norms)
        vecs = vecs / norms

    elapsed_ms = (time.perf_counter() - t0) * 1000
    logger.info(f"Embedded {len(req.texts)} texts in {elapsed_ms:.1f}ms")

    return EmbedResponse(
        embeddings=vecs.tolist(),
        model=MODEL_NAME,
        dim=vecs.shape[1],
        elapsed_ms=round(elapsed_ms, 2),
        n_texts=len(req.texts),
    )


@app.post("/embed_and_store")
async def embed_and_store(req: EmbedAndStoreRequest):
    """Embed texts and write vectors to paper_chunks.embedding in Postgres."""
    if not model:
        raise HTTPException(503, "Model not loaded yet")
    if len(req.chunk_ids) != len(req.texts):
        raise HTTPException(400, "chunk_ids and texts must have the same length")

    t0 = time.perf_counter()
    vecs = embed_texts(req.texts)

    if req.normalize:
        norms = np.linalg.norm(vecs, axis=1, keepdims=True)
        norms = np.where(norms == 0, 1.0, norms)
        vecs = vecs / norms

    # Write to PostgreSQL
    conn = psycopg2.connect(DB_URL)
    register_vector(conn)
    cur = conn.cursor()
    updated = 0

    try:
        for chunk_id, vec in zip(req.chunk_ids, vecs):
            cur.execute(
                "UPDATE paper_chunks SET embedding = %s WHERE id = %s",
                (vec, chunk_id),
            )
            updated += cur.rowcount
        conn.commit()
    finally:
        cur.close()
        conn.close()

    elapsed_ms = (time.perf_counter() - t0) * 1000
    logger.info(f"embed_and_store: {updated} chunks updated in {elapsed_ms:.1f}ms")

    return {
        "status": "ok",
        "updated": updated,
        "elapsed_ms": round(elapsed_ms, 2),
        "model": MODEL_NAME,
    }


@app.get("/health")
async def health():
    return {
        "status": "ok" if model else "loading",
        "model": MODEL_NAME,
        "device": device,
        "batch_size": BATCH_SIZE,
        "max_seq_len": MAX_SEQ_LEN,
    }
