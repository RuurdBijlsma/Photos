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
* preload 1440p thumbnail on grid item hover
* When no user exists and user visits login page -> redirect to register?admin=true and show messaging to create admin
  account
* door alle requests kijken op verse page load om te zien of ze allemaal relevant zijn (ik zag thunder icon geladen worden op timeline page load)
# HIGH PRIO
* rotate 180 deg is broken
* test of nieuwe icons perf impact hebben
  * load time (perf tab)
  * network load bytes & time
  * bundle size

# SERVER

* email password reset?
* backup (met export jsons / import?)
* import albums from google photos
* duplicate photo remover tool
* better error if exiftool isnt there (worker wont work then)
# HIGH PRIO
* 50 megapixels plaatjes willen niet rotaten om een of andere reden (die van tiramisu)

# INFRASTRUCTURE:

* update available checker
* restart server button?