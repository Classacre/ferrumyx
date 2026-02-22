"""
Docling service for PDF/XML parsing with section-aware chunking.
Provides REST API for document processing.
"""

from fastapi import FastAPI, File, UploadFile, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from typing import Optional
import tempfile
import os
import logging

from docling.document_converter import DocumentConverter
from docling.chunking import HybridChunker

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = FastAPI(
    title="Docling Document Parser",
    description="Section-aware PDF/XML parsing with hybrid chunking",
    version="1.0.0"
)

# Initialize converter (lazy load on first request)
_converter = None
_chunker = None


def get_converter():
    global _converter
    if _converter is None:
        _converter = DocumentConverter()
    return _converter


def get_chunker():
    global _chunker
    if _chunker is None:
        _chunker = HybridChunker(
            max_tokens=512,
            merge_peers=True
        )
    return _chunker


class ParsedDocument(BaseModel):
    """Parsed document with sections and chunks."""
    filename: str
    title: Optional[str] = None
    sections: list[dict]
    chunks: list[dict]
    full_text: str
    metadata: dict


class ChunkResult(BaseModel):
    """Single chunk with context."""
    chunk_id: int
    text: str
    contextualized: str
    section: Optional[str] = None
    page: Optional[int] = None
    token_count: int


@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "healthy", "service": "docling"}


@app.post("/parse", response_model=ParsedDocument)
async def parse_document(file: UploadFile = File(...)):
    """
    Parse a PDF or XML document and return structured content.
    
    Supports:
    - PDF files (with OCR if needed)
    - XML/HTML files
    - DOCX files
    """
    # Validate file type
    allowed_types = [".pdf", ".xml", ".html", ".docx", ".txt"]
    filename = file.filename or "document"
    ext = os.path.splitext(filename)[1].lower()
    
    if ext not in allowed_types:
        raise HTTPException(
            status_code=400,
            detail=f"Unsupported file type: {ext}. Allowed: {allowed_types}"
        )
    
    # Save to temp file
    with tempfile.NamedTemporaryFile(delete=False, suffix=ext) as tmp:
        content = await file.read()
        tmp.write(content)
        tmp_path = tmp.name
    
    try:
        logger.info(f"Parsing document: {filename}")
        
        # Convert document
        converter = get_converter()
        result = converter.convert(tmp_path)
        
        # Extract document
        doc = result.document
        
        # Get sections
        sections = []
        for section in doc.sections or []:
            sections.append({
                "title": section.title or "",
                "level": getattr(section, 'level', 1),
                "text": section.text or ""
            })
        
        # Get chunks with hybrid chunker
        chunker = get_chunker()
        chunks = []
        for i, chunk in enumerate(chunker.chunk(doc)):
            chunks.append({
                "chunk_id": i,
                "text": chunk.text,
                "contextualized": chunker.contextualize(chunk),
                "section": getattr(chunk, 'section', None),
                "page": getattr(chunk, 'page', None),
                "token_count": len(chunk.text.split())
            })
        
        # Full text
        full_text = doc.export_to_markdown()
        
        # Metadata
        metadata = {
            "page_count": getattr(doc, 'page_count', None),
            "has_tables": len(doc.tables) > 0 if hasattr(doc, 'tables') else False,
            "has_figures": len(doc.figures) > 0 if hasattr(doc, 'figures') else False,
        }
        
        return ParsedDocument(
            filename=filename,
            title=doc.title if hasattr(doc, 'title') else None,
            sections=sections,
            chunks=chunks,
            full_text=full_text,
            metadata=metadata
        )
        
    except Exception as e:
        logger.error(f"Error parsing document: {e}")
        raise HTTPException(status_code=500, detail=str(e))
    finally:
        os.unlink(tmp_path)


@app.post("/chunk", response_model=list[ChunkResult])
async def chunk_text(
    text: str,
    max_tokens: int = 512,
    merge_peers: bool = True
):
    """
    Chunk plain text using hybrid chunking algorithm.
    
    This is useful for chunking already-extracted text.
    """
    try:
        # Create a simple document from text
        from docling.datamodel.document import TextDocument
        
        doc = TextDocument(text=text)
        
        chunker = HybridChunker(
            max_tokens=max_tokens,
            merge_peers=merge_peers
        )
        
        chunks = []
        for i, chunk in enumerate(chunker.chunk(doc)):
            chunks.append(ChunkResult(
                chunk_id=i,
                text=chunk.text,
                contextualized=chunker.contextualize(chunk),
                section=getattr(chunk, 'section', None),
                page=getattr(chunk, 'page', None),
                token_count=len(chunk.text.split())
            ))
        
        return chunks
        
    except Exception as e:
        logger.error(f"Error chunking text: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/parse-url")
async def parse_from_url(url: str):
    """
    Parse a document from URL.
    
    Supports HTTP/HTTPS URLs to PDFs and other documents.
    """
    try:
        logger.info(f"Parsing document from URL: {url}")
        
        converter = get_converter()
        result = converter.convert(url)
        doc = result.document
        
        # Get chunks
        chunker = get_chunker()
        chunks = []
        for i, chunk in enumerate(chunker.chunk(doc)):
            chunks.append({
                "chunk_id": i,
                "text": chunk.text,
                "contextualized": chunker.contextualize(chunk)
            })
        
        return {
            "url": url,
            "title": doc.title if hasattr(doc, 'title') else None,
            "chunks": chunks,
            "full_text": doc.export_to_markdown()
        }
        
    except Exception as e:
        logger.error(f"Error parsing URL: {e}")
        raise HTTPException(status_code=500, detail=str(e))


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8002)
