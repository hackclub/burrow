@_implementationOnly import CConstants
import Foundation
import OSLog

public enum Constants {
    enum Error: Swift.Error {
        case invalidAppGroupIdentifier
    }

    public static let bundleIdentifier = AppBundleIdentifier
    public static let appGroupIdentifier = AppGroupIdentifier
    public static let networkExtensionBundleIdentifier = NetworkExtensionBundleIdentifier

    public static var socketURL: URL {
        get throws {
            try groupContainerURL.appending(component: "burrow.sock", directoryHint: .notDirectory)
        }
    }
    public static var databaseURL: URL {
        get throws {
            try groupContainerURL.appending(component: "burrow.db", directoryHint: .notDirectory)
        }
    }

    private static var groupContainerURL: URL {
        get throws { try _groupContainerURL.get() }
    }
    private static let _groupContainerURL: Result<URL, Error> = {
        switch FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupIdentifier) {
        case .some(let url): .success(url)
        case .none:
            fallbackContainerURL().mapError { _ in .invalidAppGroupIdentifier }
        }
    }()

    private static func fallbackContainerURL() -> Result<URL, any Swift.Error> {
#if targetEnvironment(simulator)
        Result {
            // The simulator app's Application Support path lives inside its sandbox container,
            // so the host daemon cannot reach it. Use a shared host temp location instead.
            let url = URL(filePath: "/tmp", directoryHint: .isDirectory)
                .appending(component: bundleIdentifier, directoryHint: .isDirectory)
                .appending(component: "SimulatorFallback", directoryHint: .isDirectory)
            try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
            return url
        }
#else
        .failure(Error.invalidAppGroupIdentifier)
#endif
    }
}

extension Logger {
    @_dynamicReplacement(for: subsystem)
    public static var subsystem: String { Constants.bundleIdentifier }
}
