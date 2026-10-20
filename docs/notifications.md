# Notification history

Mobile push sends are recorded in D1 after migration `0024_notification_history.sql` is applied and the updated Worker is deployed. Earlier sends cannot be reconstructed from this history.

Each `notification_deliveries` row identifies the account group, mobile device, session, notification identity, prompt revision, session title, available question content, and timestamps. Each `notification_attempts` row records a provider request, its token-free payload, HTTP status, provider receipt ID, and outcome. Times are Unix milliseconds. History has no automatic expiry and survives deletion of sessions and device registrations.

An `accepted` result means APNs or FCM accepted the request. It does not prove that the device received or displayed it. `rejected` means the provider refused it. `unknown` means the response was lost. `reserved` or `sending` can remain after an interruption; those entries must not be assumed unsent. `superseded` means the prompt changed before a request was made, and `failed` means preparation failed.

Apple alerts use one identity for a question through cursor movement, checkbox changes, typing, question-page navigation, temporary screen clears, and reconnects. A different question or a later user turn gets a new identity. Identical prompts repeated within the same user turn are deduplicated: the cloud prompt protocol does not yet carry a tool invocation ID to distinguish a deliberate repeat from a repaint. The first generic alert and its later structured question share an identity. Its stored question content can be empty if the desktop had not supplied it when the send began.

Android pushes refresh the existing notification, so their identity follows the prompt revision. Apple resolution pushes are silent. Both platforms reserve each send in D1 before contacting the provider. Accepted and uncertain sends are never retried. FCM retries definite transient rejections within the existing send deadline, recording each attempt. A reservation that failed or became stale before any provider attempt can be reclaimed.

The payload is the message sent to the provider. Apple’s notification extension fetches current content when the alert arrives, so this audit does not claim to capture the exact text displayed on the device. Push tokens, authorization headers, and manually typed answers are excluded from the audit.

## Read history

Use the authenticated account endpoint:

```http
GET /api/mobile/notifications
GET /api/mobile/notifications?session_id=SESSION_ID
GET /api/mobile/notifications?mobile_id=MOBILE_ID&before=CURSOR
```

The response contains up to 50 `notifications`, each with its `attempts`, and a `next_cursor`. Pass that cursor as `before` for the next page. Access uses the same mobile bearer token or dashboard authentication as other mobile endpoints. Results are restricted to the caller’s account group. Device IDs remain available after a registration is removed; current device names can be looked up in `linked_devices`.

Operators can also query D1 directly:

```sql
SELECT d.session_id, d.mobile_id, d.kind, d.prompt_revision, d.status,
       datetime(d.created_at / 1000, 'unixepoch') AS created_utc,
       a.http_status, a.provider_id, a.provider_reason
FROM notification_deliveries d
LEFT JOIN notification_attempts a ON a.delivery_id = d.id
WHERE d.mobile_id = 'MOBILE_ID'
ORDER BY d.created_at DESC, a.started_at DESC
LIMIT 100;
```

Apply the migration before deploying the Worker. The send path deliberately requires a durable audit reservation; it does not send unrecorded notifications if D1 is unavailable.
