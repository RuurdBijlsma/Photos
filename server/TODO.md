* ✅ copy setup-related endpoints from old backend
* ✅ set up new api backend.
* ✅ fix shitty refresh token finding
* ✅ fix errors in api, abstraction for it, probably.
* ✅ Api docs swagger
* ✅ in auth/model, split db models and api interfaces
* ✅ users have to be implemented in photos processing at some point (media item must have user id) (user folders)
* ✅ I accidentally made this a new repo, original was photos-processing
* ✅ use db config when setting up db. (pool size etc.)
* ✅ als een crate de settings retrieved voordat dotenv geladen is gaat het stuk.
* ✅ look at rust config package
* ✅ avif not supported by visual analyzer
* ✅ Add some kind of cli flag to specify that a worker can't work on ML type of job
* ✅ BUG als een worker dood gaat terwijl een job aan het running is dan blijft ie running en pakt niemand m meer op.
* ✅ add time_utc to media_item table
* ✅ rename taken_at_local to taken_at_local
* ✅ camelCase elke interfaces.rs struct
* ✅ protobuf for more endpoints?
* ✅ i made the photos handler/service code garbage. clean up pls.
* ✅ Dont use single character field names now that we use protobuf for big requests
* ✅ 👎 look into not using generated code, just add the prost annotations on the real structs
* ✅ response size of by-month.pb is about 51 kb, so why is the request so slow? request on rust end is around 25-30 ms,
  but on frontend end is 100-125 ms.
* ✅ de frontend blijft maar in een loop requests maken als de backend errort (/admin/folders/?folder= ten minste)
* ✅ make ratios endpoint more of a timeline endpoint, with count per month.
* ✅ thumbnails zijn gedraait (orientation tag exif)
* ✅ by-month and timeline dont return in sync media items. timeline ratios is wrong, it's not in order of
  taken_at_local.
* ✅ use time_utc for sorting with COALESCE (don't use it for binning into months and such, and don't return the utc time
  to user)
* ✅ Fix failed analysis jobs
* ✅ Refresh auth wordt niet goed gedaan in frontend.
* ✅ !BUG user_id from relative path is broken
* ✅ heb ik met de nieuwe fallback timezone 0 null's in taken at utc? ja maar dat is een leugen dus ik haal t weg
* ✅ refresh token gives 415 for some reason.
* ✅ add llm to py interop
* ✅ Improve last_error field in jobs, just put entire report in there?
* ✅ now that i have sort_timezone in the db, should i still use fallback timezone to calculate time_utc?
* ✅ visual analysis should have frame percentage or something as a column.
* ✅ ML Analysis:
    * ✅ Make ML jobtype, give priority below videos (30?) so they are done last
    * ✅ color data from python, make in rust
    * ✅ captioner logic in rust (all the questions like is_animal)
    * ✅ quality measure from python, make in rust
    * ✅ make required sql migration tables for ML analysis
    * ✅ handle machine learning analysis job, put in db
* ✅ schedule runner -> might have to use ofelia or kubernetes+helm to get clean cronjobs.
    * ✅ indexing
    * ✅ clean refresh token table on schedule
    * ✅ clustering on schedule
* ✅ Show photos in ui:
    * ✅ make endpoint: get photos by month, ui handles which month to fetch
    * ✅ make endpoint: get timeline summary -> get list of every month with amount of photos for that month. (per user)
    * ✅ moet nog een photo density endpoint hebben om de scrollbar density te laten zien.
    * ✅ nieuwe dag is niet altijd newline in de photos grid, misschien toch weer over gaan naar maanden requesten.
    * ✅ data_url veld in db is useless denk ik (ook in alle analyzers)
    * ✅ virtual scroll waar elke maand 1 virtual scroll item is? of elke row is 1 virtual item??
* ✅ pending_album_media_items isnt getting used
* ✅ Change album id from uuid to niceid (no longer univerally unique requirement)
* ✅ [BUG] pending media items seems to be not used again
* ✅ worker does not output logs to stdout anymore.
* ✅ store_media en store_visual_analysis (met de macros) moet in common_services/database
* ✅ make invite check work with "localhost:9475" instead of "http://localhost:9475" and make it work with https. (it
  currently assumes http).
* ✅ improve OCR
* ✅ [BUG] scan enqueues duplicate jobs if the photo isn't processed yet.
* ✅ [BUG] if album name for /albums/invite/accept is already a folder in media_dir/user_folder, then it doesn't work
  properly.
* ✅ rename details to media_details
* ✅ rename setup to onboarding
* ✅ don't allow start onboarding endpoint if onboarding is already done.
* ✅ Tests:
    * ✅ auth
    * ✅ onboarding
    * ✅ ingest
    * ✅ retrieve
    * ✅ album
    * ✅ cross server album
* ✅ Create integration-tests crate:
    * ✅ runs all binary crates in 1 binary, so tests can be run properly.
    * ✅ have test specific database, that's fresh at start of test.
    * ✅ have test folder for media items, make fresh before each test (tests/original_test_images copied to
      tests/tmp_folder/media_dir before integration tests are run) The tmp folder can be deleted after tests.
    * ✅ Thumbnails dir also for test in tmp folder.
    * ✅ simulate user interactions by calling api with reqwest.
    * ✅ check state after each interaction or after important interactions
* ✅ remove unused crates
* ✅ If enqueueing ingest/analyze, then remove 'remove' jobs for same relative path? Idk maybe?
* 👎 make worker crate stop on ctrl c
* 👎 [moet snel voor search embedding] machine learning stuff in aparte app/container doen? en dan met gRPC/protobuf
  communiceren met api en worker zodat de
* ✅ fix docker image not finding py_analyze (because it looks in crates/...)
* ✅ fix test tracing subscriber
* ✅ copy pics to temp folder on test start
* ✅ fix test py_analyze
* ✅ split routes/photos into timeline related and media item related
  container size van deze 2 niet zo huge worden. Tonic is rust grpc crate.
* ✅ add remote_user_id as collaborator to album.
* ✅ rename types with similar names to db tables, so ColorData from ml_analysis becomes PyColorData or something (look
  at how ml analysis ColorData is actually used)
* ✅ [BUG] accept invite is broken.
* ✅ repeated code in import album en import album item worker job, repeated code is in api/s2s en api/albums
    * ✅ parse url stuff
    * ✅ parse token maybe?
    * ✅ share reqwest client via application state and worker context so it's not made every time.
    * ✅ Improve structure of common structs in common photos. (job_payloads.rs ofzo erbij?)
    * ✅ get s2s invite summary
    * ✅ make s2s client in common code somewhere, to call s2s endpoints.
* ✅ pretty sure the watcher doesn't do anything if a folder is deleted.
* ✅ make UserStore::(find user by mail/id) (get user role) (set user media folder)
* ✅ timeline performance
    * ✅ use proper index on get-month endpoint, if not already at max perf level.
    * ✅ timeline_summary.sql en ratios_summary.sql migrations deleten, en weer maken met goeie nieuwe columns (maybe its
      already pretty good).
    * 👎 Summary table voor ratios
    * ✅ performance check voor beide /timeline endpoints met 100k photos erin (explain analyze, check of frontend js
      veel
      delay toevoegt)
* ✅ websocket om nieuwe foto events te sturen
* ✅ clean up error and warn and info tracing logs
    * ✅ error for fatal boys
    * ✅ warn for user might be impacted
    * ✅ info for info
* ✅ clean up websocket code
* ✅ add cache for processing
    * ✅ cache based on file hash
    * ✅ setting for enabling cache
    * ✅ thumbnails
    * ✅ processed_info
    * ✅ analysis_info
* ✅ Clean up timeline/service.rs duplicated code
* ✅ BIG CHANGE 2
    * ✅ MISSCHIEN KAN JE VOOR ALBUMS WEL GEWOON ALLES REQUESTEN
    * ✅ hele timeline (ratios+item jsons (zonder timestamp)) = 117ms / 185kb voor 10k items
    * ✅ frontend erop aanpassen, geowon nieuwe timeline fresh maken (virtual scroll met grid row erin, nieuwe make grid
      functie maken)
* ✅ non-analysis-worker spawns embedder
* ✅ i think ocr_text should have higher prio
* ✅ ocr_languages in settings doet niks meer
* ✅ er is iets mis met portret videos (ze krijgen een 16:9 ratio), zal iets met orientation zijn ofzo
* ✅ play with weights for full text search
* ✅ vector search lijkt wel wat beter dan fts, test met meer fotos ingested. Lijkt nu wel redelijk afgesteld. Vector
* ✅ probeer reciprocal rank fusion ofzo
* ✅ on demand video thumbnails
* ✅ on demand videos?
* ✅ Voeg toe aan album tabel: earliest_media_item_timestamp -> zodat ik 2019-2020 kan laten zien in UI.
* ✅ make get_representative_thumbnail or something, that returns the image that has an embedding closest to the centroid
  of the list sent to the function. if no embeddings are available yet, return the middle item in the list
  chronologically. If partial embeddings are available, use centroid logic for >50% available embeddings, otherwise
  middle item chronologically. Use get_representative_thumbnail when creating an album, to set as the album thumbnail.
* ✅ Fix performance of get_representative_thumbnail
* ✅ videos hebben te hoge prio in the simple timeline
* ontwikkel snelle object detection oid zodat search suggestions kan zonder llm
* ✅ negative query in search does not work
* ✅ sort by date in search is beetje dom
* ✅ basic search is langzamer nu dan eerst
* ✅ todo: if negative query exists, use embed_texts to batch embed 2 texts
* ✅ api:
    * ✅ add random image + theme endpoint
    * ✅ cors met tower-http::cors
    * ✅ change the json output of vec<photo> to have small field names (is like 50% smaller)
    * ✅ Show photos in ui
    * ✅ only allow register if no user exists
    * ✅ frontend tip: maybe put each row in a lazyload? or skeleton loader, or stop loading='lazy' op img tags
    * ✅ add expiry time to auth responses (zit er al in via jwt, moet dat nog? ik denk t wel)
    * 👎 axum-gate? crate voor axum auth
    * ✅ rate limit met tower-http::limit voor /login en /auth/refresh en password reset endpoint als ik die krijg
* ✅ kan camelcase op de proto generated structs?
* ✅ make search result item protobuf
* ✅ benchmark albums endpoints
* ✅ cache embeddings for search? could be big speedup
* ✅ search filter params moet person thumbnails geven
* ✅ in de person face clustering task, zorg dat die ook face thumbnails genereert, en die op te halen zijn via de person
  table
* ✅ sommige jobs moeten altijd runnen nadat ingest klaar is, bijvoorbeeld:
    * ✅ cluster faces
    * ✅ UpdateGlobalCentroid
    * ✅ ClusterPhotos
* ✅ gebruik get_representative-thumbnail voor face thumb selection
* ✅ current albums pb interface misses collaborators
* ✅ clean thumbnails folder task in task runner
* ✅ Fix search zo dat je alle resultaten boven een bepaalde relevancy vind
    * ✅ Als ik zoek "food" moet ik iets van duizend plaatjes krijgen
* ✅ [SPECIAL CASE] WHEN SEARCH TERM = "", then return all photos that match the filters
* ✅ search suggestions moet person names geven (moet ook een person face thumbnail bij in de response)
    * ✅ hiervoor is een face page nodig denk ik, waar je alle fotos met een person kan zien. Niet search.
* ✅ thumbnail hosting is niet veilig
* ✅ retrieve person face thumb not safe (something like /thumbnails/people/1.webp)
* ✅ also fotos exact zelfde sort datetime hebben, gaat de timeline UI mis, want de sorts zijn dan inconsistent voor deze
  items (2e sort toevoegen? idk)
* ✅ Make invite token functionality for registering new user. (Admin sets the folder, linked to the invite token in
  db, when invite token is used and user is created, delete invite token row and put media folder linked to the new
  user account)
* ✅ make sure cache control on thumbnails are immutable/max age.
* ✅ automatic onboarding
* ✅ review albums/handlers albums/service voor nieuwe ids/by-month/ratios endpoints
    * ✅ is auth wel goed implemented? met is_public enzo
    * ✅ minder repeated code maken voor de auth check daar
* ✅ [BUG] Als je met filters naar persons zoek, zoekt ie op basis van FaceName om een of andere reden. Als je 2 personen
  hebt met dezelfde naam, gaat dit fout.
* ✅ REFACTOR TIME. Theme hoeft niet in DB. het zet de variant te vast en het kan meer dan snel genoeg zonder db access
    * ✅ bij ingest, alleen de extracted kleuren van de item in db zetten
    * ✅ bij get random theme endpoint, voeg variant parameter toe (wordt in settings gezet)
    * ✅ haal theme json uit db
    * ✅ in frontend, laat user variant kiezen voor random bg image
* ✅ default sort for albums should be oldest first (currently newest first - WRONG)
* ✅ mayhaps kan de theme uit de full album item response, wordt niet meer gebruikt
* 👎 ingest queue is irritant want als metadata faalt dan zitten alle anderen nog in de queue ofzo (thumbs, analysis, llm)
* ✅ when a second user registers, make sure to do a scan / sync
* ✅ [BUG] when a second user registers with a subfolder of the first user as media folder, and there's already media in
  there that's ingested by the first user, then the photos dont count for the second user. This is weird behaviour. not
  sure how to handle this case.
* ✅ [check] als ik iets soft-delete, make sure dat visual_analysis.deleted ook op false gaat
* ✅ only re-run photo/face cluster if photos have changed since last clustering
* ✅ storage overview in bottom left van navbar.
  * ✅ als thumb storage zelfde drive is als main storage, show 1 balk
  * ✅ anders 2 balkjes, een voor thumb storage, een voor main storage
* ✅ doe quality stats weer in fast_analysis
* ✅ user level job management
  * ✅ ingest status
    * ✅ user level global progress bar (3 progress bars?)
    * ✅ per item list? net als immich
    * ✅ failed ingest overview
  * ✅ start Scan job for user
* ✅ admin page
  * ✅ jobs overview, inclusief manual job run
  * ✅ overview of users: remove user, storage used per user
* ✅ trash can (deleted=true) (in trash UI kan je echte delete doen)
* ✅ settings page
  * ✅ contrast setting in theme
* ✅ remove exif from full photo response
* ✅ write thumbnails to cache folder and symlink to thumbnails folder?
* ✅ add photo search type: Panorama (use_panorama_viewer flag). Currently type=photo|video, add panorama
* password reset flow (email) (make mail optional)
* nginx thumbnail hosting (optional maak setting voor Rust thumb hosting).
* check of readme uitleg klopt met verse windows installatie & linux
* better error if exiftool isnt there (worker wont work then)
* improve speed of album/{id} endpoint
* rotate image by changing thumbnail orientation?

# High level TODO:

* admin page
  * restart server? [for later]
  * backup (met export jsons / import?)
* import albums from google photos
* explore page
  * stats over je fotos?
  * most visited places
  * sort by all kinds of fields (temp, altitude, lat/lon extremes, wind, shutter speed)
  * photo distribution/histogram by day of  year, grouped by month/week?
  * zelfde voor time of day?
* ✅ photo viewer page
  * ✅ implement different viewers
  * find similar fotos view, misschien  een expandable ding in de info panel?
* photo upload feature
* email password reset?