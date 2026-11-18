import SwiftUI

extension SessionModel {
    /// Every computer the board can filter to, including a saved choice that is offline now.
    var filterComputers: [String] { Array(Set(ordering.machines + [preferences.boardComputer])).filter { !$0.isEmpty }.sorted() }
}

/// The row under the board's header: three filter chips and the View button. Narrow, the
/// chips scroll sideways and View stays at the trailing edge.
struct BoardFilterBar: View {
    @ObservedObject var model: SessionModel
    var list: Bool, wide: Bool
    @Binding var open: Bool
    /// Set when the bar shares its row with the floating toolbar: the chips keep to its left.
    var chipsWidth: CGFloat?
    @State private var content: CGFloat = 0
    @State private var room: CGFloat = 0
    var body: some View {
        let hours = model.preferences.boardAgeHours, computer = model.preferences.boardComputer
        let age = Preferences.ageLabel(hours), sessions = model.preferences.showEnded ? "Live + ended" : "Live", computerName = computer.isEmpty ? "All computers" : computer
        HStack(spacing: 6) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    Menu {
                        Picker("Activity", selection: $model.preferences.boardAgeHours) { ForEach(Preferences.ageChoices, id: \.self) { Text(Preferences.ageMenuLabel($0)).tag($0) } }
                    } label: { FilterChip(text: age, dot: Crab.color(Preferences.agePalette(hours))) }
                        .accessibilityLabel(age).accessibilityIdentifier("Time filter")
                    Menu {
                        Picker("Sessions", selection: $model.preferences.showEnded) { Text("Live").tag(false); Text("Live and ended").tag(true) }
                    } label: { FilterChip(text: sessions) }
                        .accessibilityLabel(sessions).accessibilityIdentifier("Sessions filter")
                    Menu {
                        Picker("Computers", selection: $model.preferences.boardComputer) { Text("All computers").tag(""); ForEach(model.filterComputers, id: \.self) { Text($0).tag($0) } }
                    } label: { FilterChip(text: computerName, color: computer.isEmpty ? Crab.text : Crab.computer(computer)) }
                        .accessibilityLabel(computerName).accessibilityIdentifier("Computer filter")
                }.onGeometryChange(for: CGFloat.self) { $0.size.width } action: { content = $0 }
            }.scrollBounceBehavior(.basedOnSize, axes: .horizontal).frame(maxWidth: chipsWidth ?? .infinity, alignment: .leading)
                .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { room = $0 }
                // Chips that run past the edge fade out, to show there is more to scroll to.
                .mask { HStack(spacing: 0) { Color.black; LinearGradient(colors: [.black, content > room + 1 ? .clear : .black], startPoint: .leading, endPoint: .trailing).frame(width: 20) } }
            if chipsWidth != nil { Spacer(minLength: 0) }
            ViewButton(open: open, identifier: "Board view") { open = true }
                .popover(isPresented: $open, attachmentAnchor: .rect(.bounds), arrowEdge: .top) { BoardViewPanel(model: model, list: list, wide: wide).panelPresentation() }
        }.padding(.horizontal, 12).padding(.bottom, chipsWidth == nil ? 12 : 0)
    }
}

/// View for the List or the PR board: which sessions, how they are grouped, what a row
/// shows, and its size. Every choice is drawn, and a sample row follows each change.
struct BoardViewPanel: View {
    @ObservedObject var model: SessionModel
    var list: Bool, wide: Bool
    @Environment(\.dismiss) private var dismiss
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var sample = SampleRow()
    private var set: Preferences.Shown { list ? .listRow : .boardRow }
    private var change: Animation? { reduceMotion ? nil : .easeInOut(duration: 0.2) }
    static let labels = ["recap": "Recap", "sessionTime": "Session time", "thinkingTime": "Thinking time", "promptRecency": "Last prompt", "completionRecency": "Last completion", "prompts": "Prompts", "completions": "Completions", "tools": "Tools", "compactions": "Compactions", "context": "Context", "git": "Git status", "changes": "Changes"]
    var body: some View {
        VStack(spacing: 0) {
        PanelHeader(scope: list ? "List" : "PR board") { dismiss() }
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                PanelSection(title: "Which sessions") {
                    TimeRuler(hours: $model.preferences.boardAgeHours)
                    Toggle("Ended sessions", isOn: $model.preferences.showEnded).font(.system(size: 15)).tint(Crab.cyan).frame(minHeight: panelTouch)
                    Segments(name: "Computer", options: [("", "All computers")] + model.filterComputers.map { ($0, $0) }, selection: $model.preferences.boardComputer, size: 13, color: { $0.isEmpty ? nil : Crab.computer($0) }, scrolls: true)
                    PanelNote("The same three filters as the chips above. Change them in either place.")
                }
                PanelSection(title: "Group by") {
                    if list {
                        HStack(alignment: .top, spacing: 6) { ForEach(GroupBy.allCases, id: \.self) { group($0) } }.fixedSize(horizontal: false, vertical: true)
                        if model.preferences.listGroupBy.byProject {
                            HStack(spacing: 12) {
                                Text("Sort projects").font(.system(size: 13)).foregroundStyle(Crab.soft).fixedSize()
                                Segments(name: "Sort projects", options: [("recent", "Most recent first"), ("alpha", "A to Z")], selection: $model.preferences.order, size: 13)
                            }
                        }
                    } else {
                        // Two tiles the size of the List's four; the note takes the other half.
                        HStack(alignment: .top, spacing: 6) {
                            HStack(alignment: .top, spacing: 6) { ForEach([GroupBy.one, .computer], id: \.self) { group($0) } }.frame(maxWidth: .infinity)
                            PanelNote("The PR board always groups by time band and repository.").padding(.leading, 6).frame(maxWidth: .infinity, alignment: .leading)
                        }.fixedSize(horizontal: false, vertical: true)
                    }
                }
                if !list {
                    PanelSection(title: "Each block starts with") {
                        HStack(alignment: .top, spacing: 10) {
                            PickerTile(label: "The session", selected: model.preferences.boardGrouping == "sessions") { model.preferences.boardGrouping = "sessions" } art: { Thumbnail(thumb: .leadSession, radius: 6) }
                            PickerTile(label: "The pull request", selected: model.preferences.boardGrouping == "prs") { model.preferences.boardGrouping = "prs" } art: { Thumbnail(thumb: .leadPullRequest, radius: 6) }
                        }.fixedSize(horizontal: false, vertical: true)
                    }
                }
                PanelSection(title: list ? "A List row shows" : "A PR board row shows") {
                    sampleRow
                    FlowLayout(spacing: 8) {
                        ForEach(set.items, id: \.self) { key in
                            ToggleChip(label: Self.labels[key] ?? key, on: model.preferences.shows(key, in: set)) { withAnimation(change) { model.preferences.toggle(key, in: set) } }
                        }
                    }
                    PanelNote(list ? "The PR board keeps its own set." : "The List keeps its own set.")
                }
                PanelSection(title: "Size and spacing") {
                    HStack(spacing: 8) {
                        TextSizeStepper(name: "Board text size", value: $model.preferences.boardTextSize, range: 10...24)
                        Segments(name: "Row spacing", options: [("compact", "Tight"), ("comfortable", "Normal"), ("relaxed", "Relaxed")], selection: $model.preferences.density)
                    }
                    if list {
                        Text("Columns").font(.system(size: 13)).foregroundStyle(Crab.soft)
                        HStack(spacing: 6) {
                            ForEach(0...4, id: \.self) { count in
                                PickerTile(label: count == 0 ? "Auto" : "\(count)", selected: model.preferences.columns == count, compact: true, identifier: "Columns " + (count == 0 ? "Auto" : "\(count)")) { model.preferences.columns = count } art: { ColumnsArt(count: count).frame(height: 34).padding(.horizontal, 1).padding(.top, 3) }
                            }
                        }
                    }
                    if wide {
                        Text("Session list side").font(.system(size: 13)).foregroundStyle(Crab.soft)
                        HStack(alignment: .top, spacing: 10) {
                            PickerTile(label: "Left", selected: !model.preferences.sidebarRight) { model.preferences.sidebarRight = false } art: { Thumbnail(thumb: .side(right: false), radius: 6) }
                            PickerTile(label: "Right", selected: model.preferences.sidebarRight) { model.preferences.sidebarRight = true } art: { Thumbnail(thumb: .side(right: true), radius: 6) }
                        }
                    }
                }
            }.padding(.horizontal, 16).padding(.top, 4).padding(.bottom, 24)
        }.accessibilityIdentifier("View panel")
        }.background(Crab.panel).foregroundStyle(Crab.text).tint(Crab.cyan)
    }
    private func group(_ choice: GroupBy) -> some View {
        PickerTile(label: choice.label, selected: (list ? model.preferences.listGroupBy : model.preferences.boardGroupBy) == choice, compact: true) {
            if list { model.preferences.listGroupBy = choice } else { model.preferences.boardGroupBy = choice }
        } art: { Thumbnail(thumb: choice.thumb) }
    }
    /// The real row view, on a sample session, with the size, spacing and items chosen here.
    private var sampleRow: some View {
        let spacing = BoardSpacing(density: model.preferences.density), card = RoundedRectangle(cornerRadius: 12)
        return SessionRow(session: sample.session(on: model.ordering.machines.first), prs: list ? [] : [sample.pullRequest], preferences: model.preferences, list: list, wide: false)
            .padding(.horizontal, list ? 12 : 8).padding(.vertical, spacing.gap(list ? 12 : 7)).frame(maxWidth: .infinity, alignment: .leading)
            .background(list ? Crab.panel : .clear, in: card).padding(list ? 10 : 6).background(Crab.ink, in: card)
            .environment(\.boardTextSize, CGFloat(model.preferences.boardTextSize)).environment(\.boardSpacing, spacing)
            .accessibilityElement(children: .combine).accessibilityIdentifier("Sample row")
    }
}

/// A session and its pull request that exist only in the View panel's sample row.
struct SampleRow {
    private let now = Int(Date().timeIntervalSince1970)
    func session(on machine: String?) -> Session {
        Session(["session_id": "sample-row", "title": "Live updates for mobile", "device_name": machine ?? "Demo computer", "state": "complete", "active": true, "branch": "mobile-realtime",
                 "session_mark": ["glyph": "◆", "fg_hex": "#AF87FF", "bg_hex": "#2A2440"], "recap": ["headline": "Stories now arrive live, with a pill for the new ones.", "generated_at": (now - 1440) * 1000],
                 "started_at": now - 8040, "prompts_changed_at": now - 1500, "completions_changed_at": now - 1440, "uncommitted": 3, "additions": 212, "deletions": 48,
                 "stats": ["work_seconds": 8040, "thinking_seconds": 2280, "tools": 212, "compressions": 3, "context_used": 186_000, "context_limit": 1_000_000]])
    }
    var pullRequest: PullRequest {
        PullRequest(["owner": "example", "repo": "mobile", "number": 2217, "pr": ["title": "Live updates and a new stories pill", "state": "OPEN", "primary": true, "branch": "mobile-realtime", "additions": 9036, "deletions": 117, "checks_total": 4, "checks_pending": 1, "mergeable": "MERGEABLE"]])
    }
}

/// A list column drawn as two rows of cells. Auto shows three dashed columns.
struct ColumnsArt: View {
    var count: Int
    var body: some View {
        Canvas { context, size in
            let columns = count == 0 ? 3 : count, gap: CGFloat = 2
            let width = (size.width - gap * CGFloat(columns - 1)) / CGFloat(columns), height = (size.height - gap) / 2
            for column in 0..<columns { for line in 0..<2 {
                let cell = Path(roundedRect: CGRect(x: CGFloat(column) * (width + gap), y: CGFloat(line) * (height + gap), width: width, height: height).insetBy(dx: 0.5, dy: 0.5), cornerRadius: 2)
                if count > 0 { context.fill(cell, with: .color(Crab.hex("4A5563"))) }
                context.stroke(cell, with: .color(Crab.box), style: StrokeStyle(lineWidth: 1, dash: count == 0 ? [3, 2] : []))
            } }
        }.accessibilityHidden(true)
    }
}

/// View for the session screen: terminal text, long lines, the terminal's height, the
/// details drawer and the stats strip.
struct SessionViewPanel: View {
    @ObservedObject var model: SessionModel
    var sessionID: String
    @Environment(\.dismiss) private var dismiss
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var change: Animation? { reduceMotion ? nil : .easeInOut(duration: 0.2) }
    static let widgets: [(key: String, label: String, tone: Color)] = [("recap", "Recap", Crab.hex("5f87ff")), ("prs", "Pull requests", Crab.hex("af87ff")), ("commits", "Commits", Crab.hex("ff8700")), ("git", "Git status", Crab.hex("5fff5f")), ("changes", "Changes", Crab.hex("ff87d7")), ("flow", "Flow scene", Crab.hex("d7af5f"))]
    static let stats: [(key: String, text: String, color: Color)] = [("sessionTime", "◉ 2h 14m", Crab.blue), ("thinkingTime", "◐ 38m", Crab.mint), ("prompts", "⟩ 14", Crab.orange), ("completions", "⋖ 13", Crab.deepOrange), ("tools", "⚒ 212", Crab.orange), ("compactions", "⊜ 3", Crab.purple), ("context", "◔ 186k / 1M · 19%", Crab.title)]
    var body: some View {
        let size = CGFloat(model.preferences.fontSize)
        NavigationStack {
            VStack(spacing: 0) {
            PanelHeader(scope: "Session") { dismiss() }
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    PanelSection(title: "Terminal text") {
                        (Text("● ").foregroundColor(Crab.mint) + Text("Read SettingsMenu.swift\nMapped every setting on the screen."))
                            .font(.system(size: size, design: .monospaced)).lineSpacing(max(0, size * (CGFloat(model.preferences.lineSpacing) / 100 - 1.2))).foregroundStyle(Crab.soft)
                            .lineLimit(model.preferences.wrap ? 4 : 2).fixedSize(horizontal: !model.preferences.wrap, vertical: true)
                            .padding(.horizontal, 12).padding(.vertical, 10).frame(maxWidth: .infinity, alignment: .leading).background(Crab.well).clipShape(RoundedRectangle(cornerRadius: 10))
                            .accessibilityElement(children: .ignore).accessibilityLabel("Sample terminal text").accessibilityIdentifier("Sample terminal text")
                        HStack(spacing: 8) {
                            TextSizeStepper(name: "Terminal text size", value: $model.preferences.fontSize, range: 9...24)
                            Segments(name: "Line spacing", options: [(120, "Tight"), (145, "Normal"), (170, "Relaxed")], selection: $model.preferences.lineSpacing)
                        }
                    }
                    PanelSection(title: "Long lines") {
                        HStack(alignment: .top, spacing: 10) {
                            PickerTile(label: "Wrap", caption: "Continue on the next line", selected: model.preferences.wrap) { model.preferences.wrap = true } art: { Thumbnail(thumb: .wrap, radius: 6) }
                            PickerTile(label: "Scroll sideways", caption: "Keep each line whole", selected: !model.preferences.wrap) { model.preferences.wrap = false } art: { Thumbnail(thumb: .scroll, radius: 6) }
                        }.fixedSize(horizontal: false, vertical: true)
                    }
                    PanelSection(title: "Terminal height") {
                        Segments(name: "Terminal height", options: [(0, "Fill"), (250, "Short"), (350, "Medium"), (500, "Tall"), (700, "Tallest")], selection: $model.preferences.terminalHeight.animation(change))
                        HStack(alignment: .top, spacing: 14) {
                            MiniScreen(terminal: model.preferences.terminalHeight, drawer: model.preferences.widgets ? Self.widgets.filter { model.preferences.visible($0.key) }.map(\.tone) : [])
                            VStack(alignment: .leading, spacing: 8) {
                                Text("Details drawer under the terminal").font(.system(size: 13)).foregroundStyle(Crab.soft).fixedSize(horizontal: false, vertical: true)
                                Segments(name: "Details drawer", options: [(true, "Open"), (false, "Tucked away")], selection: $model.preferences.widgets.animation(change), size: 13)
                                PanelNote("The small screen redraws as you choose a height and what the drawer holds.")
                            }
                        }
                    }
                    PanelSection(title: "In the drawer") {
                        FlowLayout(spacing: 8) {
                            ForEach(Self.widgets, id: \.key) { widget in
                                ToggleChip(label: widget.label, on: model.preferences.visible(widget.key), tone: widget.tone) { withAnimation(change) { model.preferences.toggle(widget.key) } }
                            }
                        }
                    }
                    PanelSection(title: "The stats strip shows") {
                        ScrollView(.horizontal, showsIndicators: false) {
                            HStack(spacing: 10) {
                                Text("✓").foregroundStyle(Crab.purple)
                                ForEach(Self.stats.filter { model.preferences.shows($0.key, in: .sessionStats) }, id: \.key) { Text($0.text).foregroundStyle($0.color) }
                            }.font(.system(size: 11, design: .monospaced)).lineLimit(1).fixedSize().padding(.horizontal, 12).frame(minHeight: 34)
                        }.scrollBounceBehavior(.basedOnSize, axes: .horizontal).background(Crab.ink, in: RoundedRectangle(cornerRadius: 10)).accessibilityElement(children: .ignore).accessibilityLabel("Sample stats strip").accessibilityIdentifier("Sample stats strip")
                        FlowLayout(spacing: 8) {
                            ForEach(Preferences.Shown.sessionStats.items, id: \.self) { key in
                                ToggleChip(label: BoardViewPanel.labels[key] ?? key, on: model.preferences.shows(key, in: .sessionStats)) { withAnimation(change) { model.preferences.toggle(key, in: .sessionStats) } }
                            }
                        }
                    }
                    NavigationLink { VoiceTranscripts(model: model, sessionID: sessionID) } label: {
                        HStack { Text("Voice transcripts for this session").font(.system(size: 15)).foregroundStyle(Crab.text); Spacer(); Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold)).foregroundStyle(Crab.dim) }
                            .frame(minHeight: 48).padding(.top, 6).contentShape(Rectangle()).overlay(alignment: .top) { Crab.line.frame(height: 1) }
                    }.buttonStyle(.plain)
                }.padding(.horizontal, 16).padding(.top, 4).padding(.bottom, 24)
            }.accessibilityIdentifier("View panel")
            }.background(Crab.panel).foregroundStyle(Crab.text)
                .navigationTitle("View").navigationBarTitleDisplayMode(.inline).toolbar(.hidden, for: .navigationBar)
        }.tint(Crab.cyan)
    }
}

/// The session screen in miniature: header, terminal, the drawer's widgets, the stats
/// strip and the message box. It follows the height and drawer choices beside it.
struct MiniScreen: View {
    /// The saved terminal height in points; 0 fills the space.
    var terminal: Int
    /// One colour per widget the open drawer holds.
    var drawer: [Color]
    private let lines: [(CGFloat, String)] = [(0.82, "6B7684"), (0.58, "5fff5f"), (0.9, "6B7684"), (0.44, "6B7684"), (0.72, "d7af5f"), (0.86, "6B7684"), (0.36, "6B7684"), (0.66, "5fff5f"), (0.8, "6B7684"), (0.52, "6B7684"), (0.74, "6B7684"), (0.4, "d7af5f"), (0.88, "6B7684"), (0.6, "6B7684")]
    private let widths: [CGFloat] = [0.6, 0.76, 0.52, 0.48, 0.66, 0.4]
    var body: some View {
        // The drawing is 150 points tall for a screen of about 700.
        let room: CGFloat = 108, drawerHeight = drawer.isEmpty ? 0 : CGFloat(drawer.count) * 7 + 6
        let height = terminal == 0 ? room - drawerHeight : min(CGFloat(terminal) * 0.12, room - drawerHeight)
        VStack(spacing: 0) {
            Crab.ink.frame(height: 14)
            VStack(alignment: .leading, spacing: 4) {
                ForEach(lines.indices, id: \.self) { index in Crab.hex(lines[index].1).frame(width: 90 * lines[index].0, height: 2) }
            }.padding(.horizontal, 7).padding(.top, 6).frame(width: 104, height: height, alignment: .topLeading).clipped()
            Crab.ink.frame(maxHeight: .infinity)
            if !drawer.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(drawer.indices, id: \.self) { index in Capsule().fill(drawer[index]).frame(width: 90 * widths[index % widths.count], height: 3) }
                }.padding(.horizontal, 7).padding(.vertical, 5).frame(maxWidth: .infinity, alignment: .leading).background(Crab.tile).overlay(alignment: .top) { Crab.line.frame(height: 1) }
            }
            Crab.panel.frame(height: 12).overlay(alignment: .top) { Crab.line.frame(height: 1) }
            Crab.ink.frame(height: 16).overlay(alignment: .top) { Crab.line.frame(height: 1) }
        }.frame(width: 104, height: 150).background(Crab.well).clipShape(RoundedRectangle(cornerRadius: 12)).overlay { RoundedRectangle(cornerRadius: 12).strokeBorder(Crab.track) }
            .accessibilityElement(children: .ignore).accessibilityLabel("Drawing of the session screen").accessibilityIdentifier("Session screen drawing")
    }
}
