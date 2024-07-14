@_implementationOnly import CConstants
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
        case .none: .failure(.invalidAppGroupIdentifier)
        }
    }()
}

extension Logger {
    @_dynamicReplacement(for: subsystem)
    public static var subsystem: String { Constants.bundleIdentifier }
}
