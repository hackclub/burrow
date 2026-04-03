import Foundation
import GRPC
import NIOTransportServices
import SwiftProtobuf

public typealias TunnelClient = Burrow_TunnelAsyncClient
public typealias NetworksClient = Burrow_NetworksAsyncClient

public protocol Client {
    init(channel: GRPCChannel)
}

extension Client {
    public static func unix(socketURL: URL) -> Self {
        let group = NIOTSEventLoopGroup()
        let configuration = ClientConnection.Configuration.default(
            target: .unixDomainSocket(socketURL.path),
            eventLoopGroup: group
        )
        return Self(channel: ClientConnection(configuration: configuration))
    }
}

extension TunnelClient: Client {
    public init(channel: any GRPCChannel) {
        self.init(channel: channel, defaultCallOptions: .init(), interceptors: .none)
    }
}

extension NetworksClient: Client {
    public init(channel: any GRPCChannel) {
        self.init(channel: channel, defaultCallOptions: .init(), interceptors: .none)
    }
}

public struct Burrow_TailnetDiscoverRequest: Sendable {
    public var email: String = ""
    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
}

public struct Burrow_TailnetDiscoverResponse: Sendable {
    public var domain: String = ""
    public var authority: String = ""
    public var oidcIssuer: String = ""
    public var managed: Bool = false
    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
}

public struct Burrow_TailnetProbeRequest: Sendable {
    public var authority: String = ""
    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
}

public struct Burrow_TailnetProbeResponse: Sendable {
    public var authority: String = ""
    public var statusCode: Int32 = 0
    public var summary: String = ""
    public var detail: String = ""
    public var reachable: Bool = false
    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
}

extension Burrow_TailnetDiscoverRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TailnetDiscoverRequest"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .same(proto: "email")
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularStringField(value: &self.email)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.email.isEmpty {
            try visitor.visitSingularStringField(value: self.email, fieldNumber: 1)
        }
        try unknownFields.traverse(visitor: &visitor)
    }
}

extension Burrow_TailnetDiscoverResponse: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TailnetDiscoverResponse"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .same(proto: "domain"),
        2: .same(proto: "authority"),
        3: .same(proto: "oidc_issuer"),
        4: .same(proto: "managed"),
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularStringField(value: &self.domain)
            case 2: try decoder.decodeSingularStringField(value: &self.authority)
            case 3: try decoder.decodeSingularStringField(value: &self.oidcIssuer)
            case 4: try decoder.decodeSingularBoolField(value: &self.managed)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.domain.isEmpty {
            try visitor.visitSingularStringField(value: self.domain, fieldNumber: 1)
        }
        if !self.authority.isEmpty {
            try visitor.visitSingularStringField(value: self.authority, fieldNumber: 2)
        }
        if !self.oidcIssuer.isEmpty {
            try visitor.visitSingularStringField(value: self.oidcIssuer, fieldNumber: 3)
        }
        if self.managed {
            try visitor.visitSingularBoolField(value: self.managed, fieldNumber: 4)
        }
        try unknownFields.traverse(visitor: &visitor)
    }
}

extension Burrow_TailnetProbeRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TailnetProbeRequest"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .same(proto: "authority")
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularStringField(value: &self.authority)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.authority.isEmpty {
            try visitor.visitSingularStringField(value: self.authority, fieldNumber: 1)
        }
        try unknownFields.traverse(visitor: &visitor)
    }
}

extension Burrow_TailnetProbeResponse: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TailnetProbeResponse"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .same(proto: "authority"),
        2: .same(proto: "status_code"),
        3: .same(proto: "summary"),
        4: .same(proto: "detail"),
        5: .same(proto: "reachable"),
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularStringField(value: &self.authority)
            case 2: try decoder.decodeSingularInt32Field(value: &self.statusCode)
            case 3: try decoder.decodeSingularStringField(value: &self.summary)
            case 4: try decoder.decodeSingularStringField(value: &self.detail)
            case 5: try decoder.decodeSingularBoolField(value: &self.reachable)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.authority.isEmpty {
            try visitor.visitSingularStringField(value: self.authority, fieldNumber: 1)
        }
        if self.statusCode != 0 {
            try visitor.visitSingularInt32Field(value: self.statusCode, fieldNumber: 2)
        }
        if !self.summary.isEmpty {
            try visitor.visitSingularStringField(value: self.summary, fieldNumber: 3)
        }
        if !self.detail.isEmpty {
            try visitor.visitSingularStringField(value: self.detail, fieldNumber: 4)
        }
        if self.reachable {
            try visitor.visitSingularBoolField(value: self.reachable, fieldNumber: 5)
        }
        try unknownFields.traverse(visitor: &visitor)
    }
}

public struct TailnetClient: Client, GRPCClient {
    public let channel: GRPCChannel
    public var defaultCallOptions: CallOptions

    public init(channel: any GRPCChannel) {
        self.channel = channel
        self.defaultCallOptions = .init()
    }

    public func discover(
        _ request: Burrow_TailnetDiscoverRequest,
        callOptions: CallOptions? = nil
    ) async throws -> Burrow_TailnetDiscoverResponse {
        try await self.performAsyncUnaryCall(
            path: "/burrow.TailnetControl/Discover",
            request: request,
            callOptions: callOptions ?? self.defaultCallOptions,
            interceptors: []
        )
    }

    public func probe(
        _ request: Burrow_TailnetProbeRequest,
        callOptions: CallOptions? = nil
    ) async throws -> Burrow_TailnetProbeResponse {
        try await self.performAsyncUnaryCall(
            path: "/burrow.TailnetControl/Probe",
            request: request,
            callOptions: callOptions ?? self.defaultCallOptions,
            interceptors: []
        )
    }
}
