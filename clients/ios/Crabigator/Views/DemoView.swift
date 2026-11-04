import SwiftUI

/// An offline walkthrough, available to everyone without an account or pairing.
struct DemoView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var selected: Int?
    @State private var board = false
    @State private var answer: String?
    @State private var draft = ""
    @State private var messages = ""
    @State private var pinned = true
    @State private var preferences = Preferences()
    @FocusState private var composing: Bool
    private let titles = ["Add dark mode", "Improve search"]
    private var choices: some View {
        ForEach(["System", "Dark", "Light"], id: \.self) { choice in
            Button(choice) { answer = choice; pinned = true }.buttonStyle(.bordered)
        }
    }
    private var screen: String {
        if selected == 1 {
            return "Sample session · Codex\n\n› Make search easier to use.\n\nAdded keyboard navigation and clearer empty states.\n\n  ✓ 18 tests passed\n  + 64 lines  − 12 lines\n\nPR #24 · Improve search · Checks passed\n\nReady for review." + messages
        }
        return "Sample session · Claude Code\n\n› Add a dark theme to the settings page.\n\nUpdated the theme colors and saved the appearance preference.\n\n  ✓ 12 tests passed\n  + 42 lines  − 8 lines\n\n" + (answer.map { "› \($0)\n\nSample answer received. The preview now uses \($0.lowercased())." } ?? "Which theme should be the default?") + messages
    }
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                Text("Sample data · Nothing is sent to a computer or AI service")
                    .font(.caption).foregroundStyle(Crab.mint).padding(12).frame(maxWidth: .infinity).background(Crab.panel)
                if selected == nil {
                    Picker("View", selection: $board) { Text("Sessions").tag(false); Text("PR board").tag(true) }
                        .pickerStyle(.segmented).padding()
                    List {
                        Section("Demo computer · example / website") {
                            ForEach(titles.indices, id: \.self) { index in
                                Button { selected = index; messages = ""; draft = ""; pinned = true } label: {
                                    VStack(alignment: .leading, spacing: 8) {
                                        Text(board ? "PR #\(23 + index) · \(titles[index])" : titles[index]).foregroundStyle(Crab.title)
                                        Text(index == 0 && answer == nil ? "Question · Choose a default theme" : "Complete · Checks passed").font(.caption).foregroundStyle(index == 0 && answer == nil ? Crab.orange : Crab.mint)
                                        Text(index == 0 ? "+42 −8 · Claude Code" : "+64 −12 · Codex").font(.caption.monospaced()).foregroundStyle(Crab.muted)
                                    }.padding(.vertical, 8)
                                }.listRowBackground(Crab.panel)
                            }
                        }
                    }.scrollContentBackground(.hidden)
                    Text("Pair your computer to see live sessions, send prompts, and receive notifications.")
                        .font(.footnote).foregroundStyle(Crab.muted).padding()
                } else {
                    HStack {
                        Text(titles[selected!]).font(.headline)
                        Spacer()
                        Button("A−") { preferences.fontSize = max(9, preferences.fontSize - 1) }.accessibilityLabel("Smaller terminal text")
                        Button("A+") { preferences.fontSize = min(24, preferences.fontSize + 1) }.accessibilityLabel("Larger terminal text")
                    }.padding()
                    TerminalViewport(history: "", screen: screen, preferences: preferences, pinned: $pinned)
                    if selected == 0 && answer == nil && !composing {
                        VStack(alignment: .leading, spacing: 10) {
                            Text("Which theme should be the default?").font(.headline)
                            ViewThatFits(in: .horizontal) {
                                HStack { choices }.fixedSize(horizontal: true, vertical: false)
                                VStack(alignment: .leading) { choices }
                            }
                        }.padding().frame(maxWidth: .infinity, alignment: .leading).background(Crab.panel)
                    }
                    HStack {
                        TextField("Try a sample message", text: $draft, axis: .vertical).lineLimit(1...4).textFieldStyle(.roundedBorder).focused($composing).accessibilityIdentifier("demo-message")
                        Button("Send") {
                            messages += "\n\n› \(draft)\n\nDemo only: your message stayed on this device. Pair a computer to send real prompts."
                            draft = ""; pinned = true; composing = false
                        }.disabled(draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    }.padding()
                }
            }.background(Crab.ink).navigationTitle("Crabigator demo").navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItemGroup(placement: .keyboard) { Spacer(); Button("Hide keyboard") { composing = false }.accessibilityIdentifier("demo-hide-keyboard") }
                    ToolbarItem(placement: .topBarLeading) { if selected != nil { Button("Back") { selected = nil } } }
                    ToolbarItem(placement: .topBarTrailing) { Button("Done") { dismiss() } }
                }
        }.preferredColorScheme(.dark).tint(Crab.orange)
    }
}
