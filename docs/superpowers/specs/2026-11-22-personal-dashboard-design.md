# Personal Dashboard Design

## Intent

Build a personal dashboard using Rust (Dioxus + Axum) that aggregates data from multiple APIs, starting with a Kanban board showing tasks from Outline. The dashboard will be self-hosted on a VPS with Docker, accessible via reverse proxy with OAuth authentication.

## Goals

- **First module**: Kanban board displaying Outline tasks in "Pending" and "Completed" columns
- **Extensible architecture**: Easy to add more modules (calendar, email, etc.) in the future
- **Secure**: OAuth authentication for dashboard access, API keys in environment variables only
- **Dockerized**: Single docker-compose setup for easy VPS deployment
- **Clean UI**: Light color palette, modern and readable

## Non-Goals

- Building all dashboard modules now (just the Kanban for v1)
- Implementing OAuth provider (we're consuming OAuth, not providing it)
- Public-facing access (personal dashboard with authentication)

## Anti-Goals

- Committing API keys or secrets to version control
- Complex deployment setup (should be `docker-compose up`)
- Heavy dependencies or over-engineering

## Constraints

- **Framework**: Dioxus for frontend, Axum for backend
- **Deployment**: Docker on VPS, reverse proxy (nginx/caddy) with custom domain
- **Authentication**: OAuth for dashboard access
- **API Source**: Outline instance at https://notes.febridev.me/
- **Styling**: Light color palette

## Architecture

### Tech Stack

- **Frontend**: Dioxus (Rust web framework) with Tailwind CSS for styling
- **Backend**: Axum (Rust web framework) as API proxy
- **Database**: None for v1 (tasks fetched live from Outline)
- **Deployment**: Docker + docker-compose

### Project Structure

```
pdashboard/
├── backend/              # Axum API server
│   ├── src/
│   │   ├── main.rs      # Server entry point
│   │   ├── config.rs    # Configuration (env vars)
│   │   ├── routes/      # API routes
│   │   │   ├── mod.rs
│   │   │   ├── tasks.rs # Task endpoints
│   │   │   └── auth.rs  # OAuth routes
│   │   ├── services/    # Business logic
│   │   │   ├── mod.rs
│   │   │   ├── outline.rs # Outline API client
│   │   │   └── parser.rs  # Markdown task parser
│   │   └── models/      # Data structures
│   │       ├── mod.rs
│   │       └── task.rs
│   ├── Cargo.toml
│   └── Dockerfile
├── frontend/             # Dioxus web app
│   ├── src/
│   │   ├── main.rs      # App entry point
│   │   ├── components/  # UI components
│   │   │   ├── mod.rs
│   │   │   ├── kanban.rs
│   │   │   └── task_card.rs
│   │   └── api/         # API client
│   │       └── mod.rs
│   ├── Cargo.toml
│   ├── index.html
│   └── Dockerfile
├── docker-compose.yml
├── .env.example         # Template for environment variables
└── README.md
```

### Data Flow

1. **Task Extraction**: 
   - Backend calls `POST /api/documents.list` to Outline
   - For each document with tasks, fetch full content via `POST /api/documents.info`
   - Parse markdown to extract checkbox items (`- [ ]` and `- [x]`)
   - Return structured task list via `/api/tasks` endpoint

2. **Frontend Display**:
   - Dioxus app calls backend `/api/tasks` endpoint
   - Tasks split into "Pending" and "Completed" columns
   - Kanban board renders with task cards

3. **Authentication**:
   - Google OAuth 2.0 flow for dashboard access
   - Session management via secure cookies
   - Protected API routes
   - Optional: Restrict access to specific Google accounts via `ALLOWED_EMAILS`

### API Endpoints

**Backend (Axum)**:
- `GET /api/tasks` - List all tasks from Outline (grouped by header)
- `GET /api/tasks/pending` - Pending tasks only
- `GET /api/tasks/completed` - Completed tasks only
- `GET /auth/google` - Initiate Google OAuth flow
- `GET /auth/google/callback` - Google OAuth callback handler
- `GET /auth/logout` - Logout user
- `/*` - Serve Dioxus frontend (SSR or static assets)

**Outline API Integration**:
- Base URL: `https://notes.febridev.me/api/`
- Auth: Bearer token (`ol_api_*`)
- Key endpoints:
  - `POST /documents.list` - List documents
  - `POST /documents.info` - Get document with markdown content

### Task Parsing Logic

Outline stores tasks as markdown checkboxes within documents:
```markdown
# Project Alpha
## TODO
- [ ] Buy groceries
- [ ] Call dentist

## Completed
- [x] Finish report
- [x] Send email
```

The parser will:
1. Split markdown by lines
2. Track current header context (H1, H2, H3, etc.)
3. Match lines starting with `- [ ]` (pending) or `- [x]` (completed)
4. Extract task text and associate with current header
5. Structure: `{ document_title, header_path, tasks: [{ text, status }] }`

Example output:
```json
{
  "document_title": "Project Alpha",
  "groups": [
    {
      "header": "TODO",
      "tasks": [
        { "text": "Buy groceries", "status": "pending" },
        { "text": "Call dentist", "status": "pending" }
      ]
    },
    {
      "header": "Completed",
      "tasks": [
        { "text": "Finish report", "status": "completed" },
        { "text": "Send email", "status": "completed" }
      ]
    }
  ]
}
```

### Configuration

Environment variables (via `.env` file):
```env
# Outline API
OUTLINE_API_URL=https://notes.febridev.me/api
OUTLINE_API_KEY=ol_api_xxx

# Google OAuth
GOOGLE_CLIENT_ID=xxx.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=xxx
GOOGLE_REDIRECT_URI=https://dashboard.yourdomain.com/auth/callback

# Server
PORT=3000
HOST=0.0.0.0

# Security
SESSION_SECRET=xxx
ALLOWED_EMAILS=user@gmail.com  # Optional: restrict to specific Google accounts
```

## Deployment

### Docker Setup

**backend/Dockerfile**:
- Multi-stage build for Rust
- Compile backend binary
- Minimal runtime image

**frontend/Dockerfile**:
- Build Dioxus assets
- Serve via Axum or separate nginx container

**docker-compose.yml**:
```yaml
version: '3.8'
services:
  app:
    build: .
    ports:
      - "3000:3000"
    env_file:
      - .env
    restart: unless-stopped
```

### Reverse Proxy

Example nginx config:
```nginx
server {
    listen 443 ssl;
    server_name dashboard.yourdomain.com;
    
    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Styling

**Light Color Palette**:
- Background: `#f8f9fa` (light gray)
- Cards: `#ffffff` (white)
- Primary: `#3b82f6` (blue)
- Text: `#1f2937` (dark gray)
- Borders: `#e5e7eb` (light border)
- Success (completed): `#10b981` (green)
- Pending: `#f59e0b` (amber)

**UI Components**:
- Clean card-based design
- Rounded corners
- Subtle shadows
- Responsive layout

## Security Considerations

1. **API Keys**: Never commit to git, use `.env` file
2. **OAuth**: Validate state parameter, use HTTPS only
3. **Sessions**: Secure, HttpOnly cookies or JWT with expiration
4. **CORS**: Restrict to your domain
5. **Rate Limiting**: Consider for API endpoints

## Future Extensions

The architecture should support adding:
- Calendar module (Google Calendar, CalDAV)
- Email summary (Gmail, IMAP)
- GitHub notifications
- Weather widget
- Custom widgets

Each module would follow the same pattern:
1. Backend service to fetch/transform data
2. API endpoint
3. Frontend component
4. Dashboard integration

## Implementation Phases

### Phase 1: Backend Foundation
1. Setup Axum server
2. Outline API client
3. Markdown task parser
4. `/api/tasks` endpoint

### Phase 2: Frontend
1. Dioxus project setup
2. Kanban component
3. Task card component
4. API integration

### Phase 3: Authentication
1. OAuth flow
2. Session management
3. Protected routes

### Phase 4: Deployment
1. Dockerfiles
2. docker-compose setup
3. Documentation

## Decisions Made

- **OAuth Provider**: Google OAuth
- **Task Behavior**: Read-only view (no ability to mark tasks complete from dashboard)
- **Task Grouping**: By header (tasks grouped by the markdown header they appear under within documents)

### Task Grouping Example

Given this Outline document:
```markdown
# Project Alpha
## TODO
- [ ] Setup database
- [ ] Design API

## Completed
- [x] Initial planning
```

The Kanban would display:
- **Project Alpha > TODO**: Setup database, Design API
- **Project Alpha > Completed**: Initial planning

## Open Questions

- **Real-time Updates**: Poll for updates, or WebSocket for live changes? (Recommendation: Start with polling, add WebSocket later if needed)
