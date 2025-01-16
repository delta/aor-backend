from transformers import pipeline, set_seed
generator = pipeline('text-generation', model='gpt2')
set_seed(42)
base_prompt = "I'm a maniac Robot in Attack on Robots, and my repertoire is trash talk. Give me an event for me to critise the attacker and dishearten him\n"
input = "Game event: Attacker placed mine on base. Base damage - 15%. Attacker has collected 100 / 1200 artifacts. \n"
print(generator(base_prompt + input, max_new_tokens=30, num_return_sequences=1))