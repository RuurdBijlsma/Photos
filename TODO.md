# WEB

* on login redirect to where you were
* improve messaging when you load the website and the server is off
* ✅ make func to refresh frontpage, call it after onboarding is done after 10s, then every 5s;
* view-option (like gmail), split view: if you single click a photo it opens in a right half of the window pane. Only
  works with enough screen width (desktop).
* don't allow user to go to /onboarding if onboarding is done already.
* setting: usebackdropblur doesnt apply everywhere.
* preload 1440p thumbnail on grid item hover
* re-establish ws connection if auth failed and it's refreshed automatically afterwards
* add sort order to timeline controller and remove it as passed down prop, and use it in api requests through that prop
* idea to fix desync timeline bug:
    * bug: timeline ids/ratios/by month might by out of sync because theyre separate requests
    * possible solution: add a param: addedAtCutoff which is set by frontend at the currenttime of the first request.
    * this would prevent new photos being added in between the ratios and byMonth request
    * it doesn't prevent removals messing things up, but removals are done by UI interaction so that's less of a problem
* [BUG] als je /profile window klein maakt kan je niet scrollen naar onder
* [investigate] krijg je ratio/monthItem desync als je de map eerst laad, terwijl de backend foto metadata ingest, en
  dan naar de timeline gaat? Want de map date filter heeft dan al de ratios opgevraagd in de timelineStore.
* settings page heeft geen visible scrollbar
* update <title> on navigation
* i dont see Source quality in podman hosted video
* With 1 drive the nav bar storage overview is kinda ugly
* GH pages release should be triggered by GH release, not push to main
* improve loading indicator for albums & cameras (perhaps also people)
* update readme screenshot to exclude the vue dev tools overlay

# SERVER

* nginx thumbnail hosting (optional maak setting voor Rust thumb hosting).
* check of readme uitleg klopt met verse windows installatie & linux
* better error if exiftool isnt there (worker wont work then)
* improve speed of album/{id} endpoint
* make single executable kind of that runs api, watcher, and worker, but have it sort of auto-scale based off number of
  jobs. So api, watcher, autoscaler, and if theres jobs, then N amount of workers.
* email password reset?
* restart server? [for later]
* backup (met export jsons / import?)
* import albums from google photos
* duplicate photo remover tool
* rotate image by changing thumbnail orientation?
* arm docker build in release process
* watcher errors when there's no users
* make setup to host only backend, frontend could be gh pages or something?
* add TZ env var to example.env and make sure it's used
* uploads dont work on podman
* minder repeated config in compose.yml: https://medium.com/@FKosa/docker-compose-yaml-anchors-aliases-the-dry-principle-for-your-config-files-31c543ef023b