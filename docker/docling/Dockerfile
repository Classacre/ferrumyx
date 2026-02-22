# Docling Docker Service
# Provides PDF/XML parsing with section-aware chunking

FROM python:3.11-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    poppler-utils \
    tesseract-ocr \
    && rm -rf /var/lib/apt/lists/*

# Install Docling
RUN pip install --no-cache-dir \
    docling[core] \
    fastapi \
    uvicorn \
    python-multipart

# Copy service code
COPY docling_service.py /app/

# Expose port
EXPOSE 8002

# Run service
CMD ["uvicorn", "docling_service:app", "--host", "0.0.0.0", "--port", "8002"]
