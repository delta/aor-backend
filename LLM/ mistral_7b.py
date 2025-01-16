from huggingface_hub import snapshot_download
from transformers import AutoTokenizer, AutoModelForCausalLM
from pathlib import Path

mistral_models_path = Path.home().joinpath('mistral_models', '7B-Instruct-v0.3')
mistral_models_path.mkdir(parents=True, exist_ok=True)

snapshot_download(repo_id="mistralai/Mistral-7B-Instruct-v0.3", allow_patterns=["params.json", "consolidated.safetensors", "tokenizer.model.v3"], local_dir=mistral_models_path)

model_path = Path.home().joinpath('mistral_models', '7B-Instruct-v0.3')

# Load tokenizer
tokenizer = AutoTokenizer.from_pretrained(model_path, use_fast=False) 

# Load model
model = AutoModelForCausalLM.from_pretrained(
    model_path,
    trust_remote_code=True 
)

# Example: Text generation
input_text = "What is the meaning of life?"
inputs = tokenizer(input_text, return_tensors="pt")
outputs = model.generate(**inputs, max_length=50, num_return_sequences=1)

# Decode and print the output
output_text = tokenizer.decode(outputs[0], skip_special_tokens=True)
print(output_text)