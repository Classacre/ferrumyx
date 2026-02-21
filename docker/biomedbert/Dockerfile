# Ferrumyx BiomedBERT Embedding Service
# Provides a FastAPI batch endpoint that embeds text using BiomedBERT-base
# and writes vectors back to PostgreSQL via pgvector.
#
# Model: microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract
# Output dimension: 768

FROM python:3.11-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential libpq-dev \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY main.py .

EXPOSE 8002

ENV MODEL_NAME="microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract"
ENV BATCH_SIZE=32
ENV MAX_SEQ_LEN=512

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8002", "--workers", "1"]
