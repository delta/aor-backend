import google.generativeai as genai
from prompt import base_prompt
import os
import time

gemini_api_key = os.getenv("GEMINI_API_KEY")

genai.configure(api_key=gemini_api_key)

event = "Game event: Attacker placed mine on base. Base damage - 15%. Attacker has collected 100 / 1200 artifacts. \n"
model = genai.GenerativeModel("gemini-2.0-flash-exp")
chat = model.start_chat(
    history=[
        {"role": "user", "parts": base_prompt},
        {"role": "model", "parts": "Sure thing. I'll assume this role right away and start the game now."},
    ]
)
event = "Game starts"
while event != "break":
    event = input("")
    start_time = time.time()
    response = chat.send_message(event)
    print(response.text, "\n")
    end_time = time.time()
    print(end_time - start_time)

