# Ruurd Photos

<p align="center">
  <img src=".github/screenshots/timeline.png" alt="Timeline">
  <br>
  <em>Chronological Timeline</em>
</p>

<p align="center">
  <img src=".github/screenshots/search.png" alt="Search">
  <br>
  <em>Search for anything</em>
</p>

<p align="center">
  <img src=".github/screenshots/explore.png" alt="Explore">
  <br>
  <em>Explore stats & locations</em>
</p>

<p align="center">
  <img src=".github/screenshots/map.png" alt="Map">
  <br>
  <em>Map view</em>
</p>

<p align="center">
  <img src=".github/screenshots/photo-view.png" alt="Photo View">
  <br>
  <em>Photo viewer with info panel</em>
</p>

<p align="center">
  <img src=".github/screenshots/admin.png" alt="Admin">
  <br>
  <em>Admin page for user & background job management</em>
</p>

# Install

1. **Create a directory for the application**

   ```bash
   mkdir photos-app && cd photos-app
   ```

2. **Download the Docker Compose file and example environment file**

   ```bash
   wget https://github.com/ruurdbijlsma/Photos/releases/latest/download/compose.yml
   wget -O .env https://github.com/ruurdbijlsma/Photos/releases/latest/download/example.env
   ```

3. **Edit `.env`**

   Set at least:
    - `MEDIA_LOCATION`
    - `JWT_SECRET`

4. **Start the application**

   ```bash
   docker compose up -d
   ```

5. **Wait for startup**

   The first launch may take a few minutes.

6. **Open the application**

   Visit http://localhost:9475 in your browser.

7. **Create an administrator account**

   Register your first account. It will automatically become the administrator.

8. **Complete the setup wizard**

   Verify that your media drives are detected correctly and choose a user folder.

   > **Note:** The user folder must be a subdirectory of your mounted media folder if you want to support multiple
   users.

9. **Start processing**

   Click "Start Processing". Your photos and videos will begin appearing in the timeline.