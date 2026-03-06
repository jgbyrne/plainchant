## Simple rough-and-ready client that you can use to interact with the site console
## Anything that can send POST requests and print messages will work, really

import requests
import readline

CONSOLE_URL = "https://{{SITE_DOMAIN_NAME}}/api/console"

# Set this in your config file
# It's the value 'access_key' in the section 'console'
KEY = "{{SITE_ACCESS_KEY}}"

try:
    while True:
        command = input(">>> ")
        headers = {"X-Authorization": "Bearer " + KEY}
        resp = requests.post(CONSOLE_URL, data=command, headers=headers)
        if resp.status_code != 200:
            print(f"[{resp.status_code}] {resp.text}")
        else:
            print(resp.text, end="")

except KeyboardInterrupt:
    pass
