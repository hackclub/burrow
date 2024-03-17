import BurrowShared
import Foundation
import Network

final class Client {
    let connection: NWConnection

    private let logger = Logger.logger(for: Client.self)
    private var generator = SystemRandomNumberGenerator()

    convenience init() throws {
        self.init(url: try Constants.socketURL)
    }

    init(url: URL) {
        let endpoint: NWEndpoint
        if url.isFileURL {
            endpoint = .unix(path: url.path(percentEncoded: false))
        } else {
            endpoint = .url(url)
        }

        let parameters = NWParameters.tcp
        parameters.defaultProtocolStack
            .applicationProtocols
            .insert(NWProtocolFramer.Options(definition: NewlineProtocolFramer.definition), at: 0)
        connection = NWConnection(to: endpoint, using: parameters)
        connection.start(queue: .global())
    }

    func request<U: Decodable>(_ request: any Request, type: U.Type = U.self) async throws -> U {
        do {
            var copy = request
            copy.id = generator.next(upperBound: UInt.max)
            let content = try JSONEncoder().encode(copy)
            logger.debug("> \(String(decoding: content, as: UTF8.self))")

            try await self.connection.send(content: content)
            let (response, _, _) = try await connection.receiveMessage()

            logger.debug("< \(String(decoding: response, as: UTF8.self))")
            return try JSONDecoder().decode(U.self, from: response)
        } catch {
            logger.error("\(error, privacy: .public)")
            throw error
        }
    }

    deinit {
        connection.cancel()
    }
}

extension Constants {
    static var socketURL: URL {
        get throws {
            try groupContainerURL.appending(component: "burrow.sock", directoryHint: .notDirectory)
        }
    }
}
