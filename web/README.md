# Photos Frontend

The web interface for **Ruurd Photos**, a self-hosted Google Photos alternative. Built with Vue 3, Vuetify, and
TypeScript.

## Features

* **Timeline View:** High-performance virtual scrolling for photo grids organized by date.
* **Map View:** Geospatial visualization of your media library.
* **Onboarding Wizard:** UI for setting up the server, picking storage folders, and scanning drives.
* **Media Viewer:** Full-screen photo and video playback.
* **Responsive Design:** Optimized for desktop and mobile web via Vuetify.

## Prerequisites

* **[Ruurd Photos Backend](https://github.com/RuurdBijlsma/photos-backend)**: The frontend requires the API to be
  running.
* Node
* npm

## Installation

### 1. Clone the repo

```bash
git clone https://github.com/RuurdBijlsma/photos-frontend.git
cd photos-frontend
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
VITE_API_BASE_URL=http://localhost:3000
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

---

## Tech Stack

* **Framework:** Vue 3 (Composition API)
* **Build Tool:** Vite
* **UI Library:** Vuetify
* **State Management:** Pinia
* **Routing:** Vue Router
* **Language:** TypeScript
* **Data Serialization:** Protocol Buffers

# Demo dataset

https://unsplash.com/data/lite/latest