import AuthenticationServices
import BurrowConfiguration
import Foundation
import SwiftUI
#if canImport(UIKit)
import UIKit
#elseif canImport(AppKit)
import AppKit
#endif

public struct BurrowView: View {
    @State private var networkViewModel: NetworkViewModel
    @State private var accountStore = NetworkAccountStore()
    @State private var activeSheet: ConfigurationSheet?
    @State private var didRunAutomation = false

    public var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    HStack(alignment: .top) {
                        VStack(alignment: .leading, spacing: 6) {
                            Text("Burrow")
                                .font(.largeTitle)
                                .fontWeight(.bold)
                            if showsHeaderSubtitle {
                                Text("Networks and accounts")
                                    .font(.headline)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        if showsToolbarAddMenu {
                            Spacer()
                            Menu {
                                Button("Add WireGuard Network") {
                                    activeSheet = .wireGuard
                                }
                                Button("Save Tor Account") {
                                    activeSheet = .tor
                                }
                                Button("Add Tailnet Account") {
                                    activeSheet = .tailnet
                                }
                            } label: {
                                Image(systemName: "plus.circle.fill")
                                    .font(.title)
                                    .accessibilityLabel("Add")
                            }
                        }
                    }
                    .padding(.top)

                    if showsInlineQuickActions {
                        quickAddSection
                    }

                    VStack(alignment: .leading, spacing: 12) {
                        sectionHeader(
                            title: "Networks",
                            detail: showsInlineQuickActions
                                ? nil
                                : "Stored daemon networks and their active account selectors"
                        )
                        if let connectionError = networkViewModel.connectionError {
                            Text(connectionError)
                                .font(.footnote)
                                .foregroundStyle(.secondary)
                        }
                        NetworkCarouselView(networks: networkViewModel.cards)
                    }

                    if showsAccountsSection {
                        VStack(alignment: .leading, spacing: 12) {
                            sectionHeader(
                                title: "Accounts",
                                detail: showsInlineQuickActions
                                    ? nil
                                    : "Per-network identities and sign-in state"
                            )
                            if accountStore.accounts.isEmpty {
                                ContentUnavailableView(
                                    "No Accounts Yet",
                                    systemImage: "person.crop.circle.badge.plus",
                                    description: Text("Save a Tor account or sign in to a Tailnet provider to keep network identities ready on this device.")
                                )
                                .frame(maxWidth: .infinity, minHeight: 180)
                            } else {
                                LazyVStack(spacing: 12) {
                                    ForEach(accountStore.accounts) { account in
                                        AccountRowView(
                                            account: account,
                                            hasSecret: accountStore.hasStoredSecret(for: account)
                                        )
                                    }
                                }
                            }
                        }
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        sectionHeader(
                            title: "Tunnel",
                            detail: showsInlineQuickActions ? nil : "Current system extension state"
                        )
                        TunnelStatusView()
                        TunnelButton()
                            .padding(.bottom)
                    }
                }
                .padding()
            }
        }
        .sheet(item: $activeSheet) { sheet in
            ConfigurationSheetView(
                sheet: sheet,
                networkViewModel: networkViewModel,
                accountStore: accountStore
            )
        }
        .onAppear {
            runAutomationIfNeeded()
        }
    }

    public init() {
        _networkViewModel = State(
            initialValue: NetworkViewModel(
                socketURLResult: Result { try Constants.socketURL }
            )
        )
    }

    private func runAutomationIfNeeded() {
        guard !didRunAutomation, BurrowAutomationConfig.current?.action == .tailnetLogin else {
            return
        }
        didRunAutomation = true
        activeSheet = .tailnet
    }

    @ViewBuilder
    private var quickAddSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            sectionHeader(title: "Add", detail: nil)
            VStack(spacing: 12) {
                ForEach(ConfigurationSheet.allCases) { sheet in
                    QuickAddButton(sheet: sheet) {
                        activeSheet = sheet
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func sectionHeader(title: String, detail: String?) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.title2.weight(.semibold))
            if let detail, !detail.isEmpty {
                Text(detail)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var showsInlineQuickActions: Bool {
        #if os(iOS)
        true
        #else
        false
        #endif
    }

    private var showsToolbarAddMenu: Bool {
        !showsInlineQuickActions
    }

    private var showsHeaderSubtitle: Bool {
        !showsInlineQuickActions
    }

    private var showsAccountsSection: Bool {
        #if os(iOS)
        !accountStore.accounts.isEmpty
        #else
        true
        #endif
    }
}

private enum ConfigurationSheet: String, CaseIterable, Identifiable {
    case wireGuard
    case tor
    case tailnet

    var id: String { rawValue }

    var kind: AccountNetworkKind {
        switch self {
        case .wireGuard: .wireGuard
        case .tor: .tor
        case .tailnet: .headscale
        }
    }

    var iconName: String {
        switch self {
        case .wireGuard:
            "wave.3.right"
        case .tor:
            "shield.lefthalf.filled.badge.checkmark"
        case .tailnet:
            "network.badge.shield.half.filled"
        }
    }

    var quickActionTitle: String {
        switch self {
        case .wireGuard:
            "WireGuard"
        case .tor:
            "Tor"
        case .tailnet:
            "Tailnet"
        }
    }

    var quickActionSubtitle: String {
        switch self {
        case .wireGuard:
            "Import a tunnel"
        case .tor:
            "Save an Arti profile"
        case .tailnet:
            "Sign in or save a control plane"
        }
    }

    var quickActionColor: Color {
        switch self {
        case .wireGuard:
            .blue
        case .tor, .tailnet:
            kind.accentColor
        }
    }
}

private struct QuickAddButton: View {
    let sheet: ConfigurationSheet
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 14) {
                Image(systemName: sheet.iconName)
                    .font(.title3.weight(.semibold))
                    .frame(width: 24)

                VStack(alignment: .leading, spacing: 4) {
                    Text(sheet.quickActionTitle)
                        .font(.headline)
                    Text(sheet.quickActionSubtitle)
                        .font(.caption)
                        .opacity(0.88)
                }

                Spacer()
            }
            .frame(maxWidth: .infinity, minHeight: 64, alignment: .leading)
        }
        .buttonStyle(.floating(color: sheet.quickActionColor, cornerRadius: 18))
    }
}

private struct AccountDraft {
    var title = ""
    var accountName = ""
    var identityName = ""
    var wireGuardConfig = ""

    var tailnetProvider: TailnetProvider = .tailscale
    var authority = ""
    var tailnet = ""
    var hostname = ProcessInfo.processInfo.hostName
    var username = ""
    var secret = ""
    var authMode: AccountAuthMode = .web

    var torAddresses = "100.64.0.2/32"
    var torDNS = "1.1.1.1, 1.0.0.1"
    var torMTU = "1400"
    var torListen = "127.0.0.1:9040"

    init(sheet: ConfigurationSheet) {
        switch sheet {
        case .wireGuard:
            break
        case .tor:
            title = "Default Tor"
            accountName = "default"
            identityName = "apple"
        case .tailnet:
            title = "Tailnet"
            accountName = "default"
            identityName = "apple"
            authority = TailnetProvider.tailscale.defaultAuthority ?? ""
        }
    }
}

private struct ConfigurationSheetView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.webAuthenticationSession) private var webAuthenticationSession

    let sheet: ConfigurationSheet
    let networkViewModel: NetworkViewModel
    let accountStore: NetworkAccountStore

    @State private var draft: AccountDraft
    @State private var isSubmitting = false
    @State private var errorMessage: String?
    @State private var loginSessionID: String?
    @State private var loginStatus: TailnetLoginStatus?
    @State private var pollingTask: Task<Void, Never>?
    @State private var didRunAutomation = false
    @State private var webAuthenticationTask: Task<Void, Never>?

    init(
        sheet: ConfigurationSheet,
        networkViewModel: NetworkViewModel,
        accountStore: NetworkAccountStore
    ) {
        self.sheet = sheet
        self.networkViewModel = networkViewModel
        self.accountStore = accountStore
        _draft = State(initialValue: AccountDraft(sheet: sheet))
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    sheetSummaryCard
                }
                .listRowInsets(.init(top: 4, leading: 0, bottom: 4, trailing: 0))
                .listRowBackground(Color.clear)

                Section("Identity") {
                    TextField("Title", text: $draft.title)
                    TextField("Account", text: $draft.accountName)
                    TextField("Identity", text: $draft.identityName)
                    if sheet == .tailnet {
                        TextField("Hostname", text: $draft.hostname)
                            .burrowLoginField()
                            .autocorrectionDisabled()
                    }
                }

                switch sheet {
                case .wireGuard:
                    Section("WireGuard Configuration") {
                        TextEditor(text: $draft.wireGuardConfig)
                            .font(.body.monospaced())
                            .frame(minHeight: wireGuardEditorHeight)
                            .contextMenu {
                                wireGuardContextActions
                            }
                    }
                case .tor:
                    Section("Tor Preferences") {
                        TextField("Virtual Addresses", text: $draft.torAddresses)
                        TextField("DNS Resolvers", text: $draft.torDNS)
                        TextField("MTU", text: $draft.torMTU)
                        TextField("Transparent Listener", text: $draft.torListen)
                    }
                case .tailnet:
                    tailnetSections
                }

                if let errorMessage {
                    Section {
                        Text(errorMessage)
                            .foregroundStyle(.red)
                    }
                }
            }
            .navigationTitle(sheet.kind.title)
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                #if os(iOS)
                ToolbarItem(placement: .topBarTrailing) {
                    Menu {
                        sheetMenuActions
                    } label: {
                        Image(systemName: "ellipsis.circle")
                    }
                    .accessibilityLabel("More")
                }
                #else
                ToolbarItem(placement: .primaryAction) {
                    Menu {
                        sheetMenuActions
                    } label: {
                        Image(systemName: "ellipsis.circle")
                    }
                    .accessibilityLabel("More")
                }
                #endif
                if !showsBottomActionButton {
                    ToolbarItem(placement: .confirmationAction) {
                        Button(confirmationTitle) {
                            submit()
                        }
                        .disabled(isSubmitting || submissionDisabled)
                    }
                }
            }
        }
        #if os(macOS)
        .frame(minWidth: 520, minHeight: 620)
        #endif
        .safeAreaInset(edge: .bottom) {
            if showsBottomActionButton {
                bottomActionBar
            }
        }
        .onAppear {
            runAutomationIfNeeded()
        }
        .onDisappear {
            pollingTask?.cancel()
            webAuthenticationTask?.cancel()
            webAuthenticationTask = nil
        }
    }

    @ViewBuilder
    private var tailnetSections: some View {
        Section("Connection") {
            Picker("Provider", selection: $draft.tailnetProvider) {
                ForEach(TailnetProvider.allCases) { provider in
                    Text(provider.title).tag(provider)
                }
            }
            .pickerStyle(.menu)

            tailnetProviderCard

            if draft.tailnetProvider.requiresControlURL {
                TextField("Server URL", text: $draft.authority)
                    .burrowLoginField()
                    .autocorrectionDisabled()
            } else {
                LabeledContent("Server") {
                    Text("Tailscale managed")
                        .foregroundStyle(.secondary)
                }
            }

            TextField("Tailnet", text: $draft.tailnet)
                .burrowLoginField()
                .autocorrectionDisabled()
        }

        Section("Authentication") {
            if draft.tailnetProvider.usesWebLogin {
                tailnetWebLoginCard
            } else {
                TextField("Username", text: $draft.username)
                    .burrowLoginField()
                    .autocorrectionDisabled()
                Picker("Authentication", selection: $draft.authMode) {
                    ForEach([AccountAuthMode.none, .password, .preauthKey]) { mode in
                        Text(mode.title).tag(mode)
                    }
                }
                .pickerStyle(.menu)
                if draft.authMode != .none {
                    SecureField(
                        draft.authMode == .password ? "Password" : "Preauth Key",
                        text: $draft.secret
                    )
                }
                Text("Credentials stay on-device. Burrow uses them when it needs to register or refresh this identity.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var sheetSummaryCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 12) {
                Image(systemName: sheet.iconName)
                    .font(.title3.weight(.semibold))
                    .foregroundStyle(sheetAccentColor)
                    .frame(width: 28, height: 28)
                    .background(
                        Circle()
                            .fill(sheetAccentColor.opacity(0.14))
                    )

                VStack(alignment: .leading, spacing: 3) {
                    Text(summaryTitle)
                        .font(.headline)
                    Text(sheet.kind.subtitle)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Spacer()
            }

            if let availabilityNote = sheet.kind.availabilityNote {
                Text(availabilityNote)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }

            if sheet == .tailnet {
                HStack(spacing: 8) {
                    summaryBadge(draft.tailnetProvider.title)
                    summaryBadge(
                        draft.tailnetProvider.usesWebLogin ? "Web Sign-In" : draft.authMode.title
                    )
                }
            }
        }
        .padding(14)
        .background(
            RoundedRectangle(cornerRadius: 18)
                .fill(.thinMaterial)
        )
    }

    private var tailnetProviderCard: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 10) {
                Image(systemName: tailnetProviderIconName)
                    .font(.headline)
                    .foregroundStyle(sheetAccentColor)
                    .frame(width: 28, height: 28)
                    .background(
                        Circle()
                            .fill(sheetAccentColor.opacity(0.14))
                    )

                VStack(alignment: .leading, spacing: 2) {
                    Text(draft.tailnetProvider.title)
                        .font(.headline)
                    Text(draft.tailnetProvider.subtitle)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }

                Spacer()
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(.thinMaterial)
        )
    }

    @ViewBuilder
    private var tailnetWebLoginCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Sign in with the shared browser session.")
                .font(.subheadline.weight(.medium))

            if let loginStatus {
                labeledValue("State", loginStatus.backendState)
                if let tailnetName = loginStatus.tailnetName {
                    labeledValue("Tailnet", tailnetName)
                }
                if let dnsName = loginStatus.selfDNSName {
                    labeledValue("Device", dnsName)
                }
                if !loginStatus.tailscaleIPs.isEmpty {
                    labeledValue("Addresses", loginStatus.tailscaleIPs.joined(separator: ", "))
                }
                if let authURL = loginStatus.authURL {
                    Button("Resume Sign-In") {
                        if let url = URL(string: authURL) {
                            openLoginURL(url)
                        }
                    }
                    .buttonStyle(.borderless)
                }
                if !loginStatus.health.isEmpty {
                    Text(loginStatus.health.joined(separator: " • "))
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            } else {
                Text("Burrow launches the local bridge, then opens the real Tailscale sign-in page in-app.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(.thinMaterial)
        )
    }

    private func summaryBadge(_ label: String) -> some View {
        Text(label)
            .font(.caption.weight(.medium))
            .foregroundStyle(.secondary)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(
                Capsule()
                    .fill(.white.opacity(0.5))
            )
    }

    @ViewBuilder
    private var bottomActionBar: some View {
        VStack(spacing: 0) {
            Divider()
                .overlay(.white.opacity(0.3))
            Button(confirmationTitle) {
                submit()
            }
            .buttonStyle(.floating(color: sheetAccentColor, cornerRadius: 18))
            .disabled(isSubmitting || submissionDisabled)
            .padding(.horizontal)
            .padding(.top, 12)
            .padding(.bottom, 8)
        }
        .background(.ultraThinMaterial)
    }

    @ViewBuilder
    private var sheetMenuActions: some View {
        Button("Use Suggested Identity") {
            applySuggestedIdentity()
        }

        switch sheet {
        case .wireGuard:
            Button("Paste Configuration") {
                pasteWireGuardConfiguration()
            }
            .disabled(clipboardString?.isEmpty ?? true)

            Button("Clear Configuration", role: .destructive) {
                draft.wireGuardConfig = ""
            }
            .disabled(draft.wireGuardConfig.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

        case .tor:
            Menu("Presets") {
                Button("Recommended Tor Defaults") {
                    applyTorDefaults()
                }
                Button("Restore Suggested Identity") {
                    applySuggestedIdentity()
                }
            }

        case .tailnet:
            Menu("Provider") {
                ForEach(TailnetProvider.allCases) { provider in
                    Button(provider.title) {
                        applyTailnetProvider(provider)
                    }
                }
            }

            if !draft.tailnetProvider.usesWebLogin {
                Menu("Authentication") {
                    ForEach([AccountAuthMode.none, .password, .preauthKey]) { mode in
                        Button(mode.title) {
                            draft.authMode = mode
                            if mode == .none {
                                draft.secret = ""
                            }
                        }
                    }
                }
            }

            Button("Restore Provider Defaults") {
                applyTailnetDefaults(for: draft.tailnetProvider)
            }
        }
    }

    @ViewBuilder
    private var wireGuardContextActions: some View {
        Button("Paste Configuration") {
            pasteWireGuardConfiguration()
        }
        .disabled(clipboardString?.isEmpty ?? true)

        Button("Clear", role: .destructive) {
            draft.wireGuardConfig = ""
        }
        .disabled(draft.wireGuardConfig.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
    }

    private var sheetAccentColor: Color {
        switch sheet {
        case .wireGuard:
            .blue
        case .tor, .tailnet:
            sheet.kind.accentColor
        }
    }

    private var summaryTitle: String {
        switch sheet {
        case .wireGuard:
            "Import WireGuard"
        case .tor:
            "Configure Tor"
        case .tailnet:
            "Connect Tailnet"
        }
    }

    private var tailnetProviderIconName: String {
        switch draft.tailnetProvider {
        case .tailscale:
            "globe.badge.chevron.backward"
        case .headscale:
            "server.rack"
        case .burrow:
            "shield"
        }
    }

    private var showsBottomActionButton: Bool {
        #if os(iOS)
        true
        #else
        false
        #endif
    }

    private var wireGuardEditorHeight: CGFloat {
        #if os(iOS)
        180
        #else
        220
        #endif
    }

    private var confirmationTitle: String {
        switch sheet {
        case .wireGuard:
            return "Add Network"
        case .tor:
            return "Save Account"
        case .tailnet:
            if draft.tailnetProvider.usesWebLogin {
                return loginStatus?.running == true ? "Save Account" : "Start Sign-In"
            }
            return "Save Account"
        }
    }

    private var submissionDisabled: Bool {
        switch sheet {
        case .wireGuard:
            return draft.wireGuardConfig.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        case .tor:
            return normalizedOptional(draft.accountName) == nil || normalizedOptional(draft.identityName) == nil
        case .tailnet:
            if normalizedOptional(draft.accountName) == nil || normalizedOptional(draft.identityName) == nil {
                return true
            }
            if draft.tailnetProvider.usesWebLogin {
                return false
            }
            if draft.tailnetProvider.requiresControlURL && normalizedOptional(draft.authority) == nil {
                return true
            }
            if draft.authMode != .none && normalizedOptional(draft.secret) == nil {
                return true
            }
            return false
        }
    }

    private func submit() {
        isSubmitting = true
        errorMessage = nil

        Task { @MainActor in
            defer { isSubmitting = false }
            do {
                switch sheet {
                case .wireGuard:
                    try await submitWireGuard()
                    dismiss()
                case .tor:
                    try submitTor()
                    dismiss()
                case .tailnet:
                    try await submitTailnet()
                }
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func submitWireGuard() async throws {
        let networkID = try await networkViewModel.addWireGuardNetwork(
            configText: draft.wireGuardConfig
        )

        let title = titleOrFallback("WireGuard \(networkID)")
        let record = NetworkAccountRecord(
            id: UUID(),
            kind: .wireGuard,
            title: title,
            authority: nil,
            provider: nil,
            accountName: normalized(draft.accountName, fallback: "default"),
            identityName: normalized(draft.identityName, fallback: "network-\(networkID)"),
            hostname: nil,
            username: nil,
            tailnet: nil,
            authMode: .none,
            note: "Linked to daemon network #\(networkID).",
            createdAt: .now,
            updatedAt: .now
        )
        try accountStore.upsert(record, secret: nil)
    }

    private func submitTor() throws {
        let title = titleOrFallback("Tor \(normalized(draft.identityName, fallback: "apple"))")
        let note = [
            "Addresses: \(csvSummary(draft.torAddresses))",
            "DNS: \(csvSummary(draft.torDNS))",
            "MTU: \(normalized(draft.torMTU, fallback: "1400"))",
            "Listen: \(normalized(draft.torListen, fallback: "127.0.0.1:9040"))",
        ].joined(separator: " • ")

        let record = NetworkAccountRecord(
            id: UUID(),
            kind: .tor,
            title: title,
            authority: "arti://local",
            provider: nil,
            accountName: normalized(draft.accountName, fallback: "default"),
            identityName: normalized(draft.identityName, fallback: "apple"),
            hostname: nil,
            username: nil,
            tailnet: nil,
            authMode: .none,
            note: note,
            createdAt: .now,
            updatedAt: .now
        )
        try accountStore.upsert(record, secret: nil)
    }

    private func submitTailnet() async throws {
        if draft.tailnetProvider.usesWebLogin {
            if loginStatus?.running == true {
                webAuthenticationTask?.cancel()
                webAuthenticationTask = nil
                try await saveTailnetAccount(secret: nil, username: nil)
                dismiss()
            } else {
                try await startTailscaleLogin()
            }
            return
        }

        let secret = draft.authMode == .none ? nil : draft.secret
        let username = normalizedOptional(draft.username)
        try await saveTailnetAccount(secret: secret, username: username)
        dismiss()
    }

    private func startTailscaleLogin() async throws {
        let response = try await TailnetBridgeClient.startLogin(
            TailnetLoginStartRequest(
                accountName: normalized(draft.accountName, fallback: "default"),
                identityName: normalized(draft.identityName, fallback: "apple"),
                hostname: normalizedOptional(draft.hostname),
                controlURL: draft.tailnetProvider.defaultAuthority
            )
        )
        loginSessionID = response.sessionID
        loginStatus = response.status
        if let authURL = response.status.authURL, let url = URL(string: authURL) {
            openLoginURL(url)
        }
        startPollingTailscaleLogin()
    }

    private func runAutomationIfNeeded() {
        guard !didRunAutomation,
              sheet == .tailnet,
              let automation = BurrowAutomationConfig.current,
              automation.action == .tailnetLogin
        else {
            return
        }

        didRunAutomation = true
        draft.tailnetProvider = .tailscale
        draft.title = automation.title ?? draft.title
        draft.accountName = automation.accountName ?? draft.accountName
        draft.identityName = automation.identityName ?? draft.identityName
        draft.hostname = automation.hostname ?? draft.hostname

        Task { @MainActor in
            do {
                try await startTailscaleLogin()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func startPollingTailscaleLogin() {
        pollingTask?.cancel()
        guard let loginSessionID else { return }
        pollingTask = Task { @MainActor in
            while !Task.isCancelled {
                do {
                    let status = try await TailnetBridgeClient.status(sessionID: loginSessionID)
                    let previousAuthURL = loginStatus?.authURL
                    loginStatus = status
                    if previousAuthURL == nil,
                       let authURL = status.authURL,
                       let url = URL(string: authURL)
                    {
                        openLoginURL(url)
                    }
                    if status.running {
                        webAuthenticationTask?.cancel()
                        webAuthenticationTask = nil
                        return
                    }
                } catch {
                    errorMessage = error.localizedDescription
                    return
                }
                try? await Task.sleep(for: .seconds(2))
            }
        }
    }

    private func openLoginURL(_ url: URL) {
        webAuthenticationTask?.cancel()
        webAuthenticationTask = Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(300))
            do {
                _ = try await webAuthenticationSession.authenticate(
                    using: url,
                    callbackURLScheme: "burrow",
                    preferredBrowserSession: .shared
                )
            } catch is CancellationError {
                return
            } catch let error as ASWebAuthenticationSessionError
                where error.code == .canceledLogin
            {
                return
            } catch {
                errorMessage = error.localizedDescription
            }
            webAuthenticationTask = nil
        }
    }

    private func saveTailnetAccount(secret: String?, username: String?) async throws {
        let provider = draft.tailnetProvider
        let title = titleOrFallback(
            hostnameFallback(
                from: provider.usesWebLogin ? (loginStatus?.tailnetName ?? "") : draft.authority,
                fallback: provider.title
            )
        )

        let payload = TailnetNetworkPayload(
            provider: provider,
            authority: normalizedOptional(provider.defaultAuthority ?? draft.authority),
            account: normalized(draft.accountName, fallback: "default"),
            identity: normalized(draft.identityName, fallback: "apple"),
            tailnet: normalizedOptional(loginStatus?.tailnetName ?? draft.tailnet),
            hostname: normalizedOptional(draft.hostname)
        )

        var noteParts: [String] = [
            provider.title,
            provider.usesWebLogin
                ? "State: \(loginStatus?.backendState ?? "NeedsLogin")"
                : "Auth: \(draft.authMode.title)",
        ]
        if let dnsName = loginStatus?.selfDNSName {
            noteParts.append("Device: \(dnsName)")
        }
        if let magicDNSSuffix = loginStatus?.magicDNSSuffix {
            noteParts.append("MagicDNS: \(magicDNSSuffix)")
        }

        do {
            let networkID = try await networkViewModel.addTailnetNetwork(payload: payload)
            noteParts.append("Linked to daemon network #\(networkID)")
        } catch {
            noteParts.append("Daemon network add pending")
        }

        let record = NetworkAccountRecord(
            id: UUID(),
            kind: .headscale,
            title: title,
            authority: payload.authority,
            provider: provider,
            accountName: payload.account,
            identityName: payload.identity,
            hostname: payload.hostname,
            username: username,
            tailnet: payload.tailnet,
            authMode: provider.usesWebLogin ? .web : draft.authMode,
            note: noteParts.joined(separator: " • "),
            createdAt: .now,
            updatedAt: .now
        )
        try accountStore.upsert(record, secret: secret)
    }

    private func applySuggestedIdentity() {
        let defaults = AccountDraft(sheet: sheet)
        if draft.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            draft.title = defaults.title
        }
        draft.accountName = defaults.accountName
        draft.identityName = defaults.identityName
        if sheet == .tailnet {
            draft.hostname = defaults.hostname
        }
    }

    private func applyTorDefaults() {
        let defaults = AccountDraft(sheet: .tor)
        draft.title = defaults.title
        draft.accountName = defaults.accountName
        draft.identityName = defaults.identityName
        draft.torAddresses = defaults.torAddresses
        draft.torDNS = defaults.torDNS
        draft.torMTU = defaults.torMTU
        draft.torListen = defaults.torListen
    }

    private func applyTailnetProvider(_ provider: TailnetProvider) {
        draft.tailnetProvider = provider
        applyTailnetDefaults(for: provider)
    }

    private func applyTailnetDefaults(for provider: TailnetProvider) {
        draft.authority = provider.defaultAuthority ?? ""
        if provider.usesWebLogin {
            draft.authMode = .web
            draft.username = ""
            draft.secret = ""
        } else {
            if draft.authMode == .web {
                draft.authMode = .none
            }
        }
    }

    private func pasteWireGuardConfiguration() {
        guard let clipboardString else { return }
        draft.wireGuardConfig = clipboardString
    }

    private var clipboardString: String? {
        #if canImport(UIKit)
        UIPasteboard.general.string
        #elseif canImport(AppKit)
        NSPasteboard.general.string(forType: .string)
        #else
        nil
        #endif
    }

    private func normalized(_ value: String, fallback: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? fallback : trimmed
    }

    private func normalizedOptional(_ value: String) -> String? {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    private func titleOrFallback(_ fallback: String) -> String {
        normalized(draft.title, fallback: fallback)
    }

    private func csvSummary(_ value: String) -> String {
        value
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
            .joined(separator: ", ")
    }

    private func hostnameFallback(from value: String, fallback: String) -> String {
        guard let url = URL(string: value), let host = url.host, !host.isEmpty else {
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? fallback : trimmed
        }
        return host
    }

    @ViewBuilder
    private func labeledValue(_ label: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.body.monospaced())
        }
    }
}

private struct AccountRowView: View {
    let account: NetworkAccountRecord
    let hasSecret: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(account.title)
                        .font(.headline)
                    HStack(spacing: 8) {
                        Text(account.kind.title)
                        if let provider = account.provider {
                            Text(provider.title)
                        }
                    }
                    .font(.subheadline)
                    .foregroundStyle(account.kind.accentColor)
                }
                Spacer()
                if hasSecret {
                    Label("Credential stored", systemImage: "key.fill")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            if let authority = account.authority {
                labeledValue("Authority", authority)
            }

            labeledValue("Account", account.accountName)
            labeledValue("Identity", account.identityName)

            if let hostname = account.hostname {
                labeledValue("Hostname", hostname)
            }

            if let username = account.username {
                labeledValue("Username", username)
            }

            if let tailnet = account.tailnet {
                labeledValue("Tailnet", tailnet)
            }

            if let note = account.note {
                Text(note)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(.thinMaterial)
        )
    }

    @ViewBuilder
    private func labeledValue(_ label: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.body.monospaced())
        }
    }
}

private extension View {
    @ViewBuilder
    func burrowLoginField() -> some View {
        #if os(iOS)
        textInputAutocapitalization(.never)
        #else
        self
        #endif
    }
}

private struct BurrowAutomationConfig {
    enum Action: String {
        case tailnetLogin = "tailnet-login"
    }

    let action: Action
    let title: String?
    let accountName: String?
    let identityName: String?
    let hostname: String?

    static let current: BurrowAutomationConfig? = {
        let environment = ProcessInfo.processInfo.environment
        guard let rawAction = environment["BURROW_UI_AUTOMATION"],
              let action = Action(rawValue: rawAction)
        else {
            return nil
        }

        return BurrowAutomationConfig(
            action: action,
            title: environment["BURROW_UI_AUTOMATION_TITLE"],
            accountName: environment["BURROW_UI_AUTOMATION_ACCOUNT"],
            identityName: environment["BURROW_UI_AUTOMATION_IDENTITY"],
            hostname: environment["BURROW_UI_AUTOMATION_HOSTNAME"]
        )
    }()
}

#if DEBUG
struct NetworkView_Previews: PreviewProvider {
    static var previews: some View {
        BurrowView()
            .environment(\.tunnel, PreviewTunnel())
    }
}
#endif
