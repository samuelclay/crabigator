import type { SessionInfo, SessionState } from './session';

// ============================================
// Device Registration
// ============================================

export interface RegisterDeviceRequest {
    device_id: string;
    secret_hash: string;
    name?: string;
}

export interface RegisterDeviceResponse {
    ok: true;
}

export interface HeartbeatResponse {
    ok: true;
    last_seen_at: number;
}

// ============================================
// Pairing
// ============================================

export interface GeneratePairingTokenResponse {
    token: string;
    expires_at: number;
    qr_data: string;        // Full QR code data string
}

export interface ClaimPairingTokenRequest {
    device_id: string;
    pairing_token: string;
    mobile_id: string;
    mobile_name?: string;
}

export interface ClaimPairingTokenResponse {
    mobile_token: string;
    desktop_name: string | null;
}

export interface LinkedDevice {
    mobile_id: string;
    mobile_name: string | null;
    paired_at: number;
}

export interface PairingStatusResponse {
    linked: LinkedDevice[];
}

// ============================================
// Sessions
// ============================================

export interface CreateSessionRequest {
    client_session_id: string;
    cwd: string;
    platform: 'claude' | 'codex';
}

export interface CreateSessionResponse {
    id: string;             // Cloud session ID
    ws_url: string;         // WebSocket URL for events
}

export interface ListSessionsResponse {
    sessions: SessionInfo[];
}

export interface GetSessionResponse extends SessionInfo {
    share_url: string | null;
}

export interface UpdateSessionRequest {
    ended_at?: number;
    state?: SessionState;
    stats?: {
        prompts?: number;
        completions?: number;
        tool_calls?: number;
        thinking_seconds?: number;
    };
}

export interface UpdateSessionResponse {
    ok: true;
}

// ============================================
// Sharing
// ============================================

export interface GenerateShareResponse {
    share_token: string;
    share_url: string;
}

export interface SharedSessionResponse {
    id: string;
    cwd: string;
    platform: 'claude' | 'codex';
    state: SessionState;
    started_at: number;
    is_active: boolean;
}

// ============================================
// Answers
// ============================================

export interface SendAnswerRequest {
    text: string;
}

export interface SendAnswerResponse {
    ok: true;
}

// ============================================
// Errors
// ============================================

export interface ErrorResponse {
    error: string;
    code: string;
    details?: unknown;
}

// ============================================
// Auth types
// ============================================

export interface DeviceAuth {
    type: 'device';
    device_id: string;
}

export interface MobileAuth {
    type: 'mobile';
    desktop_id: string;
    mobile_id: string;
}

export interface ShareAuth {
    type: 'share';
    session_id: string;
}

export type AuthContext = DeviceAuth | MobileAuth | ShareAuth;
