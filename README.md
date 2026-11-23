# Personal Dashboard

A personal dashboard built with Rust (Axum) and React that displays tasks from Outline in a Kanban board.

## Features

- Kanban board with Pending and Completed columns
- Tasks fetched from Outline API and grouped by document/header
- Google OAuth authentication
- Docker deployment ready

## Architecture

- **Backend** (`backend/`): Axum (Rust) — Outline API integration, markdown task parser, Google OAuth. Container listens on port 3000, mapped to host 3001.
- **Frontend** (`frontend/`): React + Vite + Tailwind + shadcn/ui (JavaScript) — Kanban board UI served by nginx on container port 80, mapped to host 8081. The nginx config also proxies `/api/*` and `/auth/*` to the backend, so a browser only ever needs the frontend origin.
- **Deployment**: Docker + docker-compose, reverse proxy ready.

## Prerequisites

- Docker & Docker Compose
- Outline instance with API key
- Google OAuth credentials

## Setup

### 1. Clone and configure

```bash
git clone <your-repo-url>
cd pdashboard
cp .env.example .env
```

### 2. Get your credentials

**Outline API Key:**
1. Go to your Outline instance → Settings → API & Apps
2. Create a new API key
3. Copy the key (starts with `ol_api_`)

**Google OAuth:**
1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing
3. Configure the OAuth consent screen (External is fine; while in "Testing" mode, add your own Google account as a test user)
4. Go to Credentials → Create Credentials → OAuth client ID → Web application
5. Add an authorized redirect URI matching what you put in `.env`, e.g. `https://your-domain.com/auth/google/callback` for production or `http://localhost:8081/auth/google/callback` for local Docker
6. Copy Client ID and Client Secret

### 3. Configure .env

Edit `.env` with your credentials:

```env
# Outline API
OUTLINE_API_URL=https://your-outline-instance.com/api
OUTLINE_API_KEY=ol_api_your_key_here
# Optional: Specific document ID to fetch tasks from (UUID from document URL)
# If not set, fetches from all documents with tasks
OUTLINE_TASK_DOCUMENT_ID=your-document-uuid-here

# Google OAuth
GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your-client-secret
GOOGLE_REDIRECT_URI=https://your-domain.com/auth/google/callback

# Security
SESSION_SECRET=generate-a-random-secret-string

# Server
PORT=3000
HOST=0.0.0.0
```

To get the document UUID, open the document in Outline and look at the URL:
- URL: `https://notes.febridev.me/doc/task-dX8WroOpOn`
- The UUID is available via the API or in the document properties

Generate a strong session secret, e.g. `openssl rand -hex 32`.

### 4. Deploy with Docker

```bash
docker compose up -d
```

The dashboard will be available at:
- Frontend: `http://localhost:8081` ← open this in your browser
- Backend API: `http://localhost:3001` (also reachable directly; `GET /health` returns `OK`)

You only need the frontend URL: nginx in the frontend container forwards `/api/` and `/auth/` to the backend, so the OAuth callback works on the same origin. Set `GOOGLE_REDIRECT_URI` to the URL you actually browse to (e.g. `http://localhost:8081/auth/google/callback` locally).

## Verifying the deployment

1. The Google console has both redirect URIs registered: `http://localhost:8081/auth/google/callback` (Docker) and `http://localhost:5173/auth/google/callback` (dev).
2. `docker compose up -d --build` succeeds from a cold start.
3. Open `http://localhost:8081`, sign in with Google, and confirm the board renders with tasks.
4. Sign out: the sign-in card reappears. Sign in again and confirm it works.

## Reverse Proxy (Production)

Example nginx configuration:

```nginx
server {
    listen 443 ssl http2;
    server_name dashboard.yourdomain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:8081;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location /api/ {
        proxy_pass http://localhost:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location /auth/ {
        proxy_pass http://localhost:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

Because the frontend container already proxies `/api/` and `/auth/` to the backend, proxying everything to `http://localhost:8081` is enough as well; the split above just routes API traffic directly.

Update `GOOGLE_REDIRECT_URI` in `.env` to match your domain (e.g. `https://dashboard.yourdomain.com/auth/google/callback`).

## Local Development

### Backend

```bash
cd backend
cargo run
```

The backend reads its configuration from the process environment; anything not set is looked up in a `.env` file found by walking up from the current directory (dotenv), so the single root `.env` from the setup steps works here too (only one file is loaded: a `backend/.env`, if present, replaces the root `.env` entirely). Under docker compose, variables come from the root `.env` via `env_file`. Run tests with `cargo test`.

### Frontend

```bash
cd frontend
npm install
npm run dev
```

The Vite dev server runs at `http://localhost:5173` and proxies `/api` and `/auth` to the backend on `:3000`, so run `cargo run` in `backend/` alongside it.

For Google login in dev:
1. Create a gitignored `backend/.env` as a full copy of the root `.env` with only `GOOGLE_REDIRECT_URI` changed to `http://localhost:5173/auth/google/callback`. Dotenv loads exactly one file, so the copy must contain every variable; it does not merge with the root `.env`.
2. Register `http://localhost:5173/auth/google/callback` in the Google console alongside the `8081` one.
3. Run `cargo run` in `backend/`, then open `http://localhost:5173`.

Full-stack behavior without the dev server can always be checked with `docker compose up -d --build` and `http://localhost:8081`.

## Security Notes

- Never commit `.env` to git (it is gitignored; `.env.example` is the committed template)
- Use strong random values for `SESSION_SECRET`
- Restrict Google OAuth to your domain
- Enable HTTPS in production — the session and CSRF cookies are marked `Secure`, so the login flow does not work over plain HTTP in real browsers
- Consider adding rate limiting for API endpoints

## License

MIT
