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

struct TailnetLoginStartRequest: Codable, Sendable {
    var accountName: String
    var identityName: String
    var hostname: String?
    var controlURL: String?
}

struct TailnetLoginStatus: Codable, Sendable {
    var backendState: String
    var authURL: String?
    var running: Bool
    var needsLogin: Bool
    var tailnetName: String?
    var magicDNSSuffix: String?
    var selfDNSName: String?
    var tailscaleIPs: [String]
    var health: [String]
}

struct TailnetLoginStartResponse: Codable, Sendable {
    var sessionID: String
    var status: TailnetLoginStatus
}

enum TailnetBridgeClient {
    private static let baseURL = URL(string: "http://127.0.0.1:8080")!

    static func startLogin(_ request: TailnetLoginStartRequest) async throws -> TailnetLoginStartResponse {
        var urlRequest = URLRequest(
            url: baseURL.appendingPathComponent("v1/tailscale/login/start")
        )
        urlRequest.httpMethod = "POST"
        urlRequest.setValue("application/json", forHTTPHeaderField: "Content-Type")

        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        urlRequest.httpBody = try encoder.encode(request)

        let (data, response) = try await URLSession.shared.data(for: urlRequest)
        try validate(response: response, data: data)

        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode(TailnetLoginStartResponse.self, from: data)
    }

    static func status(sessionID: String) async throws -> TailnetLoginStatus {
        let url = baseURL
            .appendingPathComponent("v1/tailscale/login")
            .appendingPathComponent(sessionID)
        let (data, response) = try await URLSession.shared.data(from: url)
        try validate(response: response, data: data)

        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode(TailnetLoginStatus.self, from: data)
    }

    private static func validate(response: URLResponse, data: Data) throws {
        guard let http = response as? HTTPURLResponse else {
            throw URLError(.badServerResponse)
        }
        guard (200..<300).contains(http.statusCode) else {
            let message = String(data: data, encoding: .utf8)?.trimmingCharacters(
                in: .whitespacesAndNewlines
            )
            throw TailnetBridgeError.server(message?.ifEmpty("HTTP \(http.statusCode)") ?? "HTTP \(http.statusCode)")
        }
    }
}

enum TailnetBridgeError: LocalizedError {
    case server(String)

    var errorDescription: String? {
        switch self {
        case .server(let message):
            message
        }
    }
}

@Observable
@MainActor
final class NetworkViewModel: Sendable {
    private(set) var networks: [Burrow_Network] = []
    private(set) var connectionError: String?
    private let socketURLResult: Result<URL, Error>

    nonisolated(unsafe) private var task: Task<Void, Never>?

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

    var usesWebLogin: Bool {
        self == .tailscale
    }

    var requiresControlURL: Bool {
        self != .tailscale
    }

    var defaultAuthority: String? {
        switch self {
        case .tailscale:
            "https://controlplane.tailscale.com"
        case .headscale, .burrow:
            nil
        }
    }

    var subtitle: String {
        switch self {
        case .tailscale:
            "Use Tailscale's real browser login flow."
        case .headscale:
            "Store a Headscale control-plane endpoint and credentials."
        case .burrow:
            "Store Burrow control-plane credentials."
        }
    }
}

enum AccountNetworkKind: String, CaseIterable, Codable, Identifiable, Sendable {
    case wireGuard
    case tor
    case headscale

    var id: String { rawValue }

    var title: String {
        switch self {
        case .wireGuard: "WireGuard"
        case .tor: "Tor"
        case .headscale: "Tailnet"
        }
    }

    var subtitle: String {
        switch self {
        case .wireGuard: "Import a tunnel and optional account metadata."
        case .tor: "Store Arti account and identity preferences."
        case .headscale: "Save Tailscale, Headscale, or Burrow control-plane identities."
        }
    }

    var accentColor: Color {
        switch self {
        case .wireGuard: .init("WireGuard")
        case .tor: .orange
        case .headscale: .mint
        }
    }

    var actionTitle: String {
        switch self {
        case .wireGuard: "Add Network"
        case .tor: "Save Account"
        case .headscale: "Save Account"
        }
    }

    var availabilityNote: String? {
        switch self {
        case .wireGuard:
            nil
        case .tor:
            "Tor account preferences are stored on Apple now. The managed Tor runtime is not wired on Apple in this branch yet."
        case .headscale:
            "Tailnet accounts can sign in from Apple now. The managed Apple runtime is still pending, but Tailnet networks can be stored in the daemon."
        }
    }
}

enum AccountAuthMode: String, CaseIterable, Codable, Identifiable, Sendable {
    case none
    case web
    case password
    case preauthKey

    var id: String { rawValue }

    var title: String {
        switch self {
        case .none: "None"
        case .web: "Web Login"
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
    var provider: String
    var title: String
    var detail: String

    init(network: Burrow_Network) {
        let payload = (try? JSONDecoder().decode(TailnetNetworkPayload.self, from: network.payload))
        id = network.id
        provider = payload?.provider.title ?? "Tailnet"
        title = payload?.tailnet ?? payload?.hostname ?? "Tailnet"
        detail = [
            payload?.provider.title,
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
                            Text(provider)
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
