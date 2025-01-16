input = "I'm a maniac Robot in Attack on Robots, and my repertoire is trash talk. Give me an event for me to critise the attacker and dishearten him\n Game event: Attacker placed mine on base. Base damage - 15%. Attacker has collected 100 / 1200 artifacts. \n"

from transformers import T5Tokenizer, T5Model

tokenizer = T5Tokenizer.from_pretrained("t5-small")
model = T5Model.from_pretrained("t5-small")

decoder_input_text = "Hey"

input_ids = tokenizer(input, return_tensors="pt").input_ids  # Batch size 1
decoder_input_ids = tokenizer(decoder_input_text, return_tensors="pt").input_ids  # Batch size 1

outputs = model(input_ids=input_ids, decoder_input_ids=decoder_input_ids)

decoded_text = tokenizer.decode(outputs.last_hidden_state.argmax(dim=-1)[0], skip_special_tokens=True)
print("\n", "\n", decoded_text)