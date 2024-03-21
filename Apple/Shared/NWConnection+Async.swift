import Foundation
import Network

extension NWConnection {
    // swiftlint:disable:next large_tuple
    func receiveMessage() async throws -> (Data, NWConnection.ContentContext?, Bool) {
        try await withUnsafeThrowingContinuation { continuation in
            receiveMessage { completeContent, contentContext, isComplete, error in
                if let error {
                    continuation.resume(throwing: error)
                } else {
                    guard let completeContent = completeContent else {
                        fatalError("Both error and completeContent were nil")
                    }
                    continuation.resume(returning: (completeContent, contentContext, isComplete))
                }
            }
        }
    }

    func send(content: Data) async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            send(content: content, completion: .contentProcessed { error in
                if let error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume(returning: ())
                }
            })
        }
    }
}
