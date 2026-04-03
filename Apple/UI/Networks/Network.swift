import BurrowConfiguration
import BurrowCore
import Foundation
import Security
import SwiftProtobuf
import SwiftUI

struct NetworkCardModel: Identifiable {
    let id: Int32
    let backgroundColor: Color
    let label: AnyView
}

struct TailnetNetworkPayload: Codable, Sendable {
    var provider: TailnetProvider
    var authority: String?
    var account: String
    var identity: String
    var tailnet: String?
    var hostname: String?

    func encoded() throws -> Data {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return try encoder.encode(self)
    }
}

struct TailnetDiscoveryResponse: Codable, Sendable {
    var domain: String
    var provider: TailnetProvider
    var authority: String
    var oidcIssuer: String?
}

struct TailnetAuthorityProbeStatus: Sendable {
    var authority: String
    var statusCode: Int
    var summary: String
    var detail: String?
}

struct TailnetLoginStatus: Sendable {
    var sessionID: String
    var backendState: String
    var authURL: URL?
    var running: Bool
    var needsLogin: Bool
    var tailnetName: String?
    var magicDNSSuffix: String?
    var selfDNSName: String?
    var tailnetIPs: [String]
    var health: [String]
}

enum TailnetDiscoveryClient {
    static func discover(email: String, socketURL: URL) async throws -> TailnetDiscoveryResponse {
        var request = Burrow_TailnetDiscoverRequest()
        request.email = email

        let response = try await TailnetClient.unix(socketURL: socketURL).discover(request)
        return TailnetDiscoveryResponse(
            domain: response.domain,
            provider: response.managed ? .tailscale : .headscale,
            authority: response.authority,
            oidcIssuer: response.oidcIssuer.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? nil
                : response.oidcIssuer
        )
    }
}

enum TailnetAuthorityProbeClient {
    static func probe(authority: String, socketURL: URL) async throws -> TailnetAuthorityProbeStatus {
        var request = Burrow_TailnetProbeRequest()
        request.authority = authority

        let response = try await TailnetClient.unix(socketURL: socketURL).probe(request)
        return TailnetAuthorityProbeStatus(
            authority: response.authority,
            statusCode: Int(response.statusCode),
            summary: response.summary,
            detail: response.detail.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? nil
                : response.detail
        )
    }
}

enum TailnetLoginClient {
    static func start(
        accountName: String,
        identityName: String,
        hostname: String?,
        authority: String,
        socketURL: URL
    ) async throws -> TailnetLoginStatus {
        var request = Burrow_TailnetLoginStartRequest()
        request.accountName = accountName
        request.identityName = identityName
        request.hostname = hostname ?? ""
        request.authority = authority
        let response = try await TailnetClient.unix(socketURL: socketURL).loginStart(request)
        return decode(response)
    }

    static func status(sessionID: String, socketURL: URL) async throws -> TailnetLoginStatus {
        var request = Burrow_TailnetLoginStatusRequest()
        request.sessionID = sessionID
        let response = try await TailnetClient.unix(socketURL: socketURL).loginStatus(request)
        return decode(response)
    }

    static func cancel(sessionID: String, socketURL: URL) async throws {
        var request = Burrow_TailnetLoginCancelRequest()
        request.sessionID = sessionID
        _ = try await TailnetClient.unix(socketURL: socketURL).loginCancel(request)
    }

    private static func decode(_ response: Burrow_TailnetLoginStatusResponse) -> TailnetLoginStatus {
        TailnetLoginStatus(
            sessionID: response.sessionID,
            backendState: response.backendState,
            authURL: URL(string: response.authURL.trimmingCharacters(in: .whitespacesAndNewlines)),
            running: response.running,
            needsLogin: response.needsLogin,
            tailnetName: response.tailnetName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? nil
                : response.tailnetName,
            magicDNSSuffix: response.magicDNSSuffix.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? nil
                : response.magicDNSSuffix,
            selfDNSName: response.selfDNSName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? nil
                : response.selfDNSName,
            tailnetIPs: response.tailnetIPs,
            health: response.health
        )
    }
}

@Observable
@MainActor
final class NetworkViewModel: Sendable {
    private(set) var networks: [Burrow_Network] = []
    private(set) var connectionError: String?
    private let socketURLResult: Result<URL, Error>

    @ObservationIgnored private var task: Task<Void, Never>?

    init(socketURLResult: Result<URL, Error>) {
        self.socketURLResult = socketURLResult
        startStreaming()
    }

    deinit {
        task?.cancel()
    }

    var cards: [NetworkCardModel] {
        networks.map(Self.makeCard(for:))
    }

    var nextNetworkID: Int32 {
        (networks.map(\.id).max() ?? 0) + 1
    }

    func addWireGuardNetwork(configText: String) async throws -> Int32 {
        try await addNetwork(type: .wireGuard, payload: Data(configText.utf8))
    }

    func addTailnetNetwork(payload: TailnetNetworkPayload) async throws -> Int32 {
        try await addNetwork(type: .tailnet, payload: payload.encoded())
    }

    func discoverTailnet(email: String) async throws -> TailnetDiscoveryResponse {
        let socketURL = try socketURLResult.get()
        return try await TailnetDiscoveryClient.discover(email: email, socketURL: socketURL)
    }

    func probeTailnetAuthority(_ authority: String) async throws -> TailnetAuthorityProbeStatus {
        let socketURL = try socketURLResult.get()
        return try await TailnetAuthorityProbeClient.probe(authority: authority, socketURL: socketURL)
    }

    func startTailnetLogin(
        accountName: String,
        identityName: String,
        hostname: String?,
        authority: String
    ) async throws -> TailnetLoginStatus {
        let socketURL = try socketURLResult.get()
        return try await TailnetLoginClient.start(
            accountName: accountName,
            identityName: identityName,
            hostname: hostname,
            authority: authority,
            socketURL: socketURL
        )
    }

    func tailnetLoginStatus(sessionID: String) async throws -> TailnetLoginStatus {
        let socketURL = try socketURLResult.get()
        return try await TailnetLoginClient.status(sessionID: sessionID, socketURL: socketURL)
    }

    func cancelTailnetLogin(sessionID: String) async throws {
        let socketURL = try socketURLResult.get()
        try await TailnetLoginClient.cancel(sessionID: sessionID, socketURL: socketURL)
    }

    private func addNetwork(type: Burrow_NetworkType, payload: Data) async throws -> Int32 {
        let socketURL = try socketURLResult.get()
        let networkID = nextNetworkID
        let request = Burrow_Network.with {
            $0.id = networkID
            $0.type = type
            $0.payload = payload
        }

        let client = NetworksClient.unix(socketURL: socketURL)
        _ = try await client.networkAdd(request)
        return networkID
    }

    private func startStreaming() {
        task?.cancel()
        let socketURLResult = self.socketURLResult
        task = Task { [weak self] in
            do {
                let socketURL = try socketURLResult.get()
                let client = NetworksClient.unix(socketURL: socketURL)
                for try await response in client.networkList(.init()) {
                    guard !Task.isCancelled else { return }
                    await MainActor.run {
                        guard let self else { return }
                        self.networks = response.network
                        self.connectionError = nil
                    }
                }
            } catch {
                guard !Task.isCancelled else { return }
                await MainActor.run {
                    guard let self else { return }
                    self.connectionError = error.localizedDescription
                }
            }
        }
    }

    private static func makeCard(for network: Burrow_Network) -> NetworkCardModel {
        switch network.type {
        case .wireGuard:
            WireGuardCard(network: network).card
        case .tailnet:
            TailnetCard(network: network).card
        case .UNRECOGNIZED(let rawValue):
            unsupportedCard(
                id: network.id,
                title: "Unknown Network",
                detail: "Type \(rawValue) is not recognized by this build."
            )
        @unknown default:
            unsupportedCard(
                id: network.id,
                title: "Unsupported Network",
                detail: "Update Burrow to view this network."
            )
        }
    }

    private static func unsupportedCard(id: Int32, title: String, detail: String) -> NetworkCardModel {
        NetworkCardModel(
            id: id,
            backgroundColor: .gray.opacity(0.85),
            label: AnyView(
                VStack(alignment: .leading, spacing: 12) {
                    Text(title)
                        .font(.title3.weight(.semibold))
                        .foregroundStyle(.white)
                    Text(detail)
                        .font(.body)
                        .foregroundStyle(.white.opacity(0.9))
                    Spacer()
                    Text("Network #\(id)")
                        .font(.footnote.monospaced())
                        .foregroundStyle(.white.opacity(0.8))
                }
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
            )
        )
    }
}

enum TailnetProvider: String, CaseIterable, Codable, Identifiable, Sendable {
    case tailscale
    case headscale
    case burrow

    var id: String { rawValue }

    var title: String {
        switch self {
        case .tailscale: "Tailscale"
        case .headscale: "Headscale"
        case .burrow: "Burrow"
        }
    }

    var defaultAuthority: String? {
        switch self {
        case .tailscale:
            "https://controlplane.tailscale.com"
        case .headscale:
            "https://ts.burrow.net"
        case .burrow:
            nil
        }
    }

    var subtitle: String {
        switch self {
        case .tailscale:
            "Managed Tailnet authority."
        case .headscale:
            "Custom Tailnet control server."
        case .burrow:
            "Burrow-native Tailnet authority."
        }
    }

    static func inferred(authority: String?, explicit: TailnetProvider?) -> TailnetProvider {
        if explicit == .burrow {
            return .burrow
        }
        if isManagedTailscaleAuthority(authority) {
            return .tailscale
        }
        return .headscale
    }

    static func isManagedTailscaleAuthority(_ authority: String?) -> Bool {
        guard let normalized = authority?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .trimmingCharacters(in: CharacterSet(charactersIn: "/")),
              !normalized.isEmpty
        else {
            return false
        }

        return normalized == "https://controlplane.tailscale.com"
            || normalized == "http://controlplane.tailscale.com"
            || normalized == "controlplane.tailscale.com"
    }
}

enum AccountNetworkKind: String, CaseIterable, Codable, Identifiable, Sendable {
    case wireGuard
    case tor
    case tailnet

    var id: String { rawValue }

    var title: String {
        switch self {
        case .wireGuard: "WireGuard"
        case .tor: "Tor"
        case .tailnet: "Tailnet"
        }
    }

    var subtitle: String {
        switch self {
        case .wireGuard: "Import a tunnel and optional account metadata."
        case .tor: "Store Arti account and identity preferences."
        case .tailnet: "Save Tailnet authority, identity, and login material."
        }
    }

    var accentColor: Color {
        switch self {
        case .wireGuard: .init("WireGuard")
        case .tor: .orange
        case .tailnet: .mint
        }
    }

    var actionTitle: String {
        switch self {
        case .wireGuard: "Add Network"
        case .tor: "Save Account"
        case .tailnet: "Save Account"
        }
    }

    var availabilityNote: String? {
        switch self {
        case .wireGuard:
            nil
        case .tor:
            "Tor account preferences are stored on Apple now. The managed Tor runtime is not wired on Apple in this branch yet."
        case .tailnet:
            "Tailnet accounts can sign in from Apple now. The managed Apple runtime is still pending, but Tailnet networks can be stored in the daemon."
        }
    }
}

enum AccountAuthMode: String, CaseIterable, Codable, Identifiable, Sendable {
    case web
    case none
    case password
    case preauthKey

    var id: String { rawValue }

    var title: String {
        switch self {
        case .web: "Browser Sign-In"
        case .none: "None"
        case .password: "Password"
        case .preauthKey: "Preauth Key"
        }
    }
}

struct NetworkAccountRecord: Codable, Identifiable, Hashable, Sendable {
    let id: UUID
    var kind: AccountNetworkKind
    var title: String
    var authority: String?
    var provider: TailnetProvider?
    var accountName: String
    var identityName: String
    var hostname: String?
    var username: String?
    var tailnet: String?
    var authMode: AccountAuthMode
    var note: String?
    var createdAt: Date
    var updatedAt: Date
}

struct TailnetCard {
    var id: Int32
    var title: String
    var detail: String

    init(network: Burrow_Network) {
        let payload = (try? JSONDecoder().decode(TailnetNetworkPayload.self, from: network.payload))
        id = network.id
        title = payload?.tailnet ?? payload?.hostname ?? "Tailnet"
        detail = [
            payload?.authority.flatMap { URL(string: $0)?.host } ?? payload?.authority,
            payload?.authority,
            payload.map { "Account: \($0.account)" },
        ]
        .compactMap { $0 }
        .joined(separator: " · ")
        .ifEmpty("Stored Tailnet configuration")
    }

    var card: NetworkCardModel {
        NetworkCardModel(
            id: id,
            backgroundColor: .mint,
            label: AnyView(
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Tailnet")
                                .font(.headline)
                                .foregroundStyle(.white.opacity(0.85))
                            Text(title)
                                .font(.title3.weight(.semibold))
                                .foregroundStyle(.white)
                        }
                        Spacer()
                    }
                    Spacer()
                    Text(detail)
                        .font(.body.monospaced())
                        .foregroundStyle(.white.opacity(0.92))
                        .lineLimit(4)
                }
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
            )
        )
    }
}

@Observable
@MainActor
final class NetworkAccountStore {
    private static let storageKey = "burrow.network-accounts"

    private let defaults: UserDefaults
    private(set) var accounts: [NetworkAccountRecord] = []

    init(defaults: UserDefaults = UserDefaults(suiteName: Constants.appGroupIdentifier) ?? .standard) {
        self.defaults = defaults
        load()
    }

    func upsert(_ record: NetworkAccountRecord, secret: String?) throws {
        if let index = accounts.firstIndex(where: { $0.id == record.id }) {
            accounts[index] = record
        } else {
            accounts.append(record)
        }
        accounts.sort {
            if $0.kind == $1.kind {
                return $0.title.localizedCaseInsensitiveCompare($1.title) == .orderedAscending
            }
            return $0.kind.rawValue < $1.kind.rawValue
        }
        try persist()
        if let secret, !secret.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            try AccountSecretStore.store(secret, for: record.id)
        } else {
            try AccountSecretStore.removeSecret(for: record.id)
        }
    }

    func delete(_ record: NetworkAccountRecord) throws {
        accounts.removeAll { $0.id == record.id }
        try persist()
        try AccountSecretStore.removeSecret(for: record.id)
    }

    func hasStoredSecret(for record: NetworkAccountRecord) -> Bool {
        AccountSecretStore.hasSecret(for: record.id)
    }

    private func load() {
        guard let data = defaults.data(forKey: Self.storageKey) else {
            accounts = []
            return
        }

        do {
            accounts = try JSONDecoder().decode([NetworkAccountRecord].self, from: data)
        } catch {
            accounts = []
        }
    }

    private func persist() throws {
        let data = try JSONEncoder().encode(accounts)
        defaults.set(data, forKey: Self.storageKey)
    }
}

private enum AccountSecretStore {
    private static let service = "\(Constants.bundleIdentifier).accounts"

    static func hasSecret(for accountID: UUID) -> Bool {
        let query = baseQuery(for: accountID)
        return SecItemCopyMatching(query as CFDictionary, nil) == errSecSuccess
    }

    static func store(_ secret: String, for accountID: UUID) throws {
        let data = Data(secret.utf8)
        let query = baseQuery(for: accountID)
        let status = SecItemCopyMatching(query as CFDictionary, nil)

        if status == errSecSuccess {
            let updateStatus = SecItemUpdate(
                query as CFDictionary,
                [kSecValueData as String: data] as CFDictionary
            )
            guard updateStatus == errSecSuccess else {
                throw AccountSecretStoreError.osStatus(updateStatus)
            }
            return
        }

        var item = query
        item[kSecValueData as String] = data
        item[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
        let addStatus = SecItemAdd(item as CFDictionary, nil)
        guard addStatus == errSecSuccess else {
            throw AccountSecretStoreError.osStatus(addStatus)
        }
    }

    static func removeSecret(for accountID: UUID) throws {
        let status = SecItemDelete(baseQuery(for: accountID) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw AccountSecretStoreError.osStatus(status)
        }
    }

    private static func baseQuery(for accountID: UUID) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: accountID.uuidString,
        ]
    }
}

private enum AccountSecretStoreError: LocalizedError {
    case osStatus(OSStatus)

    var errorDescription: String? {
        switch self {
        case .osStatus(let status):
            if let message = SecCopyErrorMessageString(status, nil) as String? {
                return message
            }
            return "Keychain error \(status)"
        }
    }
}

private extension String {
    func ifEmpty(_ fallback: @autoclosure () -> String) -> String {
        isEmpty ? fallback() : self
    }
}
