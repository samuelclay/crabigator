package com.crabigator.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import org.json.JSONObject

/**
 * Debug builds only. Shows the app with a sample roster, without pairing and without sending
 * a request, so every screen can be checked and photographed on an emulator:
 *
 *     adb shell am start -n com.crabigator.app/.PreviewActivity [--es mode board] [--ez session true] [--ez expanded true] [--ez reset true]
 *
 * `mode board` opens the PR board, `session` opens the first session, `expanded` keeps a tablet's
 * board at full width beside it, and `reset` returns every saved view choice to its default.
 */
class PreviewActivity : ComponentActivity() {
    private val model: SessionModel by viewModels()
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState); enableEdgeToEdge(statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT), navigationBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT))
        if (savedInstanceState == null) {
            val now = System.currentTimeMillis() / 1000
            val sessions = sessions(now)
            val selected = sessions.first().takeIf { intent.getBooleanExtra("session", false) }
            if (intent.getBooleanExtra("reset", false)) model.style { UiPreferences() }
            model.navigate { NavigationState(listMode = intent.getStringExtra("mode") != "board", expanded = selected == null || intent.getBooleanExtra("expanded", false), selectedId = selected?.id) }
            // The model is never started, so it polls nothing and opens no socket.
            model.state.value = AppState(paired = true, sessions = sessions, prs = pullRequests(sessions), rosterStatus = RosterStatus.Ready, selected = selected,
                screen = SCREEN, historyLoaded = true, screenLoaded = true, connected = true, desktopStatusKnown = true, details = details(now))
        }
        val account = JSONObject("""{"identities":[{"provider":"github","email":"you@example.com"}]}""")
        setContent { MaterialTheme(colorScheme = CrabTheme) { Surface(Modifier.fillMaxSize()) { CompositionLocalProvider(LocalAccountSource provides { account }) { App(model) {} } } } }
    }

    private fun session(id: String, title: String, repo: String, machine: String, state: String, glyph: String, color: String, background: String, branch: String, prompt: Long, stats: SessionStats, recap: String = "", uncommitted: Int = 0, additions: Int = 0, deletions: Int = 0, active: Boolean = true) =
        Session(id, title, repo, machine, state, active, glyph, color, recap, "claude", branch, background, stats.copy(promptAt = prompt), "/projects/${repo.substringAfter('/')}", prompt, uncommitted, additions, deletions)

    private fun sessions(now: Long) = listOf(
        session("preview-1", "Redesign the settings screen", "crabigator/crabigator", "ClayMac Studio", "thinking", "◈", "#FFAA77", "#352B25", "main", now - 180,
            SessionStats(startedAt = now - 8040, thinkingSeconds = 2280, prompts = 14, completions = 13, completionAt = now - 600, tools = 212, compactions = 3, context = ContextUsage(186_000, 1_000_000)),
            "Mapped every setting and drew four ways to arrange them.", 3, 212, 48),
        session("preview-2", "Live updates for mobile", "newsblur/newsblur", "ClayMac Studio", "complete", "◆", "#af87ff", "#2A2440", "mobile-realtime", now - 1500,
            SessionStats(startedAt = now - 15300, thinkingSeconds = 5400, prompts = 31, completions = 31, completionAt = now - 1440, tools = 640, compactions = 1, context = ContextUsage(742_000, 1_000_000)),
            "Stories now arrive over the socket and a pill announces them.", 5, 9036, 117),
        session("preview-3", "Which theme should be the default?", "crabigator/crabigator", "ClayBook M4", "question", "▲", "#7ee787", "#1F3324", "dark-mode", now - 2700,
            SessionStats(startedAt = now - 5200, thinkingSeconds = 900, prompts = 6, completions = 5, completionAt = now - 2600, tools = 88, compactions = 0, context = ContextUsage(92_000, 200_000)),
            "Added a dark theme and asked which one opens first."),
        session("preview-4", "Fix the flaky feed fetcher test", "newsblur/newsblur", "ClayBook M4", "complete", "●", "#ffd75f", "#33301F", "fetcher-retries", now - 9000,
            SessionStats(startedAt = now - 12000, thinkingSeconds = 1500, prompts = 9, completions = 9, completionAt = now - 8700, tools = 140, compactions = 0, context = ContextUsage(180_000, 200_000)),
            "The retry now waits for the socket to close before it reconnects.", 1, 18, 4),
        session("preview-5", "Port the balloon scene", "crabigator/crabigator", "ClayMac Studio", "permission", "◇", "#5fd7ff", "#1F2E33", "flow-balloon", now - 13000,
            SessionStats(startedAt = now - 20000, thinkingSeconds = 4100, prompts = 12, completions = 11, completionAt = now - 12800, tools = 301, compactions = 2)),
        session("preview-6", "Write the release notes", "crabigator/crabigator", "ClayBook M4", "complete", "■", "#ff87d7", "#33202D", "main", now - 40000,
            SessionStats(startedAt = now - 43000, thinkingSeconds = 600, prompts = 3, completions = 3, completionAt = now - 39800, tools = 22, compactions = 0), active = false),
    )

    private fun pullRequests(sessions: List<Session>) = listOf(
        PullRequest("newsblur/newsblur#2217", "Live updates and a new stories pill", "newsblur/newsblur", 2217, "OPEN", "●1 CI", "", listOf(sessions[1]), branch = "mobile-realtime", additions = 9036, deletions = 117, mergeable = "MERGEABLE", primary = true, checksPending = 1),
        PullRequest("crabigator/crabigator#412", "Add a dark theme", "crabigator/crabigator", 412, "OPEN", "✓ CI", "", listOf(sessions[2]), branch = "dark-mode", additions = 340, deletions = 22, mergeable = "MERGEABLE", primary = true, review = "APPROVED"),
        PullRequest("newsblur/newsblur#2209", "Retry the feed fetcher after a closed socket", "newsblur/newsblur", 2209, "OPEN", "✗2 CI", "", listOf(sessions[3]), branch = "fetcher-retries", additions = 18, deletions = 4, mergeable = "CONFLICTING", primary = true, checksFailed = 2, unresolvedComments = 3),
    )

    private fun details(now: Long) = mapOf(
        "stats" to JSONObject().put("work_seconds", 8040).put("thinking_seconds", 2280).put("prompts", 14).put("completions", 13).put("tools", 212).put("compressions", 3)
            .put("context_used", 186_000).put("context_limit", 1_000_000).put("prompts_changed_at", now - 180).put("completions_changed_at", now - 600),
        "git" to JSONObject("""{"branch":"main","is_repo":true,"files":[{"path":"clients/android/MenuPanel.kt","status":"M","additions":180,"deletions":40},{"path":"clients/android/ViewPanel.kt","status":"A","additions":30,"deletions":0},{"path":"clients/android/README.md","status":"M","additions":2,"deletions":8}]}"""),
        "recap" to JSONObject().put("latest", JSONObject().put("headline", "Mapped every setting and drew four ways to arrange them.").put("generated_at", (now - 600) * 1000)),
    )

    private companion object {
        const val SCREEN = "\u001b[32m●\u001b[0m Read MenuPanel.kt\n  Mapped every setting on the screen.\n\n\u001b[32m●\u001b[0m Update ViewPanel.kt\n  Drew each choice as a tile.\n\n\u001b[33m✻\u001b[0m Checking the row on a phone…\n"
    }
}
