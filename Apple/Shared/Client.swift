import Foundation
import Network

public final class Client {
    let connection: NWConnection

    private let logger = Logger.logger(for: Client.self)
    private var generator = SystemRandomNumberGenerator()
    private var continuations: [UInt: UnsafeContinuation<Data, Error>] = [:]
    private var event_map: [String : [(Data) throws -> Void]] = [:]
    private var task: Task<Void, Error>?

    public convenience init() throws {
        self.init(url: try Constants.socketURL)
    }

    public init(url: URL) {
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
        let connection = NWConnection(to: endpoint, using: parameters)
        connection.start(queue: .global())
        self.connection = connection
        self.task = Task { [weak self] in
            while true {
                let (data, _, _) = try await connection.receiveMessage()
                self?.logger.debug("Received message! \(data)")
                let response = try JSONDecoder().decode(AnyResponse.self, from: data)
                self?.logger.info("Received response for \(response.id)")
                guard let continuations = self?.continuations else {return}
                self?.logger.debug("All keys in continuation table: \(continuations.keys)")
                guard let continuation = self?.continuations[response.id] else { return }
                self?.logger.debug("Got matching continuation")
                continuation.resume(returning: data)
            }
        }
    }
    private func send<T: Request, U: Decodable>(_ request: T) async throws -> U {
        let data: Data = try await withUnsafeThrowingContinuation { continuation in
            continuations[request.id] = continuation
            do {
                let data = try JSONEncoder().encode(request)
                let completion: NWConnection.SendCompletion = .contentProcessed { error in
                    guard let error = error else {
                        return
                    }
                    continuation.resume(throwing: error)
                }
                connection.send(content: data, completion: completion)
            } catch {
                continuation.resume(throwing: error)
                return
            }
        }
        self.logger.debug("Got response data: \(String(describing: data.base64EncodedString()))")
        let res = try JSONDecoder().decode(Response<U>.self, from: data)
        self.logger.debug("Got response data decoded: \(String(describing: res))")
        return res.result
    }
    public func request<T: Codable, U: Decodable>(_ request: T, type: U.Type = U.self) async throws -> U {
        let req = BurrowRequest(
            id: generator.next(upperBound: UInt.max),
            command: request
        )
        return try await send(req)
    }
    public func on_event<T: Codable>(event_name: String, callable: @escaping (T) throws -> Void){
        let action = { data in
            let decoded = try JSONDecoder().decode(T.self, from: data)
            try callable(decoded)
        }
        if event_map[event_name] != nil{
            event_map[event_name]?.append(action)
        }else{
            event_map[event_name] = [action]
        }
    }

    deinit {
        connection.cancel()
    }
}
