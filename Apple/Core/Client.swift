import GRPC
import NIOTransportServices

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
