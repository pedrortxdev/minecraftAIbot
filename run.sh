#!/bin/bash

# Move para o diretório do script (importante para achar a .env)
cd "$(dirname "$0")"

# 1. Carrega as variáveis da .env ignorando comentários e linhas vazias
if [ -f .env ]; then
    echo "[SYSTEM] 📥 Carregando configurações da .env..."
    export $(grep -v '^#' .env | xargs)
else
    echo "[ERROR] ❌ Arquivo .env não encontrado!"
    exit 1
fi

# 2. Compila o binário (release) — garante que sempre roda o código mais recente
echo "[SYSTEM] 🔧 Compilando (cargo build --release)..."
cargo build --release 2>&1
if [ $? -ne 0 ]; then
    echo "[ERROR] ❌ Falha na compilação!"
    exit 1
fi

BINARY="./target/release/frankfurt_sentinel"

# 3. Executa o PedroRTX
echo "[SYSTEM] 🚀 Iniciando Frankfurt Sentinel..."
$BINARY

