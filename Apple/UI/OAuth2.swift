import AuthenticationServices
import Foundation
import os
import SwiftUI

enum OAuth2 {
    enum Error: Swift.Error {
        case unknown
        case invalidAuthorizationURL
        case invalidCallbackURL
        case invalidRedirectURI
    }

    struct Credential {
        var accessToken: String
        var refreshToken: String?
        var expirationDate: Date?
    }

    struct Session {
        var authorizationEndpoint: URL
        var tokenEndpoint: URL
        var redirectURI: URL
        var responseType = OAuth2.ResponseType.code
        var scopes: Set<String>
        var clientID: String
        var clientSecret: String

        fileprivate static let queue: OSAllocatedUnfairLock<[Int: CheckedContinuation<URL, Swift.Error>]> = {
            .init(initialState: [:])
        }()

        fileprivate static func handle(url: URL) {
            let continuations = queue.withLock { continuations in
                let copy = continuations
                continuations.removeAll()
                return copy
            }
            for (_, continuation) in continuations {
                continuation.resume(returning: url)
            }
        }

        init(
            authorizationEndpoint: URL,
            tokenEndpoint: URL,
            redirectURI: URL,
            scopes: Set<String>,
            clientID: String,
            clientSecret: String
        ) {
            self.authorizationEndpoint = authorizationEndpoint
            self.tokenEndpoint = tokenEndpoint
            self.redirectURI = redirectURI
            self.scopes = scopes
            self.clientID = clientID
            self.clientSecret = clientSecret
        }

        private var authorizationURL: URL {
            get throws {
                var queryItems: [URLQueryItem] = [
                    .init(name: "client_id", value: clientID),
                    .init(name: "response_type", value: responseType.rawValue),
                    .init(name: "redirect_uri", value: redirectURI.absoluteString)
                ]
                if !scopes.isEmpty {
                    queryItems.append(.init(name: "scope", value: scopes.joined(separator: ",")))
                }
                guard var components = URLComponents(url: authorizationEndpoint, resolvingAgainstBaseURL: false) else {
                    throw OAuth2.Error.invalidAuthorizationURL
                }
                components.queryItems = queryItems
                guard let authorizationURL = components.url else { throw OAuth2.Error.invalidAuthorizationURL }
                return authorizationURL
            }
        }

        private func handle(callbackURL: URL) async throws -> OAuth2.AccessTokenResponse {
            switch responseType {
            case .code:
                guard let components = URLComponents(url: callbackURL, resolvingAgainstBaseURL: false) else {
                    throw OAuth2.Error.invalidCallbackURL
                }
                return try await handle(response: try components.decode(OAuth2.CodeResponse.self))
            default:
                throw OAuth2.Error.invalidCallbackURL
            }
        }

        private func handle(response: OAuth2.CodeResponse) async throws -> OAuth2.AccessTokenResponse {
            var components = URLComponents()
            components.queryItems = [
                .init(name: "client_id", value: clientID),
                .init(name: "client_secret", value: clientSecret),
                .init(name: "grant_type", value: GrantType.authorizationCode.rawValue),
                .init(name: "code", value: response.code),
                .init(name: "redirect_uri", value: redirectURI.absoluteString)
            ]
            let httpBody = Data(components.percentEncodedQuery!.utf8)

            var request = URLRequest(url: tokenEndpoint)
            request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
            request.httpMethod = "POST"
            request.httpBody = httpBody

            let session = URLSession(configuration: .ephemeral)
            let (data, _) = try await session.data(for: request)
            return try OAuth2.decoder.decode(OAuth2.AccessTokenResponse.self, from: data)
        }

        func authorize(_ session: WebAuthenticationSession) async throws -> Credential {
            let authorizationURL = try authorizationURL
            let callbackURL = try await session.start(
                url: authorizationURL,
                redirectURI: redirectURI
            )
            return try await handle(callbackURL: callbackURL).credential
        }
    }

    private struct CodeResponse: Codable {
        var code: String
        var state: String?
    }

    private struct AccessTokenResponse: Codable {
        var accessToken: String
        var tokenType: TokenType
        var expiresIn: Double?
        var refreshToken: String?

        var credential: Credential {
            .init(
                accessToken: accessToken,
                refreshToken: refreshToken,
                expirationDate: expiresIn.map { Date(timeIntervalSinceNow: $0) }
            )
        }
    }

    enum TokenType: Codable, RawRepresentable {
        case bearer
        case unknown(String)

        init(rawValue: String) {
            self = switch rawValue.lowercased() {
            case "bearer": .bearer
            default: .unknown(rawValue)
            }
        }

        var rawValue: String {
            switch self {
            case .bearer: "bearer"
            case .unknown(let type): type
            }
        }
    }

    enum GrantType: Codable, RawRepresentable {
        case authorizationCode
        case unknown(String)

        init(rawValue: String) {
            self = switch rawValue.lowercased() {
            case "authorization_code": .authorizationCode
            default: .unknown(rawValue)
            }
        }

        var rawValue: String {
            switch self {
            case .authorizationCode: "authorization_code"
            case .unknown(let type): type
            }
        }
    }

    enum ResponseType: Codable, RawRepresentable {
        case code
        case idToken
        case unknown(String)

        init(rawValue: String) {
            self = switch rawValue.lowercased() {
            case "code": .code
            case "id_token": .idToken
            default: .unknown(rawValue)
            }
        }

        var rawValue: String {
            switch self {
            case .code: "code"
            case .idToken: "id_token"
            case .unknown(let type): type
            }
        }
    }

    fileprivate static var decoder: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return decoder
    }

    fileprivate static var encoder: JSONEncoder {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        return encoder
    }
}

extension WebAuthenticationSession: @unchecked @retroactive Sendable {
}

extension WebAuthenticationSession {
#if canImport(BrowserEngineKit)
    @available(iOS 17.4, macOS 14.4, tvOS 17.4, watchOS 10.4, *)
    fileprivate static func callback(for redirectURI: URL) throws -> ASWebAuthenticationSession.Callback {
        switch redirectURI.scheme {
        case "https":
            guard let host = redirectURI.host else { throw OAuth2.Error.invalidRedirectURI }
            return .https(host: host, path: redirectURI.path)
        case "http":
            throw OAuth2.Error.invalidRedirectURI
        case .some(let scheme):
            return .customScheme(scheme)
        case .none:
            throw OAuth2.Error.invalidRedirectURI
        }
    }
#endif

    fileprivate func start(url: URL, redirectURI: URL) async throws -> URL {
        #if canImport(BrowserEngineKit)
        if #available(iOS 17.4, macOS 14.4, tvOS 17.4, watchOS 10.4, *) {
            return try await authenticate(
                using: url,
                callback: try Self.callback(for: redirectURI),
                additionalHeaderFields: [:]
            )
        }
        #endif

        return try await withThrowingTaskGroup(of: URL.self) { group in
            group.addTask {
                return try await authenticate(using: url, callbackURLScheme: redirectURI.scheme ?? "")
            }

            let id = Int.random(in: 0..<Int.max)
            group.addTask {
                return try await withCheckedThrowingContinuation { continuation in
                    OAuth2.Session.queue.withLock { $0[id] = continuation }
                }
            }
            guard let url = try await group.next() else { throw OAuth2.Error.invalidCallbackURL }
            group.cancelAll()
            OAuth2.Session.queue.withLock { $0[id] = nil }
            return url
        }
    }
}

extension View {
    func handleOAuth2Callback() -> some View {
        onOpenURL { url in OAuth2.Session.handle(url: url) }
    }
}

extension URLComponents {
    fileprivate func decode<T: Decodable>(_ type: T.Type) throws -> T {
        guard let queryItems else {
            throw DecodingError.valueNotFound(
                T.self,
                .init(codingPath: [], debugDescription: "Missing query items")
            )
        }
        let data = try OAuth2.encoder.encode(try queryItems.values)
        return try OAuth2.decoder.decode(T.self, from: data)
    }
}

extension Sequence where Element == URLQueryItem {
    fileprivate var values: [String: String?] {
        get throws {
            try Dictionary(map { ($0.name, $0.value) }) { _, _ in
                throw DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: "Duplicate query items"))
            }
        }
    }
}
