This binary will listen for calls coming from DDNS_Client, check if the password matches
and then if the IP changed, if yes, it will change the saved one and (if set) call another
binary.

You will need a .env file in the same directory

```env
AUTH=CHANGEME
PORT=8080
IP_CONFIG_PATH=/foo/bar/ip.conf
POST_IP_PATH=/path/to/binary
```
