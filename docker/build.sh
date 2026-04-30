#!/bin/bash

# Build Docker images for Ferrumyx bioinformatics tools

set -e

echo "Building Ferrumyx bioinformatics container images..."

# Build BLAST image
echo "Building BLAST image..."
docker build -f docker/Dockerfile.blast -t ferrumyx/blast:latest .

# Build PyMOL image
echo "Building PyMOL image..."
docker build -f docker/Dockerfile.pymol -t ferrumyx/pymol:latest .

# Build FastQC image
echo "Building FastQC image..."
docker build -f docker/Dockerfile.fastqc -t ferrumyx/fastqc:latest .

# Build fpocket image
echo "Building fpocket image..."
docker build -f docker/Dockerfile.fpocket -t ferrumyx/fpocket:latest .

# Build AutoDock Vina image
echo "Building Vina image..."
docker build -f docker/Dockerfile.vina -t ferrumyx/vina:latest .

# Build RDKit image
echo "Building RDKit image..."
docker build -f docker/Dockerfile.rdkit -t ferrumyx/rdkit:latest .

# Build ADMET image
echo "Building ADMET image..."
docker build -f docker/Dockerfile.admet -t ferrumyx/admet:latest .

echo "All images built successfully!"
echo ""
echo "To start the services:"
echo "docker-compose -f docker/docker-compose.yml up -d"
echo ""
echo "To check running containers:"
echo "docker ps"