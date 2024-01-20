import NetworkExtension

extension NEVPNManager {
    func remove() async throws {
        _ = try await withUnsafeThrowingContinuation { continuation in
            removeFromPreferences(completionHandler: completion(continuation))
        }
    }

    func save() async throws {
        _ = try await withUnsafeThrowingContinuation { continuation in
            saveToPreferences(completionHandler: completion(continuation))
        }
    }
}

extension NETunnelProviderManager {
    class var managers: [NETunnelProviderManager] {
        get async throws {
            try await withUnsafeThrowingContinuation { continuation in
                loadAllFromPreferences(completionHandler: completion(continuation))
            }
        }
    }
}

private func completion(_ continuation: UnsafeContinuation<Void, Error>) -> (Error?) -> Void {
    return { error in
        if let error {
            continuation.resume(throwing: error)
        } else {
            continuation.resume(returning: ())
        }
    }
}

private func completion<T>(_ continuation: UnsafeContinuation<T, Error>) -> (T?, Error?) -> Void {
    return { value, error in
        if let error {
            continuation.resume(throwing: error)
        } else if let value {
            continuation.resume(returning: value)
        }
    }
}
