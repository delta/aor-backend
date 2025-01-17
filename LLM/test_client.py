import requests

def test_ask():
    url = "http://localhost:8000/ask"
    payload = {"inputs": {"base_event": "Attacker dropped a bomb."}}
    
    response = requests.post(url, json=payload)
    
    print("Status Code:", response.status_code)
    print("Response JSON:", response.json())

if __name__ == "__main__":
    test_ask()