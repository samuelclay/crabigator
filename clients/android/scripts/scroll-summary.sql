-- Run with Perfetto trace_processor query -f scroll-summary.sql TRACE.
-- UI-thread drawing/measurement stalls. Times include children; do not add rows.
SELECT s.name, count(*) AS calls,
       round(sum(s.dur) / 1e6, 2) AS total_ms,
       round(max(s.dur) / 1e6, 2) AS max_ms
FROM slice s
JOIN thread_track tt ON s.track_id = tt.id
JOIN thread t ON tt.utid = t.utid
JOIN process p ON t.upid = p.upid
WHERE p.name = 'com.crabigator.app' AND t.tid = p.pid AND s.dur > 0
  AND (s.name IN ('Record View#draw()', 'AndroidOwner:measureAndLayout', 'Compose:recompose')
       OR s.name LIKE 'Text%measure%')
GROUP BY s.name ORDER BY total_ms DESC;

SELECT jank_type, count(*) AS frames
FROM actual_frame_timeline_slice
JOIN process USING (upid)
WHERE process.name = 'com.crabigator.app'
GROUP BY jank_type ORDER BY frames DESC;
