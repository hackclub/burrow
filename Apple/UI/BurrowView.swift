import BurrowConfiguration
import Foundation
import SwiftUI
#if canImport(AuthenticationServices)
import AuthenticationServices
#endif
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
        guard !didRunAutomation,
              let automation = BurrowAutomationConfig.current,
              automation.action == .tailnetLogin || automation.action == .headscaleProbe
        else {
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
        case .tailnet: .tailnet
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
        .accessibilityIdentifier("quick-add-\(sheet.rawValue)")
        .buttonStyle(.floating(color: sheet.quickActionColor, cornerRadius: 18))
    }
}

private struct AccountDraft {
    var title = ""
    var accountName = ""
    var identityName = ""
    var wireGuardConfig = ""

    var discoveryEmail = ""
    var authority = ""
    var tailnet = ""
    var hostname = ProcessInfo.processInfo.hostName
    var username = ""
    var secret = ""
    var authMode: AccountAuthMode = .none

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
            authMode = .web
        }
    }
}

private struct ConfigurationSheetView: View {
    @Environment(\.dismiss) private var dismiss

    let sheet: ConfigurationSheet
    let networkViewModel: NetworkViewModel
    let accountStore: NetworkAccountStore

    @State private var draft: AccountDraft
    @State private var isSubmitting = false
    @State private var errorMessage: String?
    @State private var discoveryStatus: TailnetDiscoveryResponse?
    @State private var discoveryError: String?
    @State private var isDiscoveringTailnet = false
    @State private var authorityProbeStatus: TailnetAuthorityProbeStatus?
    @State private var authorityProbeError: String?
    @State private var isProbingAuthority = false
    @State private var tailnetLoginStatus: TailnetLoginStatus?
    @State private var tailnetLoginError: String?
    @State private var tailnetLoginSessionID: String?
    @State private var isStartingTailnetLogin = false
    @State private var tailnetPresentedAuthURL: URL?
    @State private var preserveTailnetLoginSession = false
    @State private var browserAuthenticator = TailnetBrowserAuthenticator()
    @State private var tailnetLoginPollTask: Task<Void, Never>?
    @State private var didRunAutomation = false

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
                        Task { @MainActor in
                            await cancelTailnetLoginIfNeeded()
                            dismiss()
                        }
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
        .onChange(of: draft.authority) { _, _ in
            resetAuthorityProbe()
        }
        .onChange(of: draft.discoveryEmail) { _, _ in
            resetTailnetDiscoveryFeedback()
        }
        .onChange(of: draft.authMode) { _, newMode in
            guard newMode != .web else { return }
            Task { @MainActor in
                await cancelTailnetLoginIfNeeded()
            }
        }
        .onDisappear {
            tailnetLoginPollTask?.cancel()
            browserAuthenticator.cancel()
            if !preserveTailnetLoginSession {
                Task { @MainActor in
                    await cancelTailnetLoginIfNeeded()
                }
            }
        }
    }

    @ViewBuilder
    private var tailnetSections: some View {
        Section("Connection") {
            TextField("Email address", text: $draft.discoveryEmail)
                .burrowEmailField()
                .burrowLoginField()
                .autocorrectionDisabled()
                .accessibilityIdentifier("tailnet-discovery-email")

            Button {
                discoverTailnetAuthority()
            } label: {
                Label {
                    Text(isDiscoveringTailnet ? "Finding Server" : "Find Server")
                } icon: {
                    Image(systemName: isDiscoveringTailnet ? "hourglass" : "at.circle")
                }
            }
            .buttonStyle(.borderless)
            .disabled(isDiscoveringTailnet || normalizedOptional(draft.discoveryEmail) == nil)
            .accessibilityIdentifier("tailnet-find-server")

            if let discoveryStatus {
                tailnetDiscoveryCard(status: discoveryStatus, failure: nil)
            } else if let discoveryError {
                tailnetDiscoveryCard(status: nil, failure: discoveryError)
            }

            TextField("Authority URL", text: $draft.authority)
                .burrowLoginField()
                .autocorrectionDisabled()
                .accessibilityIdentifier("tailnet-authority")

            Text("Use the managed Tailnet authority or enter a custom Tailnet control server.")
                .font(.footnote)
                .foregroundStyle(.secondary)

            Button {
                probeTailnetAuthority()
            } label: {
                Label {
                    Text(isProbingAuthority ? "Checking Connection" : "Check Connection")
                } icon: {
                    Image(systemName: isProbingAuthority ? "hourglass" : "bolt.horizontal.circle")
                }
            }
            .buttonStyle(.borderless)
            .disabled(isProbingAuthority || normalizedOptional(draft.authority) == nil)
            .accessibilityIdentifier("tailnet-check-connection")

            if let authorityProbeStatus {
                tailnetAuthorityProbeCard(status: authorityProbeStatus, failure: nil)
            } else if let authorityProbeError {
                tailnetAuthorityProbeCard(status: nil, failure: authorityProbeError)
            }

            TextField("Tailnet", text: $draft.tailnet)
                .burrowLoginField()
                .autocorrectionDisabled()
                .accessibilityIdentifier("tailnet-name")
        }

        Section("Authentication") {
            Picker("Authentication", selection: $draft.authMode) {
                ForEach(availableTailnetAuthModes) { mode in
                    Text(mode.title).tag(mode)
                }
            }
            .pickerStyle(.menu)

            if draft.authMode == .web {
                Button {
                    startTailnetLogin()
                } label: {
                    Label {
                        Text(isStartingTailnetLogin ? "Starting Sign-In" : tailnetSignInActionTitle)
                    } icon: {
                        Image(systemName: isStartingTailnetLogin ? "hourglass" : "person.badge.key")
                    }
                }
                .buttonStyle(.borderless)
                .disabled(isStartingTailnetLogin || normalizedOptional(draft.authority) == nil)
                .accessibilityIdentifier("tailnet-start-sign-in")

                if let tailnetLoginStatus {
                    tailnetLoginCard(status: tailnetLoginStatus, failure: nil)
                } else if let tailnetLoginError {
                    tailnetLoginCard(status: nil, failure: tailnetLoginError)
                }
            } else {
                TextField("Username", text: $draft.username)
                    .burrowLoginField()
                    .autocorrectionDisabled()
                if draft.authMode != .none {
                    SecureField(
                        draft.authMode == .password ? "Password" : "Preauth Key",
                        text: $draft.secret
                    )
                }
            }

            Text(tailnetAuthenticationFootnote)
                .font(.footnote)
                .foregroundStyle(.secondary)
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
                if let authorityProbeStatus {
                    Text(authorityProbeStatus.summary)
                        .font(.footnote.weight(.medium))
                        .foregroundStyle(.primary)
                    if let detail = authorityProbeStatus.detail {
                        Text(detail)
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                            .lineLimit(3)
                    }
                } else if let authorityProbeError {
                    Text("Connection failed")
                        .font(.footnote.weight(.medium))
                        .foregroundStyle(.red)
                    Text(authorityProbeError)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                        .lineLimit(3)
                }
            }

            if sheet == .tailnet {
                HStack(spacing: 8) {
                    summaryBadge(isManagedTailnetAuthority ? "Managed" : "Custom")
                    summaryBadge(draft.authMode.title)
                    if tailnetLoginStatus?.running == true {
                        summaryBadge("Signed In")
                    }
                }
            }
        }
        .padding(14)
        .background(
            RoundedRectangle(cornerRadius: 18)
                .fill(.thinMaterial)
        )
    }

    private func tailnetAuthorityProbeCard(
        status: TailnetAuthorityProbeStatus?,
        failure: String?
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            if let status {
                Text(status.summary)
                    .font(.subheadline.weight(.medium))
                Text(status.detail ?? "HTTP \(status.statusCode) from \(status.authority)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            } else if let failure {
                Text("Connection failed")
                    .font(.subheadline.weight(.medium))
                    .foregroundStyle(.red)
                Text(failure)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(.thinMaterial)
        )
        .accessibilityIdentifier("tailnet-authority-probe-card")
    }

    private func tailnetDiscoveryCard(
        status: TailnetDiscoveryResponse?,
        failure: String?
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            if let status {
                Text("Discovered Tailnet Server")
                    .font(.subheadline.weight(.medium))
                Text(status.authority)
                    .font(.footnote.monospaced())
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                Text(status.provider == .tailscale ? "Managed authority" : "Custom authority")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                if let oidcIssuer = status.oidcIssuer {
                    Text("OIDC: \(oidcIssuer)")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                        .lineLimit(3)
                        .textSelection(.enabled)
                }
            } else if let failure {
                Text("Discovery failed")
                    .font(.subheadline.weight(.medium))
                    .foregroundStyle(.red)
                Text(failure)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(.thinMaterial)
        )
        .accessibilityIdentifier("tailnet-discovery-card")
    }

    private func tailnetLoginCard(
        status: TailnetLoginStatus?,
        failure: String?
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            if let status {
                Text(status.running ? "Signed In" : status.needsLogin ? "Browser Sign-In Required" : "Checking Sign-In")
                    .font(.subheadline.weight(.medium))
                if let tailnetName = status.tailnetName, !tailnetName.isEmpty {
                    Text("Tailnet: \(tailnetName)")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
                if let selfDNSName = status.selfDNSName, !selfDNSName.isEmpty {
                    Text(selfDNSName)
                        .font(.footnote.monospaced())
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }
                if !status.tailnetIPs.isEmpty {
                    Text(status.tailnetIPs.joined(separator: ", "))
                        .font(.footnote.monospaced())
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }
                if !status.health.isEmpty {
                    Text(status.health.joined(separator: " • "))
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            } else if let failure {
                Text("Sign-In failed")
                    .font(.subheadline.weight(.medium))
                    .foregroundStyle(.red)
                Text(failure)
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(.thinMaterial)
        )
        .accessibilityIdentifier("tailnet-login-card")
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
            Button("Use Tailscale Managed Server") {
                applyTailnetDefaults(for: .tailscale)
            }

            if availableTailnetAuthModes.count > 1 {
                Menu("Authentication") {
                    ForEach(availableTailnetAuthModes) { mode in
                        Button(mode.title) {
                            draft.authMode = mode
                            if mode == .none {
                                draft.secret = ""
                            }
                        }
                    }
                }
            }

            Button("Clear Discovery Result") {
                resetTailnetDiscoveryFeedback()
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
            if normalizedOptional(draft.authority) == nil {
                return true
            }
            if draft.authMode == .web {
                return tailnetLoginStatus?.running != true
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
        let secret = (draft.authMode == .none || draft.authMode == .web) ? nil : draft.secret
        let username = normalizedOptional(draft.username)
        preserveTailnetLoginSession = draft.authMode == .web && tailnetLoginStatus?.running == true
        try await saveTailnetAccount(secret: secret, username: username)
        dismiss()
    }

    private func runAutomationIfNeeded() {
        guard !didRunAutomation,
              sheet == .tailnet,
              let automation = BurrowAutomationConfig.current,
              automation.action == .tailnetLogin || automation.action == .headscaleProbe
        else {
            return
        }

        didRunAutomation = true
        draft.title = automation.title ?? draft.title
        draft.accountName = automation.accountName ?? draft.accountName
        draft.identityName = automation.identityName ?? draft.identityName
        draft.hostname = automation.hostname ?? draft.hostname

        Task { @MainActor in
            switch automation.action {
            case .tailnetLogin:
                applyTailnetDefaults(for: .tailscale)
                startTailnetLogin()
            case .headscaleProbe:
                draft.authority = automation.authority ?? TailnetProvider.headscale.defaultAuthority ?? draft.authority
                probeTailnetAuthority()
            }
        }
    }

    private func saveTailnetAccount(secret: String?, username: String?) async throws {
        let provider = inferredTailnetProvider
        let title = titleOrFallback(
            hostnameFallback(from: draft.authority, fallback: "Tailnet")
        )

        let payload = TailnetNetworkPayload(
            provider: provider,
            authority: normalizedOptional(draft.authority) ?? normalizedOptional(provider.defaultAuthority ?? ""),
            account: normalized(draft.accountName, fallback: "default"),
            identity: normalized(draft.identityName, fallback: "apple"),
            tailnet: normalizedOptional(draft.tailnet),
            hostname: normalizedOptional(draft.hostname)
        )

        var noteParts: [String] = [
            isManagedTailnetAuthority ? "Managed Tailnet" : "Custom Tailnet",
            "Auth: \(draft.authMode.title)",
        ]

        if draft.authMode == .web, tailnetLoginStatus?.running == true {
            noteParts.append("Browser sign-in complete")
        }

        do {
            let networkID = try await networkViewModel.addTailnetNetwork(payload: payload)
            noteParts.append("Linked to daemon network #\(networkID)")
        } catch {
            noteParts.append("Daemon network add pending")
        }

        let record = NetworkAccountRecord(
            id: UUID(),
            kind: .tailnet,
            title: title,
            authority: payload.authority,
            provider: provider,
            accountName: payload.account,
            identityName: payload.identity,
            hostname: payload.hostname,
            username: username,
            tailnet: payload.tailnet,
            authMode: draft.authMode,
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

    private func applyTailnetDefaults(for provider: TailnetProvider) {
        resetTailnetDiscoveryFeedback()
        draft.authority = provider.defaultAuthority ?? ""
        if !availableTailnetAuthModes.contains(draft.authMode) {
            draft.authMode = .web
        }
    }

    private func startTailnetLogin() {
        guard let authority = normalizedOptional(draft.authority) else {
            tailnetLoginStatus = nil
            tailnetLoginError = "Enter a server URL first."
            return
        }

        isStartingTailnetLogin = true
        tailnetLoginError = nil
        preserveTailnetLoginSession = false

        Task { @MainActor in
            defer { isStartingTailnetLogin = false }
            do {
                let status = try await networkViewModel.startTailnetLogin(
                    accountName: normalized(draft.accountName, fallback: "default"),
                    identityName: normalized(draft.identityName, fallback: "apple"),
                    hostname: normalizedOptional(draft.hostname),
                    authority: authority
                )
                tailnetLoginSessionID = status.sessionID
                updateTailnetLoginStatus(status)
                beginTailnetLoginPolling(sessionID: status.sessionID)
            } catch {
                tailnetLoginError = error.localizedDescription
            }
        }
    }

    private func probeTailnetAuthority() {
        guard let authority = normalizedOptional(draft.authority) else {
            authorityProbeStatus = nil
            authorityProbeError = "Enter a server URL first."
            return
        }

        isProbingAuthority = true
        authorityProbeStatus = nil
        authorityProbeError = nil

        Task { @MainActor in
            defer { isProbingAuthority = false }
            do {
                authorityProbeStatus = try await networkViewModel.probeTailnetAuthority(authority)
            } catch {
                authorityProbeError = error.localizedDescription
            }
        }
    }

    private func resetAuthorityProbe() {
        authorityProbeStatus = nil
        authorityProbeError = nil
        tailnetLoginError = nil
    }

    private func resetTailnetDiscoveryFeedback() {
        discoveryStatus = nil
        discoveryError = nil
    }

    private func discoverTailnetAuthority() {
        guard let email = normalizedOptional(draft.discoveryEmail) else {
            discoveryStatus = nil
            discoveryError = "Enter an email address first."
            return
        }

        isDiscoveringTailnet = true
        discoveryStatus = nil
        discoveryError = nil

        Task { @MainActor in
            defer { isDiscoveringTailnet = false }
            do {
                let discovery = try await networkViewModel.discoverTailnet(email: email)
                discoveryStatus = discovery
                draft.authority = discovery.authority
                probeTailnetAuthority()
            } catch {
                discoveryError = error.localizedDescription
            }
        }
    }

    private func beginTailnetLoginPolling(sessionID: String) {
        tailnetLoginPollTask?.cancel()
        tailnetLoginPollTask = Task { @MainActor in
            while !Task.isCancelled {
                do {
                    let status = try await networkViewModel.tailnetLoginStatus(sessionID: sessionID)
                    updateTailnetLoginStatus(status)
                    if status.running {
                        tailnetLoginPollTask = nil
                        return
                    }
                } catch {
                    tailnetLoginError = error.localizedDescription
                    tailnetLoginPollTask = nil
                    return
                }
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }

    private func updateTailnetLoginStatus(_ status: TailnetLoginStatus) {
        tailnetLoginStatus = status
        tailnetLoginError = nil
        tailnetLoginSessionID = status.sessionID

        if status.running {
            browserAuthenticator.cancel()
            tailnetPresentedAuthURL = nil
            return
        }

        guard let authURL = status.authURL else {
            return
        }

        if tailnetPresentedAuthURL != authURL {
            tailnetPresentedAuthURL = authURL
            browserAuthenticator.start(url: authURL) { [sessionID = status.sessionID] in
                Task { @MainActor in
                    if tailnetLoginStatus?.running != true {
                        tailnetLoginSessionID = sessionID
                    }
                }
            }
        }
    }

    private func cancelTailnetLoginIfNeeded() async {
        tailnetLoginPollTask?.cancel()
        tailnetLoginPollTask = nil
        browserAuthenticator.cancel()
        tailnetPresentedAuthURL = nil

        guard tailnetLoginStatus?.running != true,
              let sessionID = tailnetLoginSessionID
        else {
            return
        }

        do {
            try await networkViewModel.cancelTailnetLogin(sessionID: sessionID)
        } catch {
            tailnetLoginError = error.localizedDescription
        }

        tailnetLoginStatus = nil
        tailnetLoginSessionID = nil
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

    private var availableTailnetAuthModes: [AccountAuthMode] {
        [.web, .none, .password, .preauthKey]
    }

    private var tailnetSignInActionTitle: String {
        if tailnetLoginStatus?.running == true {
            return "Signed In"
        }
        if tailnetLoginSessionID != nil {
            return "Resume Sign-In"
        }
        return "Start Sign-In"
    }

    private var tailnetAuthenticationFootnote: String {
        switch draft.authMode {
        case .web:
            return "Burrow asks the daemon to start a Tailnet browser sign-in session, then closes it locally once the daemon reports the device is running."
        case .none:
            return "Save the authority only. Useful when the control plane handles authentication elsewhere."
        case .password, .preauthKey:
            return "Tailnet account material stays on-device. Burrow stores the authority and credentials for daemon-managed registration and refresh."
        }
    }

    private var inferredTailnetProvider: TailnetProvider {
        TailnetProvider.inferred(
            authority: normalizedOptional(draft.authority),
            explicit: discoveryStatus?.provider
        )
    }

    private var isManagedTailnetAuthority: Bool {
        TailnetProvider.isManagedTailscaleAuthority(normalizedOptional(draft.authority))
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

    @ViewBuilder
    func burrowEmailField() -> some View {
        #if os(iOS)
        textInputAutocapitalization(.never)
            .keyboardType(.emailAddress)
        #else
        self
        #endif
    }
}

#if canImport(AuthenticationServices)
@MainActor
private final class TailnetBrowserAuthenticator: NSObject {
    private var session: ASWebAuthenticationSession?

    func start(url: URL, onDismiss: @escaping @Sendable () -> Void) {
        cancel()
        let session = ASWebAuthenticationSession(url: url, callbackURLScheme: nil) { _, _ in
            onDismiss()
        }
        session.presentationContextProvider = self
        session.prefersEphemeralWebBrowserSession = false
        self.session = session
        _ = session.start()
    }

    func cancel() {
        session?.cancel()
        session = nil
    }
}

extension TailnetBrowserAuthenticator: ASWebAuthenticationPresentationContextProviding {
    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        #if canImport(AppKit)
        return NSApplication.shared.keyWindow
            ?? NSApplication.shared.windows.first
            ?? ASPresentationAnchor()
        #elseif canImport(UIKit)
        return ASPresentationAnchor()
        #else
        return ASPresentationAnchor()
        #endif
    }
}
#else
@MainActor
private final class TailnetBrowserAuthenticator {
    func start(url: URL, onDismiss: @escaping @Sendable () -> Void) {
        _ = url
        onDismiss()
    }

    func cancel() {}
}
#endif

private struct BurrowAutomationConfig {
    enum Action: String {
        case tailnetLogin = "tailnet-login"
        case headscaleProbe = "headscale-probe"
    }

    let action: Action
    let title: String?
    let accountName: String?
    let identityName: String?
    let hostname: String?
    let authority: String?

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
            hostname: environment["BURROW_UI_AUTOMATION_HOSTNAME"],
            authority: environment["BURROW_UI_AUTOMATION_AUTHORITY"]
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
