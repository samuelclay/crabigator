import type { Env } from './types/env';
import type { ErrorResponse } from './types/api';

type Handler = (request: Request, env: Env, params: Record<string, string>) => Promise<Response>;

interface Route {
    method: string;
    pattern: RegExp;
    paramNames: string[];
    handler: Handler;
}

export class Router {
    private routes: Route[] = [];

    private addRoute(method: string, path: string, handler: Handler): void {
        // Convert path pattern to regex
        // e.g., '/api/sessions/:id' -> /^\/api\/sessions\/([^/]+)$/
        const paramNames: string[] = [];
        const regexStr = path.replace(/:([^/]+)/g, (_match, paramName) => {
            paramNames.push(paramName);
            return '([^/]+)';
        });
        const pattern = new RegExp(`^${regexStr}$`);
        this.routes.push({ method, pattern, paramNames, handler });
    }

    get(path: string, handler: Handler): void {
        this.addRoute('GET', path, handler);
    }

    post(path: string, handler: Handler): void {
        this.addRoute('POST', path, handler);
    }

    patch(path: string, handler: Handler): void {
        this.addRoute('PATCH', path, handler);
    }

    delete(path: string, handler: Handler): void {
        this.addRoute('DELETE', path, handler);
    }

    async handle(request: Request, env: Env): Promise<Response> {
        const url = new URL(request.url);
        const method = request.method;
        const path = url.pathname;

        // Handle CORS preflight
        if (method === 'OPTIONS') {
            return this.corsResponse();
        }

        // Find matching route
        for (const route of this.routes) {
            if (route.method !== method) continue;

            const match = path.match(route.pattern);
            if (!match) continue;

            // Extract params
            const params: Record<string, string> = {};
            route.paramNames.forEach((name, index) => {
                params[name] = match[index + 1];
            });

            try {
                const response = await route.handler(request, env, params);
                if (response.status === 101) {
                    return response;
                }
                return this.addCorsHeaders(response);
            } catch (error) {
                console.error('Handler error:', error);
                return this.errorResponse(
                    'Internal server error',
                    'INTERNAL_ERROR',
                    500
                );
            }
        }

        return this.errorResponse('Not found', 'NOT_FOUND', 404);
    }

    private corsResponse(): Response {
        return new Response(null, {
            status: 204,
            headers: this.corsHeaders(),
        });
    }

    private corsHeaders(): HeadersInit {
        return {
            'Access-Control-Allow-Origin': '*',
            'Access-Control-Allow-Methods': 'GET, POST, PATCH, DELETE, OPTIONS',
            'Access-Control-Allow-Headers': 'Content-Type, Authorization, X-Device-Id, X-Signature, X-Timestamp',
            'Access-Control-Max-Age': '86400',
        };
    }

    private addCorsHeaders(response: Response): Response {
        const newHeaders = new Headers(response.headers);
        for (const [key, value] of Object.entries(this.corsHeaders())) {
            newHeaders.set(key, value);
        }
        return new Response(response.body, {
            status: response.status,
            statusText: response.statusText,
            headers: newHeaders,
        });
    }

    errorResponse(error: string, code: string, status: number): Response {
        const body: ErrorResponse = { error, code };
        return new Response(JSON.stringify(body), {
            status,
            headers: {
                'Content-Type': 'application/json',
                ...this.corsHeaders(),
            },
        });
    }
}

export function jsonResponse<T>(data: T, status = 200): Response {
    return new Response(JSON.stringify(data), {
        status,
        headers: { 'Content-Type': 'application/json' },
    });
}
