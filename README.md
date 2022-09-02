## Potato Auth
A very simple *auth request* server for nginx.

`PotatoAuth` is in beta and should be used with caution with sensitive data.

## Installation
### Windows
* Download from the *releases* page or build from source.

* Copy `potato_auth.exe` to a convenient location.

* Copy `static/` and its contents to the same directory as `potato_auth.exe`

* Start `potato_auth` and navigate to `localhost:8675/setup` to begin.

A default `potato_auth.config` and `potato_auth.userdb` will be created in `~/PotatoAuth` unless another config location is specified. Logs will be written to `~/PotatoAuth/logs/` unless another log directory is specified.

### Linux
* Download from the *releases* page or build from source.

* Copy `potato_auth` to a convenient location such as `/opt/PotatoAuth`

* Copy `static/` and its contents to the same directory as `potato_auth`

* Start `potato_auth` and navigate to `localhost:8675/setup` to begin.

`PotatoAuth` does not require sudo permissions to operate, but does require write permissions to several directories that may need to be created and permissions set prior to starting `potato_auth`. The default locations are listed below but can be set to any location in `PotatoAuth.conf`

* `/var/log/PotatoAuth`
* `/etc/PotatoAuth`

## NGINX Config
Nginx configurations may vary significantly, so adjust this as neccesary.

* Copy `potato_auth.nginx.conf` to `/etc/nginx/` and rename to `potato_auth.conf`

* Add the following lines to any server block that you want `PotatoAuth` to protect:

```
include /etc/nxing/potato_auth.conf;
auth_request /authrequest;
```

An example of a basic server would be the following:
```
server {
        include /etc/nginx/potato_auth.conf;
        auth_request /authrequest;

        listen 80 default_server;
        listen [::]:80 default_server;

        server_name _;

        location /foo{
            proxy_pass http://localhost:8000/;
        }
}
```