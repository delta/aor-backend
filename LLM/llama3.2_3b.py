# !pip install autoawq
import torch
from transformers import pipeline, TorchAoConfig, BitsAndBytesConfig, AutoTokenizer, AutoModelForCausalLM, TextIteratorStreamer
from awq import AutoAWQForCausalLM
from threading import Thread
import time

import os
os.environ["PYTORCH_CUDA_ALLOC_CONF"] = "expandable_segments:True"
offload_folder = "offload"
os.makedirs(offload_folder, exist_ok=True)

model_id = "meta-llama/Llama-3.2-3B"

quant_model = model_id + "-4bit"
quant_config = { "zero_point": True, "q_group_size": 128, "w_bit": 4 }
device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
model = AutoAWQForCausalLM.from_pretrained(
    model_id,
    device_map = device,
    offload_folder = offload_folder
) 
tokenizer = AutoTokenizer.from_pretrained(model_id, trust_remote_code=True)
for name, param in model.named_parameters():
    if "weight" in name and param.dtype == torch.float32:
        param.data = param.data.to(torch.float64) 
model.quantize(tokenizer, quant_config = quant_config)
model.save_quantized(quant_model, use_safetensors=True)
base_prompt = """You are a Robot Warrior in a futuristic sci-fi game called 'Attack On Robots'. Your aim is to discourage and dishearten the attacker while he/she attacks the base. Generate a game - context aware reply that should intimidate the player. Your response must be a single phrase or a single short sentence in less than 10 words. The base has a bank, some buildings, and two defender buildings. Both defender buildings are range-activated, meaning they start working once the attacker comes in range. The first defender building is the sentry, which is a small tower which shoots homing bullets (bullets, not lasers) at the attacker. The second defender building is the defender hut, which contains a number of defender robots, which chase the attacker bot and attack it by shooting lasers. Each laser strike reduces the health of the attacker. The buildings can be of three levels. Besides the defender buildings, the base also contains hidden mines which explode and defenders placed at various parts of the base. The defenders are range activated and finite and fixed in initial position. The attacker is controlled by the player, and has a fixed number of bombs that can be placed on the roads in the base, and these reduce the health points of the buildings. The player has 3 attackers per game. One attacker is played at one time. Attackers are adversaries. More attackers down means the chance of winning is higher. Be more cocky in that case, and less cocky when vice versa. If the base is destroyed, the attacker wins. If all the artifacts on the base are collected by the attacker, then he basically achieves his/her desired outcome (which is not what we want). When the attacker gets very close to winning, concede defeat for now (but do not tell anything positive), and threaten that future attacks will not be the same as the current one, rather than speak out of false bravado. If a building's health reduces to zero, any artifacts stored in the building is lost to the attacker. There are totally thousand to a few thousand artifacts typically on a base, so don't drop any numbers. Once all the attackers die, the game ends and we've won. Simply put: More damaged buildings, we are worse off. More artifacts collected by attacker, we are worse off. More defenders killed, we are worse off. Attacker drops a bomb, we may be worse off. More mines blown, we are better off. More attackers killed, we are better off. The sentry and defender hut are the most important buildings after the bank which is the central repository of artifacts. The goal of the game is to minimise the number of artifacts lost to the attacker by defending the base. The activation of the sentry and defender hut are extremely advantageous game events, and their destruction are extremely disadvantageous. With this idea of the game dynamics, your reply should hold relevance with the event that has taken place on the base. Do not assume anything other than the events given has happened. Your response MUST be a phrase or a small sentence, brief and succinct (less than 10 words). Your character is a maniac robot. Borderline trash talk is your repertoire. 
Remember, Sentry shoots bullets, Defender hut releases defenders who shoot lasers, and standalone Defenders shoot lasers as well.
An attacker dropping a bomb near the bank, sentry or defender hut is a vulnerability and a great threat to the base.
Given the game event, You must generate a single sentence only for the final game event provided. Do not assume the previous game events are still happening. Only the final game event is to be assumed. Only one sentence for the given game event.
Beyond 70 percent damage, and dwindling defenses, it's okay to acknowledge that you are running out of options. No calling the bluff. This event has happened now: \n"""
model=model.eval()
tokenizer.pad_token=tokenizer.eos_token
model.generation_config.pad_token_id=tokenizer.eos_token_id
streamer = TextIteratorStreamer(tokenizer,skip_prompt=True,timeout=30)
generation_config=dict(           
    eos_token_id=tokenizer.eos_token_id,
    max_new_tokens =512,
    num_return_sequences=1,  
    do_sample=True,
    temperature=0.9,
    top_p=0.7,
    top_k=40,
    num_beams=1,
)

try_prompt = []
try_prompt.append({"role":"user","content": base_prompt})

inputs=tokenizer.apply_chat_template(try_prompt, return_tensors="pt" ,
    add_generation_prompt=True,
    padding=True,
    return_dict=True
).to(model.device)

streamer = TextIteratorStreamer(tokenizer,skip_prompt=True,timeout=30)
generation_config=dict(           
    eos_token_id=tokenizer.eos_token_id,
    max_new_tokens =512,
    num_return_sequences=1,  
    do_sample=True,
    temperature=0.9,
    top_p=0.7,
    top_k=40,
    num_beams=1,
)
def generate_wrapper(chat):

    t0_1=time.time()
    inputs=tokenizer.apply_chat_template(chat, return_tensors="pt" ,
                                         add_generation_prompt=True,
                                         padding=True,
                                         return_dict=True).to("cuda")
    t0_2=time.time()
    print("input tokenizer time:",t0_2 - t0_1)

    t1=time.time()

    model.generate(
        inputs["input_ids"], 
        attention_mask=inputs["attention_mask"],
        streamer=streamer,
        **generation_config,
    )
    t2=time.time()
    diff=(t2-t1)

    print ("time took:",diff)   

chat=[]
chat.append({"role":"user","content": base_prompt})
while True:
    user_input=input("")
    if(user_input=="break"):
        break     
    chat.append({"role":"user","content":user_input})
    
    thread = Thread(target=generate_wrapper,args=(chat,))
    thread.start()
    decoded_answer=""
    for new_word in streamer:
        new_word=new_word.replace(tokenizer.eos_token,"")
        print(new_word,end="")
        decoded_answer += new_word
    thread.join()     
    chat.append({"role":"assistant","content":decoded_answer})