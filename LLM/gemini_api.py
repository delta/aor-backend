import google.generativeai as genai

genai.configure(api_key="AIzaSyCteSFTfOeL0EZPM-MCLzh0j9nHgHK8Ke0")

model = genai.GenerativeModel("gemini-1.5-flash")
response = model.generate_content("How does AI work?")
print(response.text)