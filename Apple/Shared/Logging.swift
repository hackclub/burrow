import os
@_exported import OSLog

extension Logger {
    private static let loggers: OSAllocatedUnfairLock<[String: Logger]> = OSAllocatedUnfairLock(initialState: [:])

    public static let subsystem = Constants.bundleIdentifier

    public static func logger(for type: Any.Type) -> Logger {
        let category = String(describing: type)
        let logger = loggers.withLock { loggers in
            if let logger = loggers[category] { return logger }
            let logger = Logger(subsystem: subsystem, category: category)
            loggers[category] = logger
            return logger
        }
        return logger
    }
}
