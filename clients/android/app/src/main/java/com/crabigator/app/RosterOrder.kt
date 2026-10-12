package com.crabigator.app

/** Refresh positions every ten minutes; always render the latest row contents. */
internal class RosterOrder {
    private var refreshedAt: Long? = null
    private var preferences: Pair<String, String>? = null
    private var layout = RosterLayout(emptyList(), emptyList(), emptyList())

    fun update(roster: List<Session>, prs: List<PullRequest>, machines: List<String>, now: Long,
        elapsedSeconds: Long, grouping: String, order: String): RosterLayout {
        val reset = refreshedAt == null || elapsedSeconds - refreshedAt!! >= 600 || preferences != (grouping to order)
        val sessions = roster.sortedWith(compareByDescending<Session> { it.attention }
            .thenByDescending { it.activityAt }.thenBy { it.id })
        val list = sessionListGroups(sessions, machines, grouping, order)
        val board = prBoardGroups(roster, prs, machines, now, true, "")
        val watched = prs.filter { it.sessions.isEmpty() }
        val machineRanks = machines.withIndex().associate { it.value to it.index }
        layout = if (reset) RosterLayout(list, board, watched) else RosterLayout(
            retainSections(layout.list.map { (it.machine to it.project) to it.sessions },
                list.map { (it.machine to it.project) to it.sessions }, { it.id })
                .groupBy { it.first.first }.entries.sortedBy { machineRanks[it.key] ?: Int.MAX_VALUE }.flatMap { (_, groups) ->
                    groups.map { (key, rows) -> SessionListGroup(key.first, key.second, rows) }
                },
            retainSections(layout.board.sections(), board.sections(), { it.session.id }) { old, live ->
                live.copy(prs = retainRows(old.prs, live.prs) { it.key })
            }.groupBy { it.first.machine }.entries.sortedBy { machineRanks[it.key] ?: Int.MAX_VALUE }.map { (machine, sections) ->
                BoardMachine(machine, sections.groupBy { it.first.age }.map { (age, repos) ->
                    BoardBand(age, repos.map { (key, rows) -> BoardRepository(key.repo, rows) })
                })
            },
            retainRows(layout.watched, watched) { it.key },
        )
        if (reset) { refreshedAt = elapsedSeconds; preferences = grouping to order }
        return layout
    }
}

internal data class RosterLayout(val list: List<SessionListGroup>, val board: List<BoardMachine>, val watched: List<PullRequest>) {
    fun filtered(showEnded: Boolean, query: String): RosterLayout {
        fun visible(session: Session) = showEnded || session.active
        fun matches(session: Session) = "${session.title} ${session.repo} ${session.branch} ${session.machine}".contains(query, true)
        fun matches(pr: PullRequest) = "${pr.title} ${pr.repo} ${pr.number}".contains(query, true)
        return RosterLayout(
            list.map { it.copy(sessions = it.sessions.filter { session -> visible(session) && matches(session) }) }.filter { it.sessions.isNotEmpty() },
            board.map { machine -> machine.copy(bands = machine.bands.map { band -> band.copy(repositories = band.repositories.map { repo ->
                repo.copy(sessions = repo.sessions.filter { visible(it.session) && (matches(it.session) || it.prs.any(::matches)) })
            }.filter { it.sessions.isNotEmpty() }) }.filter { it.repositories.isNotEmpty() }) }.filter { it.bands.isNotEmpty() },
            watched.filter(::matches),
        )
    }
}

private data class BoardSection(val machine: String, val age: BoardAge, val repo: String)
private fun List<BoardMachine>.sections() = flatMap { machine -> machine.bands.flatMap { band ->
    band.repositories.map { BoardSection(machine.machine, band.age, it.repo) to it.sessions }
} }

/** Keep existing positions and append new rows to their section. Removed rows disappear immediately. */
private fun <K, T> retainSections(previous: List<Pair<K, List<T>>>, current: List<Pair<K, List<T>>>, id: (T) -> String,
    refresh: (T, T) -> T = { _, live -> live }): List<Pair<K, List<T>>> {
    val remaining = current.flatMap { it.second }.associateBy(id).toMutableMap()
    val sections = linkedMapOf<K, MutableList<T>>()
    previous.forEach { (key, rows) ->
        sections[key] = rows.mapNotNull { old -> remaining.remove(id(old))?.let { refresh(old, it) } }.toMutableList()
    }
    current.forEach { (key, rows) ->
        val added = rows.mapNotNull { remaining.remove(id(it)) }
        if (added.isNotEmpty()) sections.getOrPut(key) { mutableListOf() }.addAll(added)
    }
    return sections.filterValues { it.isNotEmpty() }.map { it.key to it.value.toList() }
}

private fun <T> retainRows(previous: List<T>, current: List<T>, id: (T) -> String): List<T> =
    retainSections(listOf(Unit to previous), listOf(Unit to current), id).flatMap { it.second }
