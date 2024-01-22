import Foundation

// swiftlint:disable identifier_name
enum BurrowError: Error {
    case addrDoesntExist
    case resultIsError
    case cantParseResult
    case resultIsNone
}

protocol Request: Codable where Command: Codable {
    associatedtype Command

    var id: UInt { get set }
    var command: Command { get set }
}

struct BurrowSingleCommand: Request {
    var id: UInt
    var command: String
}

struct BurrowRequest<T>: Request where T: Codable {
    var id: UInt
    var command: T
}

struct BurrowStartRequest: Codable {
    struct TunOptions: Codable {
        let name: String?
        let no_pi: Bool
        let tun_excl: Bool
        let tun_retrieve: Bool
        let address: String?
    }
    struct StartOptions: Codable {
        let tun: TunOptions
    }
    let Start: StartOptions
}

struct Response<T>: Decodable where T: Decodable {
    var id: UInt
    var result: T
}

struct BurrowResult<T>: Codable where T: Codable {
    var Ok: T?
    var Err: String?
}

struct ServerConfigData: Codable {
    struct InternalConfig: Codable {
        let address: String?
        let name: String?
        let mtu: Int32?
    }
    let ServerConfig: InternalConfig
}

// swiftlint:enable identifier_name
