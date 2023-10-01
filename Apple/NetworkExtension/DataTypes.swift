import Foundation

enum BurrowError: Error {
    case addrDoesntExist
    case resultIsError
    case cantParseResult
    case resultIsNone
}

protocol Request: Codable {
    var id: UInt { get set }
    var command: String { get set }
}

struct BurrowRequest: Request {
    var id: UInt
    var command: String
}

struct Response<T>: Decodable where T: Decodable {
    var id: UInt
    var result: T
}

// swiftlint:disable identifier_name
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
