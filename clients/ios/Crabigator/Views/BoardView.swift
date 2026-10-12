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
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                Image("Crabigator").resizable().scaledToFit().frame(width: 28, height: 28).padding(8).accessibilityLabel("Crabigator")
                Spacer()
                IconButton(icon: "magnifyingglass", label: "Search", selected: searching) { withAnimation { searching.toggle(); if !searching { query = "" } } }
                IconButton(icon: "gearshape", label: "Settings") { settings = true }
                    .popover(isPresented: $settings, attachmentAnchor: .rect(.bounds), arrowEdge: .top) { SettingsMenu(model: model).nativePopover() }
            }.padding(.horizontal, 8).padding(.vertical, 6)
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
                }.environment(\.boardTextSize, CGFloat(model.preferences.boardTextSize))
            }
        }.background(Crab.ink).overlay(alignment: .bottom) { footer.padding(.bottom, 16) }
    }
    var footer: some View {
        HStack(spacing: 4) {
            segment("List", icon: "list.bullet", isList: true)
            segment("PR board", icon: "rectangle.split.3x1", isList: false)
        }.padding(5).background(.ultraThinMaterial, in: Capsule()).overlay(Capsule().strokeBorder(Color.white.opacity(0.12))).shadow(color: .black.opacity(0.3), radius: 10, y: 4)
    }
    func segment(_ label: String, icon: String, isList: Bool) -> some View {
        Button {
            withAnimation(.easeInOut(duration: 0.36)) {
                if listMode == isList && wide { expanded.toggle() } else { listMode = isList }
            }
        } label: {
            HStack(spacing: 7) {
                Image(systemName: icon); Text(label).boardFont(13, weight: .medium)
                if wide && listMode == isList { Image(systemName: expanded ? "arrow.down.right.and.arrow.up.left" : "arrow.up.left.and.arrow.down.right").boardFont(10) }
            }.padding(.horizontal, 14).frame(height: 40).background(listMode == isList ? Crab.orange.opacity(0.18) : .clear, in: Capsule()).foregroundStyle(listMode == isList ? Crab.orange : Crab.muted)
        }.buttonStyle(.plain).accessibilityIdentifier(label).accessibilityValue(listMode == isList ? (wide ? (expanded ? "Selected, full width" : "Selected, sidebar") : "Selected") : "")
    }
    func sections(_ list: Bool) -> [RosterSection] {
        (list ? model.ordering.list : model.ordering.board).compactMap { section in
            var copy = section
            copy.sessions = section.sessions.filter { session in
                (model.preferences.showEnded || session.active) && (session.matches(query) || (!list && (model.ordering.prsBySession[session.id] ?? []).contains { $0.matches(query) }))
            }
            return copy.sessions.isEmpty ? nil : copy
        }
    }
    func boardContent(list: Bool, width: CGFloat) -> some View {
        let sections = sections(list), order = sections.flatMap(\.sessions)
        let fit = max(1, Int(width / 320)), maximum = max(1, Int(width / 280))
        let columns = model.preferences.columns == 0 ? fit : min(maximum, model.preferences.columns)
        return ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 8, pinnedViews: [.sectionHeaders]) {
                    ForEach(model.ordering.machines, id: \.self) { machine in
                        let groups = sections.filter { $0.machine == machine }, rows = groups.flatMap(\.sessions)
                        if !rows.isEmpty {
                            Section {
                                if !collapsed.contains(machine) {
                                    ForEach(groups) { group in
                                        if let age = group.age, groups.first(where: { $0.age == age })?.id == group.id {
                                            Text(age.label).boardFont(12, weight: .medium).foregroundStyle(Crab.color(age.palette)).frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 10).padding(.vertical, 6).background(Crab.color(age.palette).opacity(0.13), in: RoundedRectangle(cornerRadius: 4))
                                        }
                                        if !group.project.isEmpty { Text(group.project).boardFont(12).foregroundStyle(Crab.yellow).frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 8).padding(.top, 4) }
                                        if list {
                                            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8, alignment: .top), count: columns), spacing: 8) {
                                                ForEach(group.sessions) { session in sessionButton(session, list: true, full: false, order: order) }
                                            }
                                        } else { ForEach(group.sessions) { session in sessionButton(session, list: false, full: width >= 840, order: order); Divider().overlay(Color.white.opacity(0.06)) } }
                                    }
                                }
                            } header: {
                                Button {
                                    withAnimation(.easeInOut(duration: 0.3)) { if !collapsed.insert(machine).inserted { collapsed.remove(machine) }; proxy.scrollTo(machine, anchor: .top) }
                                } label: {
                                    HStack(spacing: 8) { Image(systemName: "desktopcomputer"); Text(machine).lineLimit(1); Spacer(); Text("\(rows.count) \(rows.count == 1 ? "session" : "sessions")").boardFont(11); Image(systemName: collapsed.contains(machine) ? "chevron.right" : "chevron.down").boardFont(11) }
                                        .boardFont(13, weight: .medium).foregroundStyle(Crab.muted).padding(.vertical, 13).padding(.horizontal, 8).background(Crab.ink)
                                }.buttonStyle(.plain).id(machine).accessibilityIdentifier("Machine \(machine)")
                            }
                        }
                    }
                    if !list {
                        let watched = model.ordering.watched.filter { $0.matches(query) }
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
                    if sections.isEmpty && (list || !model.ordering.watched.contains { $0.matches(query) }) { Text(query.isEmpty ? "No active sessions" : "No matching sessions").foregroundStyle(Crab.muted).padding(24) }
                }.padding(.horizontal, 12).padding(.bottom, 88)
            }.refreshable { await model.refresh() }
        }
    }
    func sessionButton(_ session: Session, list: Bool, full: Bool, order: [Session]) -> some View {
        let selected = model.selectionID == session.id
        let shape = RoundedRectangle(cornerRadius: list ? 12 : 6)
        return Button { select(session, order) } label: {
            SessionRow(session: session, prs: model.ordering.prsBySession[session.id] ?? [], preferences: model.preferences, list: list, wide: full)
                .padding(.horizontal, list ? 12 : 8).padding(.vertical, list ? (model.preferences.density == "compact" ? 8 : 12) : 7)
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
    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            IdentityChip(session: session)
            if list || !wide { SessionStatus(session: session) }
            VStack(alignment: .leading, spacing: 6) {
                HStack(alignment: .top, spacing: 12) {
                    Text(session.title).boardFont(14, weight: .medium).foregroundStyle(session.active ? Crab.title : Crab.muted).lineLimit(wide ? 1 : 2).frame(maxWidth: .infinity, alignment: .leading)
                    if wide {
                        Text(session.branch).boardFont(12).foregroundStyle(Crab.muted).lineLimit(1).frame(width: 180, alignment: .leading)
                        HStack(alignment: .top) { SessionStatus(session: session); Spacer(); recency }.frame(width: 300, alignment: .leading)
                    }
                }
                if list {
                    SessionTiming(session: session, preferences: preferences)
                    if preferences.visible("tools") || preferences.visible("compactions") {
                        HStack { if preferences.visible("tools") { Text("⚒ \(session.tools.map(String.init) ?? "—")") }; if preferences.visible("compactions") { Text("⊜ \(session.compactions.map(String.init) ?? "—")") } }.boardFont(11).foregroundStyle(Crab.orange)
                    }
                } else {
                    if !wide { HStack(alignment: .top) { Text(session.branch).boardFont(11).foregroundStyle(Crab.muted).lineLimit(1); Spacer(); recency } }
                    ForEach(prs) { pr in PRRow(pr: pr, wide: wide) }
                }
            }
        }.fixedSize(horizontal: false, vertical: true).contentShape(Rectangle())
    }
    var recency: some View {
        TimelineView(.periodic(from: .now, by: 1)) { t in
            HStack(spacing: 10) { Text("⟩ " + ageText(session.promptAt, now: Int(t.date.timeIntervalSince1970)).replacingOccurrences(of: " ago", with: "")).foregroundStyle(Crab.orange); Text("⋖ " + ageText(session.completionAt, now: Int(t.date.timeIntervalSince1970))).foregroundStyle(Crab.deepOrange) }.boardFont(11).monospacedDigit().lineLimit(1).fixedSize()
        }
    }
}
struct PRRow: View {
    var pr: PullRequest, wide: Bool
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .top, spacing: 12) {
                Text(verbatim: "#\(pr.number)  \(pr.title)").boardFont(13).foregroundStyle(pr.primary || pr.watched ? Crab.purple : Crab.muted).lineLimit(wide ? 1 : 2).frame(maxWidth: .infinity, alignment: .leading)
                if wide {
                    Text(pr.branch).boardFont(12).foregroundStyle(Crab.muted).lineLimit(1).frame(width: 180, alignment: .leading)
                    status.frame(width: 300)
                }
            }
            if !wide { status }
        }.fixedSize(horizontal: false, vertical: true)
    }
    var status: some View {
        HStack(spacing: 6) {
            badge; changes
            Spacer(minLength: 4)
            HStack(spacing: 6) {
                if !pr.checks.isEmpty {
                    Text(pr.checks).foregroundStyle(pr.checksFailed > 0 ? Crab.red : pr.checksPending > 0 ? Crab.yellow : Crab.mint)
                        .accessibilityLabel(pr.checksFailed > 0 ? "\(pr.checksFailed) checks failed" : pr.checksPending > 0 ? "\(pr.checksPending) checks pending" : "Checks passed")
                }
                if pr.state == "OPEN" {
                    if pr.mergeable == "CONFLICTING" { Text("Conflicts").foregroundStyle(Crab.red) }
                    else if pr.mergeable == "MERGEABLE" {
                        Text(pr.mergeState == "BEHIND" ? "Behind" : "Clean").foregroundStyle(pr.mergeState == "BEHIND" ? Crab.yellow : Crab.mint)
                    }
                }
            }
            .boardFont(11)
        }.lineLimit(1).minimumScaleFactor(0.8)
    }
    var changes: some View {
        HStack(spacing: 8) {
            if let n = pr.additions { Text("+\(n)").foregroundStyle(Crab.mint) }; if let n = pr.deletions { Text("−\(n)").foregroundStyle(Crab.red) }
        }.boardFont(11).lineLimit(1)
    }
    var badge: some View {
            let color = pr.state == "MERGED" ? Crab.purple : pr.state == "CLOSED" ? Crab.red : pr.draft ? Crab.muted : Crab.mint
            return Text(pr.draft && pr.state == "OPEN" ? "Draft" : pr.state.isEmpty ? "Pending" : pr.state.capitalized).padding(.horizontal, 5).padding(.vertical, 2).foregroundStyle(color).background(color.opacity(0.12), in: RoundedRectangle(cornerRadius: 4)).boardFont(11).lineLimit(1)
    }
}
