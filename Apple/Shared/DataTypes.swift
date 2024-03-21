import Foundation

// swiftlint:disable identifier_name
public enum BurrowError: Error {
    case addrDoesntExist
    case resultIsError
    case cantParseResult
    case resultIsNone
    case noClient
}

public protocol Request: Codable where Command: Codable {
    associatedtype Command

    var id: UInt { get set }
    var command: Command { get set }
}

public struct BurrowSingleCommand: Request {
    public var id: UInt
    public var command: String
    public init(id: UInt, command: String) {
        self.id = id
        self.command = command
    }
}

public struct BurrowRequest<T>: Request where T: Codable {
    public var id: UInt
    public var command: T
    public init(id: UInt, command: T) {
        self.id = id
        self.command = command
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

public struct AnyResponse: Codable {
    public var id: UInt
    public init(id: UInt) {
        self.id = id
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

public struct BurrowStartRequest: Codable {
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
    public struct StartOptions: Codable {
        public let tun: TunOptions
        public init(tun: TunOptions) {
            self.tun = tun
        }
    }
    public let Start: StartOptions
    public init(Start: StartOptions) {
        self.Start = Start
    }
}

// swiftlint:enable identifier_name
