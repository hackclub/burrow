import Foundation

// swiftlint:disable identifier_name
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


public struct EmptyParams: Codable {}

public struct BurrowSingleCommand: Request {
    public var id: UInt
    public var method: String
    public var params: EmptyParams?
    
    public init(id: UInt, command: String) {
        self.id = id
        self.method = command
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

public struct Notification<T>: Codable where T: Codable {
    public var method: String
    public var params: T
    public init(method: String, params: T) {
        self.method = method
        self.params = params
    }
}

public struct NotificationPeek : Codable {
    public var method: String
    public init(method: String) {
        self.method = method
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

public struct ServerConfigData: Codable {
    public struct InternalConfig: Codable {
        public let address: String?
        public let name: String?
        public let mtu: Int32?
    }
    public let ServerConfig: InternalConfig
    public init(ServerConfig: InternalConfig) {
        self.ServerConfig = ServerConfig
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

// swiftlint:enable identifier_name
