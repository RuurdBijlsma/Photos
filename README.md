# Ruurd Photos

* [Screenshots](https://github.com/RuurdBijlsma/Photos#homepage)
* [Setup](https://github.com/RuurdBijlsma/Photos#prerequisites)
    * [Prerequisites](https://github.com/RuurdBijlsma/Photos#prerequisites)
    * [Guide](https://github.com/RuurdBijlsma/Photos#setup-server)
    * [Automatic upload from Android](https://github.com/RuurdBijlsma/Photos#automatic-upload-from-android)
    * [Setting up HTTPS](https://github.com/RuurdBijlsma/Photos#setting-up-https)

### Homepage

Homepage showing all photos in a grid. Aspect ratio of photos is maintained and scroll is infinite. Scrollbar on the
right shows years, on hover months are shown.

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/home-light.png?raw=true)

### Explore

View things and places.

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/explore.png?raw=true)

### View photo or video

View the photo or video alongside extra info in the pane on the right. The info pane can be hidden, images allow for
zooming. Swipe or use the arrow buttons to go to the next photo.

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/image-view.png?raw=true)

### Photos map

View map of all your photos and videos. Photos are blue circles, videos are orange. Click a circle to view it.

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/map.png?raw=true)

### Search for things

Possible search features

* Item in photo (using machine learning image classification)
* Place taken (country, province, municipality, city)
* Date taken
* Filename
* Filetype

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/thing-search.png?raw=true)

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/place-search.png?raw=true)

### Import albums from Google Photos

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/import.png?raw=true)

### Dark theme

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/home-dark.png?raw=true)

### Responsive layout and touch support

![f](https://github.com/ruurdbijlsma/Photos/blob/master/.gh/mobile.png?raw=true)

## Prerequisites

* Git (https://git-scm.com/)
* Docker + docker-compose (https://www.docker.com/products/docker-desktop)

## Setup server

1. Clone this repo `git clone --recurse-submodules git@github.com:RuurdBijlsma/Photos.git`
2. `cd Photos`
3. Copy `example.env` to `.env`: `cp example.env .env` and edit `.env`
    * Change `PHOTOS_DIR` to where your source photos are.
    * Change `EMAIL` and `PASSWORD` to what you want to use to log in.
    * (Optional) change `THUMBNAILS_DIR` and `BACKUPS_DIR`
4. Run `docker-compose up --build` and wait until your photos are processed (takes a WHILE)
5. Visit http://localhost, enter `http://localhost:3333` as the API endpoint and log in with your credentials from step
   3
6. (optional) Set a Mapbox api key on the settings page to make the maps work

### Automatic upload from Android

1. Download FolderSync (https://play.google.com/store/apps/details?id=dk.tacit.android.foldersync.lite&hl=nl&gl=US)
2. Set up SFTP sync account to your server
3. Set up sync to remote folderpair to automatically upload files from /DCIM/Camera (on phone) to ./media/photos (on
   server)

### Setting up HTTPS

Use nginx reverse-proxy for this, the server runs at localhost:3333, the frontend runs at localhost:80.

Example nginx conf:

```
server {
        listen 80;
        server_name mysite.com;
        return 307 https://$server_name$request_uri;
}
server {
    listen              443 ssl;
    server_name         mysite.com;

    location / {
        proxy_pass http://localhost:80;
        proxy_redirect http://localhost:80/ /;
        proxy_read_timeout 60s;

        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
    }

    ssl_certificate /etc/letsencrypt/live/mysite.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/mysite.com/privkey.pem;
}
server {
        listen 80;
        server_name api.mysite.com;
        return 307 https://$server_name$request_uri;
}
server {
    listen              443 ssl;
    server_name         api.mysite.com;

    location / {
        proxy_pass http://localhost:3333;
        proxy_redirect http://localhost:3333/ /;
        proxy_read_timeout 60s;

        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
    }

    ssl_certificate /etc/letsencrypt/live/mysite.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/mysite.com/privkey.pem;
}
```
