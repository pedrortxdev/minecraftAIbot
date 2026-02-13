#!/bin/bash
# Wrapper to start the bot

cd "$(dirname "$0")"

# Configurações de Conexão
export SERVER_ADDR="minesraiz.aternos.me"
export SERVER_PORT="35809"

# Autenticação (Vazio = Offline/Pirata)
export BOT_NAME="PedroRTX"
export BOT_EMAIL=""

# O Cérebro (Gemini API)
export GEMINI_API_KEY="AIzaSyAQsaKY12g9teuuWgsNBVt-wxSWyrIZnWY"
export MODEL_FLASH="gemini-2.0-flash"
export MODEL_PRO="gemini-2.5-pro"

# Run
./target/release/frankfurt_sentinel

