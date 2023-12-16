import Foundation

// swiftlint:disable identifier_name
enum BurrowError: Error {
    case addrDoesntExist
    case resultIsError
    case cantParseResult
    case resultIsNone
}

protocol Request: Codable where CommandT: Codable {
    associatedtype CommandT
    var id: UInt { get set }
    var command: CommandT { get set }
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

func start_req_fd(id: UInt) -> BurrowRequest<BurrowStartRequest> {
    let command = BurrowStartRequest(Start: BurrowStartRequest.StartOptions(
        tun: BurrowStartRequest.TunOptions(name: nil, no_pi: false, tun_excl: false, tun_retrieve: true, address: nil)
    ))
    return BurrowRequest(id: id, command: command)
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
