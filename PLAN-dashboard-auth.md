# Add Server-Side Authentication to Dashboard Endpoints

## Problem

The dashboard API endpoints have no server-side authentication. Anyone can:
- List all sessions: `GET /api/sessions`
- Stream session events: `GET /api/sessions/:id/events`
- Send commands to any session: `POST /api/sessions/:id/answer`, etc.

The pairing code only gates the client-side UI - it doesn't protect the API.

## Solution

Add mobile token verification to all unauthenticated endpoints. The auth infrastructure already exists (`verifyMobileToken` in middleware.ts) - we just need to apply it.

### Key Design Decisions

1. **SSE Auth via Query Param**: EventSource can't set headers, so accept `?token=xxx` for SSE endpoints
2. **Multi-Device Groups**: Mobile token contains `group_id` which can link multiple desktops. Session list shows all sessions from all devices in the user's group.
3. **Session Ownership**: Individual session access verified by checking session's device belongs to same group
4. **Auth Failure Handling**: Return 401 → dashboard clears token and shows pairing gate
5. **Long-Lived Pairing Tokens**: Tokens stay alive for session duration, not 5 minutes. Reusable across multiple devices.

### Session Filtering (Multi-Device)

When a user pairs their phone with Desktop A:
- Desktop A's `group_id` is stored in the mobile token (KV)
- If Desktop B is added to the same group, sessions from both appear
- Filter: `sessions JOIN devices WHERE devices.group_id = mobile_token.group_id`

## Files to Modify

### 1. `workers/crabigator-api/src/auth/middleware.ts`
Add middleware functions:

```typescript
// Extract token from header or query param (for SSE)
function extractToken(request: Request): string | null {
    const authHeader = request.headers.get('Authorization');
    if (authHeader?.startsWith('Bearer ')) {
        return authHeader.slice(7);
    }
    const url = new URL(request.url);
    return url.searchParams.get('token');
}

// Require mobile auth with group_id
export async function requireMobileAuth(request, env): Promise<{ auth: MobileAuth & { group_id: string } } | { error: Response }>

// Verify session belongs to mobile's device group
export async function requireSessionAccess(request, env, sessionId): Promise<{ auth } | { error: Response }>
```

### 2. `workers/crabigator-api/src/index.ts`
Add auth to all unauthenticated session endpoints:

| Endpoint | Auth Needed |
|----------|-------------|
| `GET /api/sessions` | `requireMobileAuth` + filter by group_id |
| `GET /api/sessions/stream` | `requireMobileAuth` + filter by group_id |
| `GET /api/sessions/:id/events` | `requireSessionAccess` |
| `POST /api/sessions/:id/answer` | `requireSessionAccess` |
| `POST /api/sessions/:id/key` | `requireSessionAccess` |
| `POST /api/sessions/:id/draft` | `requireSessionAccess` |
| `GET /api/sessions/:id/draft` | `requireSessionAccess` |
| `GET /api/sessions/:id/state` | `requireSessionAccess` |
| `POST /api/sessions/:id/viewer-active` | `requireSessionAccess` |

### 3. `workers/crabigator-api/src/durable-objects/SessionListDO.ts`
Add group_id filtering:
- Accept `?group_id=xxx` param on `/sessions` and `/subscribe`
- Filter sessions to only those belonging to devices in that group
- Store device's group_id with session data when registered

### 4. Dashboard JS Files

**`src/dashboard/js/pairing.ts`** - Add auth helpers:
```javascript
function getAuthHeaders() {
    const headers = { 'Content-Type': 'application/json' };
    if (mobileToken) {
        headers['Authorization'] = 'Bearer ' + mobileToken;
    }
    return headers;
}

function getAuthQueryParam() {
    return mobileToken ? '?token=' + encodeURIComponent(mobileToken) : '';
}
```

**`src/dashboard/js/session.ts`** - Add headers to `loadSessions()`:
```javascript
const resp = await fetch(API_BASE + '/sessions', { headers: getAuthHeaders() });
if (resp.status === 401) { clearPairing(); return; }
```

**`src/dashboard/js/sse.ts`** - Add token to EventSource:
```javascript
sessionListSource = new EventSource(API_BASE + '/sessions/stream' + getAuthQueryParam());
```

**`src/dashboard/js/events.ts`** - Add token to session EventSource:
```javascript
const eventSource = new EventSource(API_BASE + '/sessions/' + sessionId + '/events' + getAuthQueryParam());
```

**Other files needing auth headers**: prompt.ts, input.ts (draft save/load)

### 5. `workers/crabigator-api/src/handlers/pairing.ts`
Fix pairing token behavior:
- **Remove 5-minute TTL** - token lives as long as session is active
- **Allow reuse** - don't mark as "claimed" after first use
- Token cleanup: delete when session ends (via desktop disconnect)

## Implementation Order

1. Add `extractToken`, `requireMobileAuth`, `requireSessionAccess` to middleware.ts
2. Update SessionListDO to accept and filter by group_id
3. Add auth checks to all endpoints in index.ts
4. Add `getAuthHeaders()` and `getAuthQueryParam()` to pairing.ts
5. Update all dashboard JS fetch/EventSource calls
6. Add 401 handling to clear pairing on auth failure
7. Fix pairing token TTL and reusability in pairing.ts

## Verification

1. Clear localStorage (`clearPairing()` or DevTools)
2. Visit dashboard - should show pairing gate
3. Try direct API call without token: `curl https://drinkcrabigator.com/api/sessions` → 401
4. Pair with valid code
5. Dashboard should show only your sessions
6. Try accessing another user's session ID → 403
7. Verify pairing token works across multiple devices/browsers
8. Deploy and verify sessions reappear (reconnection fix)
