from fastapi import FastAPI
from langserve import add_routes
import uvicorn
from langchain_core.prompts import ChatPromptTemplate
from langchain_ollama import OllamaLLM
from prompt import base_prompt

app = FastAPI(
    title="My AI Model API",
    description="A simple API",
    version="1.0",
)

prompt = ChatPromptTemplate.from_template(base_prompt + " {base_event}.")

llm = OllamaLLM(model = "mistral")

add_routes(
    app, prompt|llm, path="/ask"
)

if __name__ == "__main__":
    uvicorn.run(app, host="localhost", port=8000)