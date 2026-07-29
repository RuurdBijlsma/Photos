# Photos Frontend

The web interface for **Ruurd Photos**, a self-hosted Google Photos alternative. Built with Vue, Vuetify, and
TypeScript.

## Features

* **Timeline View:** All your media in a grid on the front-page, organized by date.
* **Map View:** Geospatial visualization of your media library.
* **Albums:** Organize your media in albums, you can share these albums to anyone.
* **People:** People recognized in your photos, you can label them so they show up in searches, and you can browse all
  photos of this person.
* **Cameras:** Your media organized by which camera captured it.
* **Search:** Smart search through your media, powered by machine learning.
* **Memories:** The front page features cards like "On this day" highlighting photos and videos you took years ago, but
  on the current date.

## Prerequisites

* The frontend requires the server to be running.
* Node
* npm

## Installation

### 1. Clone the repo

```bash
git clone https://github.com/RuurdBijlsma/Photos.git
cd Photos/web
```

### 2. Install dependencies

```bash
npm install
```

### 3. Configure Environment

Copy the example environment file and configure the connection to your backend.

```bash
cp example.env .env
```

Edit `.env` to point to your API URL (usually localhost if running locally):

```properties
VITE_API_BASE_URL=http://localhost:5272
```

---

## Usage

### 1. Run Development Server

To start the local development server with Hot Module Replacement (HMR):

```bash
npm run dev
```

Open your browser and navigate to `http://localhost:5173` (or the port shown in the terminal).

### 2. Build for Production

To type-check and build the application for production deployment:

```bash
npm run build
```

The output will be generated in the `dist/` directory.

### 3. Generate Protocol Buffers

If you modify the `.proto` definitions in `src/proto`, regenerate the TypeScript types:

```bash
npm run proto:gen
```

### 4. Linting & Formatting

* **Lint:** `npm run lint`
* **Format:** `npm run lint:format`
