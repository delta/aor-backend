import google.generativeai as genai
from prompt import base_prompt
import os

gemini_api_key = os.getenv("GEMINI_API_KEY")

genai.configure(api_key=gemini_api_key)

model = genai.GenerativeModel("gemini-1.5-flash")

ip = "Game starts"

while ip != "break":
    response = model.generate_content(base_prompt + ip)
    print(response.text, "\n")
    ip = input("")