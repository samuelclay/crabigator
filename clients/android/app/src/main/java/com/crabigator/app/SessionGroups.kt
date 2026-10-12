package com.crabigator.app

internal val Session.machineGroup: String get() = machine.ifBlank { "Unknown machine" }
internal val Session.activityAt: Long get() = maxOf(stats.promptAt, stats.completionAt, stats.startedAt)

/** Rank machines once; later arrivals go after machines the user has already seen. */
internal fun retainMachineOrder(previous: List<String>, sessions: List<Session>): List<String> {
    val ranked = sessions.groupBy { it.machineGroup }.entries
        .sortedWith(compareByDescending<Map.Entry<String, List<Session>>> { entry -> entry.value.maxOf { it.activityAt } }.thenBy { it.key })
        .map { it.key }
    return previous + ranked.filterNot { it in previous }
}

internal data class SessionListGroup(val machine: String, val project: String, val sessions: List<Session>)

internal fun sessionListGroups(sessions: List<Session>, machines: List<String>, grouping: String, order: String): List<SessionListGroup> {
    val byMachine = sessions.groupBy { it.machineGroup }
    return machines.flatMap { machine ->
        val rows = byMachine[machine] ?: return@flatMap emptyList()
        if (grouping != "project") listOf(SessionListGroup(machine, "", rows))
        else rows.groupBy { it.repo.ifBlank { "Sessions" } }.entries.let { projects ->
            if (order == "alpha") projects.sortedBy { it.key.lowercase() }
            else projects.sortedByDescending { entry -> entry.value.maxOf { it.activityAt } }
        }.map { SessionListGroup(machine, it.key, it.value) }
    }
}

internal data class MachinePrGroup(val machine: String, val sessions: List<Session>, val prs: List<PullRequest>)

/** A shared PR belongs in each contributing machine's section, with only its local sessions. */
internal fun machinePrGroups(sessions: List<Session>, prs: List<PullRequest>, machines: List<String>): List<MachinePrGroup> =
    machines.mapNotNull { machine ->
        val localPrs = prs.mapNotNull { pr ->
            val local = pr.sessions.filter { it.machineGroup == machine }
            if (local.isEmpty()) null else pr.copy(sessions = local)
        }
        val localSessions = (sessions.filter { it.machineGroup == machine } + localPrs.flatMap { it.sessions }).distinctBy { it.id }
            .sortedWith(compareByDescending<Session> { it.attention }.thenByDescending { it.activityAt }.thenBy { it.id })
        if (localSessions.isEmpty()) null else MachinePrGroup(machine, localSessions, localPrs)
    }
