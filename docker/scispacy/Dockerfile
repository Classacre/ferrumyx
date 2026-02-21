# SciSpacy NER Service
# Provides a FastAPI REST endpoint for biomedical named entity recognition.
# Models: en_core_sci_lg (general biomedical) + en_ner_bc5cdr_md (chemical/disease)

FROM python:3.11-slim

WORKDIR /app

# System deps for spacy / numpy
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Python packages
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Download SciSpacy models
# en_core_sci_lg: general biomedical NER (~800MB)
# en_ner_bc5cdr_md: chemical + disease NER (~200MB)
RUN pip install --no-cache-dir \
    "https://s3-us-west-2.amazonaws.com/ai2-s2-scispacy/releases/v0.5.4/en_core_sci_lg-0.5.4.tar.gz" \
    "https://s3-us-west-2.amazonaws.com/ai2-s2-scispacy/releases/v0.5.4/en_ner_bc5cdr_md-0.5.4.tar.gz"

COPY main.py .

EXPOSE 8001

ENV UVICORN_WORKERS=2
ENV MODEL_CACHE_SIZE=3

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8001", "--workers", "2"]
