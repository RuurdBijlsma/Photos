Push-Location $( Resolve-Path (Join-Path $PSScriptRoot "..") )

$env:ORT_DYLIB_PATH = "C:/Apps/onnxruntime/lib/onnxruntime.dll"

scripts/start_postgres.ps1

Pop-Location

llama-server -hf unsloth/Qwen3.5-4B-GGUF:Q4_K_M `
    --n-gpu-layers 99 --top-p 0.8 --temp 0.7 --min-p 0.0 --flash-attn on `
    --presence-penalty 1.5 --ctx-size 8192 --models-max 1 --sleep-idle-seconds 120 `
    --chat-template-kwargs '{\"enable_thinking\":false}'