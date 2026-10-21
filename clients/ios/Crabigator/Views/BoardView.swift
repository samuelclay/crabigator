import SwiftUI

struct BoardView: View {
    @ObservedObject var model: SessionModel
    @Binding var listMode: Bool
    @Binding var expanded: Bool
    var wide: Bool
    var select: (Session, [Session]) -> Void
    @State private var searching = false
    @State private var query = ""
    @State private var settings = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var collapsed: Set<String> = []
    var spacing: BoardSpacing { BoardSpacing(density: model.preferences.density) }
    var spacingAnimation: Animation? { reduceMotion ? nil : .easeInOut(duration: 0.3) }
    var floatingToolbar: Bool { wide && expanded }
    var body: some View {
        VStack(spacing: 0) {
            Color.clear.frame(height: floatingToolbar && !searching ? 0 : 60).animation(reduceMotion ? nil : .easeInOut(duration: 0.36), value: floatingToolbar)
            if searching { TextField("Search", text: $query).textFieldStyle(.roundedBorder).padding(.horizontal, 12).padding(.bottom, 8).accessibilityIdentifier("Search sessions") }
            if model.sessions.isEmpty && model.prs.isEmpty {
                Spacer()
                switch model.rosterStatus {
                case .loading: ProgressView().tint(Crab.orange).accessibilityLabel("Loading sessions")
                case .failed: Button("Retry") { Task { await model.refresh() } }.buttonStyle(.bordered)
                case .ready: VStack(spacing: 16) { Text("No sessions").foregroundStyle(Crab.muted); PairingFields(model: model) }.padding(24)
                }
                Spacer()
            } else {
                GeometryReader { geometry in
                    ZStack {
                        boardContent(list: true, width: geometry.size.width).opacity(listMode ? 1 : 0).allowsHitTesting(listMode).accessibilityHidden(!listMode)
                        boardContent(list: false, width: geometry.size.width).opacity(listMode ? 0 : 1).allowsHitTesting(!listMode).accessibilityHidden(listMode)
                    }
                }.environment(\.boardTextSize, CGFloat(model.preferences.boardTextSize)).environment(\.boardSpacing, spacing)
            }
        }.background(Crab.ink)
            .overlay(alignment: .top) {
                header.frame(maxWidth: floatingToolbar ? 276 : .infinity)
                    .contentShape(Rectangle()).simultaneousGesture(TapGesture().onEnded {})
                    .background { if floatingToolbar { Capsule().fill(.ultraThinMaterial).overlay(Capsule().strokeBorder(Color.white.opacity(0.12))).shadow(color: .black.opacity(0.25), radius: 8, y: 3) } }
                    .animation(reduceMotion ? nil : .easeInOut(duration: 0.36), value: floatingToolbar)
            }
            .overlay(alignment: .bottom) { footer.padding(.bottom, 16) }
    }
    var header: some View {
        HStack(spacing: 4) {
            Image("Crabigator").resizable().scaledToFit().frame(width: 28, height: 28).padding(8).accessibilityLabel("Crabigator")
            Spacer()
            NewSessionButton(model: model)
            IconButton(icon: "magnifyingglass", label: "Search", selected: searching) { withAnimation { searching.toggle(); if !searching { query = "" } } }
            IconButton(icon: "gearshape", label: "Settings") { settings = true }
                .popover(isPresented: $settings, attachmentAnchor: .rect(.bounds), arrowEdge: .top) { SettingsMenu(model: model,
                    search: { settings = false; searching = true },
                    fullscreen: wide && model.selectionID != nil ? { settings = false; withAnimation(.easeInOut(duration: 0.36)) { expanded.toggle() } } : nil,
                    expanded: expanded).nativePopover() }
        }.padding(.horizontal, 8).padding(.vertical, floatingToolbar ? 2 : 6)
    }
    var footer: some View { BrowserModeControl(model: model, wide: wide) }
    func sections(_ list: Bool, includeAge: Bool = true) -> [RosterSection] {
        var preferences = model.preferences
        if !includeAge { preferences.boardAgeHours = 0 }
        return (list ? model.ordering.list : model.ordering.board).compactMap { section in
            var copy = section
            copy.sessions = section.sessions.filter { session in
                (preferences.includesSession(session, prs: model.ordering.prsBySession[session.id] ?? [], now: Int(Date().timeIntervalSince1970))) && (session.matches(query) || (!list && (model.ordering.prsBySession[session.id] ?? []).contains { $0.matches(query) }))
            }
            return copy.sessions.isEmpty ? nil : copy
        }
    }
    func boardContent(list: Bool, width: CGFloat) -> some View {
        let byPR = !list && model.preferences.boardGrouping == "prs"
        let now = Int(Date().timeIntervalSince1970)
        let filtered = sections(list, includeAge: !byPR)
        let sections = byPR
            ? groupBoardByPullRequest(filtered, prsBySession: model.ordering.prsBySession, now: now) { model.preferences.includesActivity($0, now: now) } : filtered
        var seen = Set<String>()
        let order = sections.flatMap { $0.pullRequests.flatMap(\.sessions) + $0.sessions }.filter { seen.insert($0.id).inserted }
        let fit = max(1, Int(width / 320)), maximum = max(1, Int(width / 280))
        let columns = model.preferences.columns == 0 ? fit : min(maximum, model.preferences.columns)
        return ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: spacing.gap(8), pinnedViews: [.sectionHeaders]) {
                    ForEach(model.ordering.machines, id: \.self) { machine in
                        let groups = sections.filter { $0.machine == machine }, rows = order.filter { $0.machine == machine }
                        if !rows.isEmpty {
                            Section {
                                if !collapsed.contains(machine) {
                                    ForEach(groups) { group in
                                        if let age = group.age, groups.first(where: { $0.age == age })?.id == group.id {
                                            Text(age.label).boardFont(12, weight: .medium).foregroundStyle(Crab.color(age.palette)).frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 10).padding(.vertical, spacing.gap(6)).background(Crab.color(age.palette).opacity(0.13), in: RoundedRectangle(cornerRadius: 4))
                                        }
                                        if !group.project.isEmpty { Text(group.project).boardFont(12).foregroundStyle(Crab.yellow).frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 8).padding(.top, spacing.gap(4)) }
                                        if list {
                                            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8, alignment: .top), count: columns), spacing: spacing.gap(8)) {
                                                ForEach(group.sessions) { session in sessionButton(session, list: true, full: false, order: order) }
                                            }
                                        } else {
                                            ForEach(group.pullRequests) { block in
                                                VStack(alignment: .leading, spacing: spacing.gap(4)) {
                                                    Group {
                                                        if let url = URL(string: block.pr.url), url.scheme == "https" {
                                                            Link(destination: url) { PRRow(pr: block.pr, wide: width >= 840) }.buttonStyle(.plain)
                                                        } else { PRRow(pr: block.pr, wide: width >= 840) }
                                                    }
                                                    .padding(.horizontal, 8)
                                                    // Match the session title's inset past its 38-point identity chip and 10-point gap.
                                                    .padding(.leading, width < 840 ? 48 : 0)
                                                    ForEach(block.sessions) { session in sessionButton(session, list: false, full: width >= 840, order: order, prs: []) }
                                                }
                                            }
                                            ForEach(group.sessions) { session in sessionButton(session, list: false, full: width >= 840, order: order) }
                                        }
                                    }
                                }
                            } header: {
                                Button {
                                    withAnimation(.easeInOut(duration: 0.3)) { if !collapsed.insert(machine).inserted { collapsed.remove(machine) }; proxy.scrollTo(machine, anchor: .top) }
                                } label: {
                                    HStack(spacing: 8) { Image(systemName: "desktopcomputer"); Text(machine).lineLimit(1).frame(maxWidth: floatingToolbar ? max(60, (width - 308) / 2 - 36) : .infinity, alignment: .leading); Spacer(); Text("\(rows.count) \(rows.count == 1 ? "session" : "sessions")").boardFont(11); Image(systemName: collapsed.contains(machine) ? "chevron.right" : "chevron.down").boardFont(11) }
                                        .boardFont(13, weight: .medium).foregroundStyle(Crab.muted).padding(.vertical, spacing.gap(13)).frame(minHeight: floatingToolbar ? 52 : 44).padding(.horizontal, 8).background(Crab.ink)
                                }.buttonStyle(.plain).id(machine).accessibilityIdentifier("Machine \(machine)")
                            }
                        }
                    }
                    if !list {
                        let watched = model.ordering.watched.filter { $0.matches(query) && model.preferences.includesActivity(max($0.updatedAt, $0.closedAt, $0.mentionedAt), now: Int(Date().timeIntervalSince1970)) }
                        if !watched.isEmpty {
                            Section {
                                if !collapsed.contains("\u{0}watched") { ForEach(watched) { pr in
                                    if let url = URL(string: pr.url), url.scheme == "https" { Link(destination: url) { PRRow(pr: pr, wide: width >= 840) }.buttonStyle(.plain) }
                                    else { PRRow(pr: pr, wide: width >= 840) }
                                } }
                            } header: {
                                Button {
                                    withAnimation { if !collapsed.insert("\u{0}watched").inserted { collapsed.remove("\u{0}watched") }; proxy.scrollTo("watched", anchor: .top) }
                                } label: {
                                    HStack { Text("Tracked PRs"); Spacer(); Text("\(watched.count) pull requests"); Image(systemName: collapsed.contains("\u{0}watched") ? "chevron.right" : "chevron.down") }.boardFont(12).foregroundStyle(Crab.muted).padding(10).background(Crab.ink)
                                }.buttonStyle(.plain).id("watched")
                            }
                        }
                    }
                    if sections.isEmpty && (list || !model.ordering.watched.contains { $0.matches(query) && model.preferences.includesActivity(max($0.updatedAt, $0.closedAt, $0.mentionedAt), now: Int(Date().timeIntervalSince1970)) }) { Text("No sessions match your filters").foregroundStyle(Crab.muted).padding(24) }
                // Lazy rows must discard cached wide/narrow geometry in both directions.
                }.id(!list && width >= 840).transition(.identity)
                    .padding(.horizontal, 12).padding(.bottom, 88)
            }.refreshable { await model.refresh() }
        }.animation(spacingAnimation, value: model.preferences.density)
    }
    func sessionButton(_ session: Session, list: Bool, full: Bool, order: [Session], prs: [PullRequest]? = nil) -> some View {
        let selected = !(wide && expanded) && model.selectionID == session.id
        let shape = RoundedRectangle(cornerRadius: list ? 12 : 6)
        return Button { select(session, order) } label: {
            SessionRow(session: session, prs: prs ?? model.ordering.prsBySession[session.id] ?? [], preferences: model.preferences, list: list, wide: full)
                // Rebuild the column layout when switching between full width
                // and the sidebar, without retaining an animated wide layout.
                .animation(spacingAnimation, value: model.preferences.density)
                .id(full)
                .transaction { $0.animation = nil }
                .padding(.horizontal, list ? 12 : 8).padding(.vertical, list ? spacing.gap(12) : spacing.gap(7))
                .frame(maxWidth: .infinity, alignment: .leading)
                .background {
                    ZStack {
                        shape.fill(list ? Crab.panel : .clear).opacity(selected ? 0 : 1)
                        shape.fill(Crab.orange.opacity(0.13)).opacity(selected ? 1 : 0)
                    }
                }
                .overlay { shape.strokeBorder(Crab.orange, lineWidth: 2).opacity(selected ? 1 : 0) }
                .animation(reduceMotion ? nil : .easeOut(duration: selected ? 0.12 : 0.35), value: selected)
        }.buttonStyle(.plain).accessibilityIdentifier("Session \(session.id)").accessibilityValue(selected ? "Selected" : "")
            .accessibilityAddTraits(selected ? .isSelected : [])
    }
}
struct SessionRow: View {
    var session: Session, prs: [PullRequest], preferences: Preferences
    var list: Bool, wide: Bool
    @Environment(\.boardSpacing) private var spacing
    var body: some View {
        HStack(alignment: .sessionTitleCenter, spacing: 10) {
            IdentityChip(session: session)
            VStack(alignment: .leading, spacing: spacing.gap(6)) {
                HStack(alignment: .sessionTitleCenter, spacing: 12) {
                    Text(session.title).sessionTitleAlignment().boardFont(14, weight: .medium).foregroundStyle(session.active ? Crab.title : Crab.muted).lineLimit(wide || spacing.tight ? 1 : 2).frame(maxWidth: .infinity, alignment: .leading)
                    if wide {
                        Text(session.branch).boardFont(12).foregroundStyle(Crab.muted).lineLimit(1).frame(width: 180, alignment: .leading)
                        HStack(alignment: .sessionTitleCenter, spacing: 8) { SessionStatus(session: session); recency; Spacer(minLength: 0) }.frame(width: 360, alignment: .leading)
                    }
                }
                if preferences.boardRecaps && !session.recap.isEmpty {
                    Text(session.recap).boardFont(12).foregroundStyle(Crab.muted).fixedSize(horizontal: false, vertical: true)
                }
                if list {
                    SessionTiming(session: session, preferences: preferences)
                    if preferences.visible("tools") || preferences.visible("compactions") {
                        HStack { if preferences.visible("tools") { Text("⚒ \(session.tools.map(String.init) ?? "—")") }; if preferences.visible("compactions") { Text("⊜ \(session.compactions.map(String.init) ?? "—")") } }.boardFont(11).foregroundStyle(Crab.orange)
                    }
                } else {
                    if !wide { HStack(alignment: .sessionTitleCenter) { Text(session.branch).boardFont(11).foregroundStyle(Crab.muted).lineLimit(1); Spacer(); HStack(spacing: 8) { SessionStatus(session: session); recency } } }
                    ForEach(prs.filter { $0.visibleOnBoard(for: session.id) }) { pr in PRRow(pr: pr, wide: wide) }
                }
            }
        }.fixedSize(horizontal: false, vertical: true).contentShape(Rectangle())
    }
    var recency: some View {
        TimelineView(.periodic(from: .now, by: 1)) { t in
            RecencyColumns(session: session, now: Int(t.date.timeIntervalSince1970))
        }
    }
}
struct PRRow: View {
    var pr: PullRequest, wide: Bool
    @Environment(\.boardSpacing) private var spacing
    var body: some View {
        VStack(alignment: .leading, spacing: spacing.gap(4)) {
            HStack(alignment: .top, spacing: 12) {
                Text(verbatim: "#\(pr.number)  \(pr.title)").boardFont(13).foregroundStyle(pr.primary || pr.watched ? Crab.purple : Crab.muted).lineLimit(wide || spacing.tight ? 1 : 2).frame(maxWidth: .infinity, alignment: .leading)
                if wide {
                    Text(pr.branch).boardFont(12).foregroundStyle(Crab.muted).lineLimit(1).frame(width: 180, alignment: .leading)
                    status.frame(width: 360)
                }
            }
            if !wide { status }
        }.fixedSize(horizontal: false, vertical: true)
    }
    var status: some View {
        ViewThatFits(in: .horizontal) {
            statusLine(compact: false).frame(minWidth: 320)
            statusLine(compact: true)
        }.boardFont(11).lineLimit(1).minimumScaleFactor(0.4)
    }
    func statusLine(compact: Bool) -> some View {
        HStack(spacing: compact ? 3 : 6) {
            HStack(spacing: compact ? 3 : 6) {
                badge(compact: compact)
                HStack(spacing: compact ? 3 : 8) {
                    if let n = pr.additions { Text("+\(n)").foregroundStyle(Crab.mint) }
                    if let n = pr.deletions { Text("−\(n)").foregroundStyle(Crab.red) }
                }
                Spacer(minLength: 0)
            }.frame(maxWidth: .infinity, alignment: .leading)
            statusColumns(compact: compact)
        }
    }
    func statusColumns(compact: Bool) -> some View {
        HStack(spacing: compact ? 2 : 6) {
            Text(pr.checks.isEmpty ? "—" : pr.checks)
                .foregroundStyle(pr.checks.isEmpty ? Crab.muted : pr.checksFailed > 0 ? Crab.red : pr.checksPending > 0 ? Crab.yellow : Crab.mint)
                .frame(width: compact ? 42 : 48, alignment: .leading)
                .accessibilityLabel(pr.checks.isEmpty ? "CI status unavailable" : pr.checksFailed > 0 ? "\(pr.checksFailed) checks failed" : pr.checksPending > 0 ? "\(pr.checksPending) checks pending" : "Checks passed")
            HStack(spacing: 2) {
                if pr.unresolvedComments > 0 {
                    Image(systemName: "bubble.left")
                    Text("\(pr.unresolvedComments)")
                }
            }.foregroundStyle(Crab.orange)
                .frame(width: compact ? 24 : 36, alignment: .leading).accessibilityElement(children: .ignore)
                .accessibilityLabel("Unresolved comments: \(pr.unresolvedComments)")
            Text(approval.symbol).foregroundStyle(approval.color).frame(width: compact ? 12 : 20)
                .accessibilityLabel("Approval: " + approval.label)
            Text(mergeStatus).foregroundStyle(mergeStatus == "Conflicts" ? Crab.red : mergeStatus == "Behind" ? Crab.yellow : mergeStatus == "Clean" ? Crab.mint : Crab.muted)
                .frame(width: compact ? 46 : 54, alignment: .leading)
                .accessibilityLabel("Branch status: " + mergeStatus)
        }
    }
    var approval: (symbol: String, label: String, color: Color) {
        guard pr.state == "OPEN" else { return ("—", "Not applicable", Crab.muted) }
        if pr.review == "APPROVED" { return ("✓", "Approved", Crab.mint) }
        if pr.review == "CHANGES_REQUESTED" { return ("✗", "Changes requested", Crab.red) }
        if pr.reviewDismissed { return ("⊘", "Review dismissed", Crab.orange) }
        return ("◌", "Awaiting review", Crab.muted)
    }
    var mergeStatus: String {
        guard pr.state == "OPEN" else { return "—" }
        if pr.mergeable == "CONFLICTING" { return "Conflicts" }
        if pr.mergeState == "BEHIND" { return "Behind" }
        return pr.mergeable == "MERGEABLE" ? "Clean" : "—"
    }
    func badge(compact: Bool) -> some View {
            let color = pr.state == "MERGED" ? Crab.purple : pr.state == "CLOSED" ? Crab.red : pr.draft ? Crab.muted : Crab.mint
            return Text(pr.draft && pr.state == "OPEN" ? "Draft" : pr.state.isEmpty ? "Pending" : pr.state.capitalized).padding(.horizontal, compact ? 3 : 5).padding(.vertical, 2).foregroundStyle(color).background(color.opacity(0.12), in: RoundedRectangle(cornerRadius: 4)).boardFont(11).lineLimit(1)
    }
}
