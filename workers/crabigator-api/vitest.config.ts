import { cloudflareTest } from '@cloudflare/vitest-pool-workers';
import { defineConfig } from 'vitest/config';

export default defineConfig({
    plugins: [
        cloudflareTest({
            main: './src/index.ts',
            wrangler: { configPath: './wrangler.example.jsonc' },
            miniflare: {
                bindings: {
                    APP_CONFIG: {
                        public_origin: '',
                        features: {
                            transcription: false,
                            billing: false,
                            gifts: false,
                            outbound_email: false,
                            marketing_analytics: false,
                            traffic_alerts: false,
                            staff: true,
                        },
                        billing: {
                            visible_session_limit: 7,
                            price_display: '€5',
                            price_period: 'per week',
                        },
                    },
                    STAFF_ACCESS_KEY: 'test-only-staff-access-key',
                },
            },
        }),
    ],
    test: { globals: false },
});
