package com.crabigator.app

internal enum class BoardAge(val label: String, val palette: Int) {
    Hour("Last hour", 51), Three("1–3 hours", 45), Six("3–6 hours", 39), Nine("6–9 hours", 33),
    Twelve("9–12 hours", 27), Day("12–24 hours", 25), Older("Older", 245);

    companion object {
        fun fromSeconds(seconds: Long) = when {
            seconds < 3600 -> Hour
            seconds < 10800 -> Three
            seconds < 21600 -> Six
            seconds < 32400 -> Nine
            seconds < 43200 -> Twelve
            seconds <= 86400 -> Day
            else -> Older
        }
    }
}

internal data class BoardSession(val session: Session, val prs: List<PullRequest>, val activity: Long) {
    val repo get() = prs.firstOrNull()?.repo ?: session.repo.ifBlank { "Sessions" }
}
internal data class BoardPRGroup(val pr: PullRequest, val sessions: List<BoardSession>) {
    val activity: Long get() {
        val prompted = sessions.maxOfOrNull {
            if (it.session.active && it.session.state in listOf("thinking", "question", "permission")) it.activity
            else it.session.stats.promptAt
        } ?: 0
        return prompted.takeIf { it > 0 } ?: maxOf(pr.updatedAt, pr.closedAt, pr.lastMentionedAt)
    }
}
internal data class BoardRepository(val repo: String, val sessions: List<BoardSession>, val pullRequests: List<BoardPRGroup> = emptyList())
internal data class BoardBand(val age: BoardAge, val repositories: List<BoardRepository>)
internal data class BoardMachine(val machine: String, val bands: List<BoardBand>) {
    val sessions get() = bands.flatMap { it.repositories }.flatMap { it.pullRequests.flatMap { pr -> pr.sessions } + it.sessions }.distinctBy { it.session.id }
}

private val PullRequest.attentionRank get() = when {
    state.isEmpty() -> 2
    state == "MERGED" -> 6
    state != "OPEN" -> 7
    mergeable == "CONFLICTING" || checksFailed > 0 -> 0
    review == "CHANGES_REQUESTED" -> 1
    draft -> 2
    checksPending > 0 -> 3
    review != "APPROVED" -> 4
    else -> 5
}

/** Match the CLI session board: work in flight uses now; completions never move a settled row. */
internal fun prBoardGroups(roster: List<Session>, prs: List<PullRequest>, machines: List<String>, now: Long, showEnded: Boolean, query: String): List<BoardMachine> {
    val prOrder = compareBy<PullRequest> { it.attentionRank }.thenByDescending { it.primary }.thenByDescending { it.lastMentionedAt }
    val bySession = prs.flatMap { pr -> pr.sessions.map { it.id to pr } }.groupBy({ it.first }, { it.second })
    val rows = (roster + prs.flatMap { it.sessions }).distinctBy { it.id }.filter { showEnded || it.active }.mapNotNull { session ->
        val local = bySession[session.id].orEmpty().distinctBy { it.key }.sortedWith(prOrder)
        if (!("${session.title} ${session.repo} ${session.branch} ${session.machine}".contains(query, true) ||
                local.any { "${it.title} ${it.repo} ${it.number}".contains(query, true) })) return@mapNotNull null
        val working = session.active && session.state in listOf("thinking", "question", "permission")
        val activity = if (working) now else session.stats.promptAt.takeIf { it > 0 }
            ?: local.maxOfOrNull { maxOf(it.updatedAt, it.closedAt, it.lastMentionedAt) } ?: 0
        BoardSession(session, local, activity)
    }.groupBy { it.session.machineGroup }
    return machines.mapNotNull { machine ->
        val sessions = rows[machine] ?: return@mapNotNull null
        val bands = sessions.groupBy { BoardAge.fromSeconds((now - it.activity).coerceAtLeast(0)) }.toSortedMap().map { (age, entries) ->
            val repos = entries.groupBy { it.repo }.map { (repo, values) ->
                BoardRepository(repo, values.sortedByDescending { it.activity })
            }.sortedByDescending { it.sessions.first().activity }
            BoardBand(age, repos)
        }
        BoardMachine(machine, bands)
    }
}


/** Each primary PR appears once per computer, with every visible touching session beneath it. */
internal fun groupBoardByPullRequest(machines: List<BoardMachine>, now: Long,
    includesActivity: (Long) -> Boolean = { true }): List<BoardMachine> = machines.mapNotNull { machine ->
    val groups = linkedMapOf<String, BoardPRGroup>()
    val represented = mutableSetOf<String>()
    machine.sessions.forEach { entry ->
        entry.prs.filter { (it.primary || it.watched) && it.visibleOnBoard(entry.session.id) }.forEach { pr ->
            represented.add(entry.session.id)
            val previous = groups[pr.key]
            groups[pr.key] = BoardPRGroup(pr, previous?.sessions.orEmpty() + entry)
        }
    }
    val bands = BoardAge.entries.mapNotNull { age ->
        val loose = machine.bands.firstOrNull { it.age == age }?.repositories.orEmpty()
            .map { it.copy(sessions = it.sessions.filter { entry -> entry.session.id !in represented && includesActivity(entry.activity) }) }
        val requests = groups.values.filter { includesActivity(it.activity) && BoardAge.fromSeconds((now - it.activity).coerceAtLeast(0)) == age }.groupBy { it.pr.repo }
        val repos = (loose.map { it.repo } + requests.keys).distinct().mapNotNull { repo ->
            val sessions = loose.firstOrNull { it.repo == repo }?.sessions.orEmpty()
            val prs = requests[repo].orEmpty()
            if (sessions.isEmpty() && prs.isEmpty()) null else BoardRepository(repo, sessions, prs)
        }
        if (repos.isEmpty()) null else BoardBand(age, repos)
    }
    if (bands.isEmpty()) null else machine.copy(bands = bands)
}
