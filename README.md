# Personal Dashboard

A personal dashboard built with Rust (Dioxus + Axum) that displays tasks from Outline in a Kanban board.

## Features

- Kanban board with Pending and Completed columns
- Tasks fetched from Outline API and grouped by document/header
- Google OAuth authentication
- Docker deployment ready

## Architecture

- **Backend** (`backend/`): Axum (Rust) — Outline API integration, markdown task parser, Google OAuth. Container listens on port 3000, mapped to host 3001.
- **Frontend** (`frontend/`): Dioxus (Rust/WASM) — Kanban board UI, served by nginx on container port 80, mapped to host 8081. The nginx config also proxies `/api/*` and `/auth/*` to the backend, so a browser only ever needs the frontend origin.
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

Generate a strong session secret, e.g. `openssl rand -hex 32`.

### 4. Deploy with Docker

```bash
docker compose up -d
```

The dashboard will be available at:
- Frontend: `http://localhost:8081` ← open this in your browser
- Backend API: `http://localhost:3001` (also reachable directly; `GET /health` returns `OK`)

You only need the frontend URL: nginx in the frontend container forwards `/api/` and `/auth/` to the backend, so the OAuth callback works on the same origin. Set `GOOGLE_REDIRECT_URI` to the URL you actually browse to (e.g. `http://localhost:8081/auth/google/callback` locally).

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

The backend reads its configuration from the process environment; anything not set is looked up in a `.env` file found by walking up from the current directory (dotenv), so the single root `.env` from the setup steps works here too (a `backend/.env` overrides it). Under docker compose, variables come from the root `.env` via `env_file`. Run tests with `cargo test`.

### Frontend

```bash
cd frontend
dx serve
```

Note: the frontend calls `/api/tasks` on its own origin, and no dev-server proxy is configured, so for full-stack behavior (API + OAuth) use `docker compose up` and browse to `http://localhost:8081`.

## Security Notes

- Never commit `.env` to git (it is gitignored; `.env.example` is the committed template)
- Use strong random values for `SESSION_SECRET`
- Restrict Google OAuth to your domain
- Enable HTTPS in production — the session and CSRF cookies are marked `Secure`, so the login flow does not work over plain HTTP in real browsers
- Consider adding rate limiting for API endpoints

## License

MIT
