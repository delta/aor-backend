import torch
from transformers import pipeline
from prompt import base_prompt

model_id = "meta-llama/Llama-3.2-3B"

pipe = pipeline(
    "text-generation", 
    model=model_id, 
    torch_dtype=torch.bfloat16, 
    device_map="auto"
)

print(pipe(base_prompt + " Game event: Attacker placed mine on base. Base damage - 15%. Attacker has collected 100 / 1200 artifacts. \n", max_new_tokens=50, num_return_sequences=1))