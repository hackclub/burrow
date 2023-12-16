import Foundation
import Network
import os

final class LineProtocol: NWProtocolFramerImplementation {
    static let definition = NWProtocolFramer.Definition(implementation: LineProtocol.self)
    static let label = "Lines"
    init(framer: NWProtocolFramer.Instance) { }
    func start(framer: NWProtocolFramer.Instance) -> NWProtocolFramer.StartResult { .ready }
    func stop(framer: NWProtocolFramer.Instance) -> Bool { true }
    func wakeup(framer: NWProtocolFramer.Instance) { }
    func cleanup(framer: NWProtocolFramer.Instance) { }
    func lines(from buffer: UnsafeMutableRawBufferPointer?) -> (lines: [Data], size: Int)? {
        guard let buffer = buffer else { return nil }
        let lines = buffer
            .split(separator: 10)
        guard !lines.isEmpty else { return nil }
        let size = lines
            .lazy
            .map(\.count)
            .reduce(0, +) + lines.count
        let strings = lines
            .lazy
            .map { Data($0) }
        return (lines: Array(strings), size: size)
    }
    func handleInput(framer: NWProtocolFramer.Instance) -> Int {
        var result: [Data] = []
        _ = framer.parseInput(minimumIncompleteLength: 1, maximumLength: 16_000) { buffer, _ in
            guard let (lines, size) = lines(from: buffer) else {
                return 0
            }
            result = lines
            return size
        }
        for line in result {
            framer.deliverInput(data: line, message: .init(instance: framer), isComplete: true)
        }
        return 0
    }
    func handleOutput(
        framer: NWProtocolFramer.Instance,
        message: NWProtocolFramer.Message,
        messageLength: Int,
        isComplete: Bool
    ) {
        do {
            try framer.writeOutputNoCopy(length: messageLength)
        } catch {
        }
    }
}

extension NWConnection {
    func receiveMessage() async throws -> (Data?, NWConnection.ContentContext?, Bool) {
        try await withUnsafeThrowingContinuation { continuation in
            receiveMessage { completeContent, contentContext, isComplete, error in
                if let error = error {
                    continuation.resume(throwing: error)
                }
                continuation.resume(returning: (completeContent, contentContext, isComplete))
            }
        }
    }
    func send_raw(_ request: Data) async throws -> Data {
        try await withCheckedThrowingContinuation { continuation in
            let comp: NWConnection.SendCompletion = .contentProcessed {error in
                if let error = error {
                    continuation.resume(with: .failure(error))
                } else {
                    continuation.resume(with: .success(request))
                }
            }
            self.send(content: request, completion: comp)
        }
    }
}

final class BurrowIpc {
    let connection: NWConnection
    private var generator = SystemRandomNumberGenerator()
    private var logger: Logger
    init(logger: Logger) {
        let params = NWParameters.tcp
        params.defaultProtocolStack
            .applicationProtocols
            .insert(NWProtocolFramer.Options(definition: LineProtocol.definition), at: 0)
        let connection = NWConnection(to: .unix(path: "burrow.sock"), using: params)
        connection.start(queue: .global())
        self.connection = connection
        self.logger = logger
    }
    func send<T: Request, U: Decodable>(_ request: T) async throws -> U {
        do {
            let id: UInt = generator.next(upperBound: UInt.max)
            var copy = request
            copy.id = id
            var data = try JSONEncoder().encode(request)
            data.append(contentsOf: [10])
            _ = try await self.connection.send_raw(data)
            return try JSONDecoder().decode(Response<U>.self, from: data).result
        } catch {
            throw error
        }
    }

    func receive_raw() async throws -> Data {
        let (completeContent, _, _) = try await connection.receiveMessage()
        self.logger.info("Received raw message response")
        guard let data = completeContent else {
            throw BurrowError.resultIsNone
        }
        return data
    }

    func request<U: Decodable>(_ request: any Request, type: U.Type) async throws -> U {
        do {
            var data: Data = try JSONEncoder().encode(request)
            data.append(contentsOf: [10])
            _ = try await self.connection.send_raw(data)
            self.logger.debug("message sent")
            let receivedData = try await receive_raw()
            self.logger.info("Received result: \(String(decoding: receivedData, as: UTF8.self))")
            return try self.parse_response(receivedData)
        } catch {
            throw error
        }
    }

    func parse_response<U: Decodable>(_ response: Data) throws -> U {
        try JSONDecoder().decode(U.self, from: response)
    }
}
