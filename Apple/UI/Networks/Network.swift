import Atomics
import BurrowCore
import SwiftProtobuf
import SwiftUI

protocol Network {
    associatedtype NetworkType: Message
    associatedtype Label: View

    static var type: Burrow_NetworkType { get }

    var id: Int32 { get }
    var backgroundColor: Color { get }

    @MainActor var label: Label { get }
}

@Observable
@MainActor
final class NetworkViewModel: Sendable {
    private(set) var networks: [Burrow_Network] = []

    private var task: Task<Void, Error>!

    init(socketURL: URL) {
        task = Task { [weak self] in
            let client = NetworksClient.unix(socketURL: socketURL)
            for try await networks in client.networkList(.init()) {
                guard let viewModel = self else { continue }
                Task { @MainActor in
                    viewModel.networks = networks.network
                }
            }
        }
    }
}
