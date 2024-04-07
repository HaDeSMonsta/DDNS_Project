This is my solution to having one Server with a dynamic
IP-Address and another one with a fixed.

The project contains three modules:
1. Client: A Docker image to which calls the Server every n minutes
2. Server: Waits for calls with the correct password, will update the IP in a config
file if it does not match
3. Post: Will be called by the Server if the IP changed

For specific instructions refer to the README in the subdirectories
