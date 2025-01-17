import google.generativeai as genai
from prompt import base_prompt

genai.configure(api_key="AIzaSyCteSFTfOeL0EZPM-MCLzh0j9nHgHK8Ke0")

event = "Game event: Attacker placed mine on base. Base damage - 15%. Attacker has collected 100 / 1200 artifacts. \n"
model = genai.GenerativeModel("gemini-1.5-flash")
chat = model.start_chat(
    history=[
        {"role": "user", "parts": base_prompt},
        {"role": "model", "parts": "Sure thing. I'll assume this role right away and start the game now."},
    ]
)
event = "Game starts"
while event != "break":
    response = chat.send_message(event)
    print(response.text, "\n")
    event = input("")

