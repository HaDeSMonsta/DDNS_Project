This image should run on the Server with the dynamic IP, it will call the Server every n minutes.
See this example docker-compose.yaml
```yaml
version: "3.8"
services:
  app:
    container_name: ddns_client
    image: hadesmonsta/ddns_client
    restart: unless-stopped
    env_file:
      - .env
```

You will also need a .env file with

```env
AUTH=CHANGEME
SERVER_ADDRESS=<IP>:<PORT>
SLEEP_MINS=n
```

Please replace the values with your actual values
