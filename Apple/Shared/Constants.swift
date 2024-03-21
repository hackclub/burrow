@_implementationOnly import Constants

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

    private static let _groupContainerURL: Result<URL, Error> = {
        guard let groupContainerURL = FileManager.default
            .containerURL(forSecurityApplicationGroupIdentifier: appGroupIdentifier) else {
            return .failure(.invalidAppGroupIdentifier)
        }
        return .success(groupContainerURL)
    }()
    public static var socketURL: URL {
        get throws {
            try groupContainerURL.appending(component: "burrow.sock", directoryHint: .notDirectory)
        }
    }
}
