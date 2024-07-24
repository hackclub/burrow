@_implementationOnly import CConstants
import OSLog

public enum Constants {
    enum Error: Swift.Error {
        case invalidAppGroupIdentifier
    }

    public static let bundleIdentifier = AppBundleIdentifier
    public static let appGroupIdentifier = AppGroupIdentifier
    public static let networkExtensionBundleIdentifier = NetworkExtensionBundleIdentifier

    public static var groupContainerURL: URL {
        get throws { try _groupContainerURL.get() }
    }
    public static var socketURL: URL {
        get throws {
            try groupContainerURL.appending(component: "burrow.sock", directoryHint: .notDirectory)
        }
    }
    public static var dbURL: URL {
        get throws {
            try groupContainerURL.appending(component: "burrow.db", directoryHint: .notDirectory)
        }
    }

    private static let _groupContainerURL: Result<URL, Error> = {
        guard let groupContainerURL = FileManager.default
            .containerURL(forSecurityApplicationGroupIdentifier: appGroupIdentifier) else {
            return .failure(.invalidAppGroupIdentifier)
        }
        return .success(groupContainerURL)
    }()
}

extension Logger {
    @_dynamicReplacement(for: subsystem)
    public static var subsystem: String { Constants.bundleIdentifier }
}
