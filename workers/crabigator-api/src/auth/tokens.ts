/**
 * Hash a string using SHA-256
 */
export async function sha256(data: string): Promise<string> {
    const encoder = new TextEncoder();
    const dataBuffer = encoder.encode(data);
    const hashBuffer = await crypto.subtle.digest('SHA-256', dataBuffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Generate HMAC-SHA256 signature
 */
export async function hmacSign(secret: string, message: string): Promise<string> {
    const encoder = new TextEncoder();
    const keyData = encoder.encode(secret);
    const messageData = encoder.encode(message);

    const key = await crypto.subtle.importKey(
        'raw',
        keyData,
        { name: 'HMAC', hash: 'SHA-256' },
        false,
        ['sign']
    );

    const signature = await crypto.subtle.sign('HMAC', key, messageData);
    const signatureArray = Array.from(new Uint8Array(signature));
    return signatureArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Verify HMAC-SHA256 signature
 */
export async function hmacVerify(secret: string, message: string, signature: string): Promise<boolean> {
    const expectedSignature = await hmacSign(secret, message);
    return timingSafeEqual(expectedSignature, signature);
}

/**
 * Timing-safe string comparison
 */
function timingSafeEqual(a: string, b: string): boolean {
    if (a.length !== b.length) {
        return false;
    }
    let result = 0;
    for (let i = 0; i < a.length; i++) {
        result |= a.charCodeAt(i) ^ b.charCodeAt(i);
    }
    return result === 0;
}

/**
 * Generate a random token
 */
export function generateToken(bytes = 32): string {
    const array = new Uint8Array(bytes);
    crypto.getRandomValues(array);
    return Array.from(array)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');
}

/**
 * Generate a UUID v4
 */
export function generateUUID(): string {
    return crypto.randomUUID();
}

/**
 * Generate a short pairing code (human-readable)
 */
export function generatePairingCode(): string {
    const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'; // Exclude confusing chars
    let code = '';
    const array = new Uint8Array(9);
    crypto.getRandomValues(array);
    for (let i = 0; i < 9; i++) {
        if (i === 3 || i === 6) {
            code += '-';
        }
        code += chars[array[i] % chars.length];
    }
    return code;
}
