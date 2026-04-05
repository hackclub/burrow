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

public struct Burrow_TailnetLoginStartRequest: Sendable {
    public var accountName: String = ""
    public var identityName: String = ""
    public var hostname: String = ""
    public var authority: String = ""
    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
}

public struct Burrow_TailnetLoginStatusRequest: Sendable {
    public var sessionID: String = ""
    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
}

public struct Burrow_TailnetLoginCancelRequest: Sendable {
    public var sessionID: String = ""
    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
}

public struct Burrow_TailnetLoginStatusResponse: Sendable {
    public var sessionID: String = ""
    public var backendState: String = ""
    public var authURL: String = ""
    public var running: Bool = false
    public var needsLogin: Bool = false
    public var tailnetName: String = ""
    public var magicDNSSuffix: String = ""
    public var selfDNSName: String = ""
    public var tailnetIPs: [String] = []
    public var health: [String] = []
    public var unknownFields = SwiftProtobuf.UnknownStorage()

    public init() {}
}

public struct Burrow_TunnelPacket: Sendable {
    public var payload = Data()
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

extension Burrow_TailnetLoginStartRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TailnetLoginStartRequest"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .standard(proto: "account_name"),
        2: .standard(proto: "identity_name"),
        3: .same(proto: "hostname"),
        4: .same(proto: "authority"),
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularStringField(value: &self.accountName)
            case 2: try decoder.decodeSingularStringField(value: &self.identityName)
            case 3: try decoder.decodeSingularStringField(value: &self.hostname)
            case 4: try decoder.decodeSingularStringField(value: &self.authority)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.accountName.isEmpty {
            try visitor.visitSingularStringField(value: self.accountName, fieldNumber: 1)
        }
        if !self.identityName.isEmpty {
            try visitor.visitSingularStringField(value: self.identityName, fieldNumber: 2)
        }
        if !self.hostname.isEmpty {
            try visitor.visitSingularStringField(value: self.hostname, fieldNumber: 3)
        }
        if !self.authority.isEmpty {
            try visitor.visitSingularStringField(value: self.authority, fieldNumber: 4)
        }
        try unknownFields.traverse(visitor: &visitor)
    }
}

extension Burrow_TailnetLoginStatusRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TailnetLoginStatusRequest"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .standard(proto: "session_id")
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularStringField(value: &self.sessionID)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.sessionID.isEmpty {
            try visitor.visitSingularStringField(value: self.sessionID, fieldNumber: 1)
        }
        try unknownFields.traverse(visitor: &visitor)
    }
}

extension Burrow_TailnetLoginCancelRequest: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TailnetLoginCancelRequest"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .standard(proto: "session_id")
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularStringField(value: &self.sessionID)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.sessionID.isEmpty {
            try visitor.visitSingularStringField(value: self.sessionID, fieldNumber: 1)
        }
        try unknownFields.traverse(visitor: &visitor)
    }
}

extension Burrow_TailnetLoginStatusResponse: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TailnetLoginStatusResponse"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .standard(proto: "session_id"),
        2: .standard(proto: "backend_state"),
        3: .standard(proto: "auth_url"),
        4: .same(proto: "running"),
        5: .standard(proto: "needs_login"),
        6: .standard(proto: "tailnet_name"),
        7: .standard(proto: "magic_dns_suffix"),
        8: .standard(proto: "self_dns_name"),
        9: .standard(proto: "tailnet_ips"),
        10: .same(proto: "health"),
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularStringField(value: &self.sessionID)
            case 2: try decoder.decodeSingularStringField(value: &self.backendState)
            case 3: try decoder.decodeSingularStringField(value: &self.authURL)
            case 4: try decoder.decodeSingularBoolField(value: &self.running)
            case 5: try decoder.decodeSingularBoolField(value: &self.needsLogin)
            case 6: try decoder.decodeSingularStringField(value: &self.tailnetName)
            case 7: try decoder.decodeSingularStringField(value: &self.magicDNSSuffix)
            case 8: try decoder.decodeSingularStringField(value: &self.selfDNSName)
            case 9: try decoder.decodeRepeatedStringField(value: &self.tailnetIPs)
            case 10: try decoder.decodeRepeatedStringField(value: &self.health)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.sessionID.isEmpty {
            try visitor.visitSingularStringField(value: self.sessionID, fieldNumber: 1)
        }
        if !self.backendState.isEmpty {
            try visitor.visitSingularStringField(value: self.backendState, fieldNumber: 2)
        }
        if !self.authURL.isEmpty {
            try visitor.visitSingularStringField(value: self.authURL, fieldNumber: 3)
        }
        if self.running {
            try visitor.visitSingularBoolField(value: self.running, fieldNumber: 4)
        }
        if self.needsLogin {
            try visitor.visitSingularBoolField(value: self.needsLogin, fieldNumber: 5)
        }
        if !self.tailnetName.isEmpty {
            try visitor.visitSingularStringField(value: self.tailnetName, fieldNumber: 6)
        }
        if !self.magicDNSSuffix.isEmpty {
            try visitor.visitSingularStringField(value: self.magicDNSSuffix, fieldNumber: 7)
        }
        if !self.selfDNSName.isEmpty {
            try visitor.visitSingularStringField(value: self.selfDNSName, fieldNumber: 8)
        }
        if !self.tailnetIPs.isEmpty {
            try visitor.visitRepeatedStringField(value: self.tailnetIPs, fieldNumber: 9)
        }
        if !self.health.isEmpty {
            try visitor.visitRepeatedStringField(value: self.health, fieldNumber: 10)
        }
        try unknownFields.traverse(visitor: &visitor)
    }
}

extension Burrow_TunnelPacket: SwiftProtobuf.Message, SwiftProtobuf._MessageImplementationBase, SwiftProtobuf._ProtoNameProviding {
    public static let protoMessageName: String = "burrow.TunnelPacket"
    public static let _protobuf_nameMap: SwiftProtobuf._NameMap = [
        1: .same(proto: "payload")
    ]

    public mutating func decodeMessage<D: SwiftProtobuf.Decoder>(decoder: inout D) throws {
        while let fieldNumber = try decoder.nextFieldNumber() {
            switch fieldNumber {
            case 1: try decoder.decodeSingularBytesField(value: &self.payload)
            default: break
            }
        }
    }

    public func traverse<V: SwiftProtobuf.Visitor>(visitor: inout V) throws {
        if !self.payload.isEmpty {
            try visitor.visitSingularBytesField(value: self.payload, fieldNumber: 1)
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

    public func loginStart(
        _ request: Burrow_TailnetLoginStartRequest,
        callOptions: CallOptions? = nil
    ) async throws -> Burrow_TailnetLoginStatusResponse {
        try await self.performAsyncUnaryCall(
            path: "/burrow.TailnetControl/LoginStart",
            request: request,
            callOptions: callOptions ?? self.defaultCallOptions,
            interceptors: []
        )
    }

    public func loginStatus(
        _ request: Burrow_TailnetLoginStatusRequest,
        callOptions: CallOptions? = nil
    ) async throws -> Burrow_TailnetLoginStatusResponse {
        try await self.performAsyncUnaryCall(
            path: "/burrow.TailnetControl/LoginStatus",
            request: request,
            callOptions: callOptions ?? self.defaultCallOptions,
            interceptors: []
        )
    }

    public func loginCancel(
        _ request: Burrow_TailnetLoginCancelRequest,
        callOptions: CallOptions? = nil
    ) async throws -> Burrow_Empty {
        try await self.performAsyncUnaryCall(
            path: "/burrow.TailnetControl/LoginCancel",
            request: request,
            callOptions: callOptions ?? self.defaultCallOptions,
            interceptors: []
        )
    }
}

public struct TunnelPacketClient: Client, GRPCClient {
    public let channel: GRPCChannel
    public var defaultCallOptions: CallOptions

    public init(channel: any GRPCChannel) {
        self.channel = channel
        self.defaultCallOptions = .init()
    }

    public func makeTunnelPacketsCall(
        callOptions: CallOptions? = nil
    ) -> GRPCAsyncBidirectionalStreamingCall<Burrow_TunnelPacket, Burrow_TunnelPacket> {
        self.makeAsyncBidirectionalStreamingCall(
            path: "/burrow.Tunnel/TunnelPackets",
            callOptions: callOptions ?? self.defaultCallOptions,
            interceptors: []
        )
    }
}
