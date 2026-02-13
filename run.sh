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

# 2. Verifica se o binário existe
BINARY="./target/release/frankfurt_sentinel"
if [ ! -f "$BINARY" ]; then
    echo "[ERROR] ❌ Binário não encontrado em $BINARY"
    echo "Dica: Execute 'cargo build --release' primeiro."
    exit 1
fi

# 3. Executa o PedroRTX
echo "[SYSTEM] 🚀 Iniciando Frankfurt Sentinel..."
$BINARY

