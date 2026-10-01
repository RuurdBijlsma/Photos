# WEB

* view-option (like gmail), split view: if you single click a photo it opens in a right half of the window pane. Only
  works with enough screen width (desktop).
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
* --- MEDIUM PRIO ---
* improve messaging when you load the website and the server is off
* on login redirect to where you were
* don't allow user to go to /onboarding if onboarding is done already.
* --- HIGH PRIO ---
* door alle requests kijken op verse page load om te zien of ze allemaal relevant zijn (ik zag thunder icon geladen worden op timeline page load)
* in failed list op ingest pagina, download knop voor de file toevoegen zodat je kan inspecteren of ie stuk is
* fix PWA icons (currently has black borders on firefox for some reason)
* [MOBILE REFACTOR]
* activity view is uggo
* fix snackbars
* on mobile notification bar and bottom bar should be transparent if possible.

# SERVER

* email password reset?
* backup (met export jsons / import?)
* import albums from google photos
* duplicate photo remover tool
* better error if exiftool isnt there (worker wont work then)
* apply rotation immidiately when going next/prev to other media item.
* AVI toevoegen aan settings video_extensions
* The original `PXL_20260911_122108760.jpg` puts exiftool in an infinite loop.
  * possible solution -> if hangs (in media analyzer), then run:
    * `exiftool -HDRPlusMakerNote= -overwrite_original <temp_file>`
    * might as well also delete: `MakerNotes`
  * on a temp file to get the exif data
  * On a detected hang it should kill & restart the exiftool daemon

# INFRASTRUCTURE:

* update available checker
* restart server button?