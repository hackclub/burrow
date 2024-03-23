import Foundation

// swiftlint:disable identifier_name raw_value_for_camel_cased_codable_enum
public enum BurrowError: Error {
    case addrDoesntExist
    case resultIsError
    case cantParseResult
    case resultIsNone
    case noClient
}

public protocol Request: Codable where Params: Codable {
    associatedtype Params

    var id: UInt { get set }
    var method: String { get set }
    var params: Params? { get set }
}

public enum MessageType: String, Codable {
    case Request
    case Response
    case Notification
}

public struct MessagePeek: Codable {
    public var type: MessageType
    public init(type: MessageType) {
        self.type = type
    }
}

public struct BurrowSimpleRequest: Request {
    public var id: UInt
    public var method: String
    public var params: String?
    public init(id: UInt, command: String, params: String? = nil) {
        self.id = id
        self.method = command
        self.params = params
    }
}

public struct BurrowRequest<T>: Request where T: Codable {
    public var id: UInt
    public var method: String
    public var params: T?
    public init(id: UInt, command: T) {
        self.id = id
        self.method = "\(T.self)"
        self.params = command
    }
}

public struct Response<T>: Decodable where T: Decodable {
    public var id: UInt
    public var result: T
    public init(id: UInt, result: T) {
        self.id = id
        self.result = result
    }
}

public struct ResponsePeek: Codable {
    public var id: UInt
    public init(id: UInt) {
        self.id = id
    }
}

public enum NotificationType: String, Codable {
    case ConfigChange
}

public struct Notification<T>: Codable where T: Codable {
    public var method: NotificationType
    public var params: T
    public init(method: NotificationType, params: T) {
        self.method = method
        self.params = params
    }
}

public struct NotificationPeek: Codable {
    public var method: NotificationType
    public init(method: NotificationType) {
        self.method = method
    }
}

public struct AnyResponseData: Codable {
    public var type: String
    public init(type: String) {
        self.type = type
    }
}

public struct BurrowResult<T>: Codable where T: Codable {
    public var Ok: T?
    public var Err: String?
    public init(Ok: T, Err: String? = nil) {
        self.Ok = Ok
        self.Err = Err
    }
}

public struct ServerConfig: Codable {
    public let address: String?
    public let name: String?
    public let mtu: Int32?
    public init(address: String?, name: String?, mtu: Int32?) {
        self.address = address
        self.name = name
        self.mtu = mtu
    }
}

public struct Start: Codable {
    public struct TunOptions: Codable {
        public let name: String?
        public let no_pi: Bool
        public let tun_excl: Bool
        public let tun_retrieve: Bool
        public let address: String?
        public init(name: String?, no_pi: Bool, tun_excl: Bool, tun_retrieve: Bool, address: String?) {
            self.name = name
            self.no_pi = no_pi
            self.tun_excl = tun_excl
            self.tun_retrieve = tun_retrieve
            self.address = address
        }
    }
    public let tun: TunOptions
    public init(tun: TunOptions) {
        self.tun = tun
    }
}

// swiftlint:enable identifier_name raw_value_for_camel_cased_codable_enum
