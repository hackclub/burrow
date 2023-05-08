import NetworkExtension

extension NEVPNManager {
    func remove() async throws {
        let _: Void = try await withUnsafeThrowingContinuation { continuation in
            removeFromPreferences(completionHandler: completion(continuation))
        }
    }

    func save() async throws {
        let _: Void = try await withUnsafeThrowingContinuation { continuation in
            saveToPreferences(completionHandler: completion(continuation))
        }
    }
}

extension NETunnelProviderManager {
    class var managers: [NETunnelProviderManager] {
        get async throws {
            try await withUnsafeThrowingContinuation { continuation in
                loadAllFromPreferences { managers, error in
                    if let error = error {
                        continuation.resume(throwing: error)
                    } else {
                        continuation.resume(returning: managers ?? [])
                    }
                }
            }
        }
    }
}

private func completion(_ continuation: UnsafeContinuation<Void, Error>) -> (Error?) -> Void {
    return { error in
        if let error = error {
            continuation.resume(throwing: error)
        } else {
            continuation.resume(returning: ())
        }
    }
}
