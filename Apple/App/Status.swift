import Foundation
import NetworkExtension

extension Tunnel {
    enum Status: CustomStringConvertible, Equatable, Hashable {
        case unknown
        case permissionRequired
        case disabled
        case connecting
        case connected(Date)
        case disconnecting
        case disconnected
        case reasserting
        case invalid
        case configurationReadWriteFailed

        var description: String {
            switch self {
            case .unknown:
                return "Unknown"
            case .permissionRequired:
                return "Permission Required"
            case .disconnected:
                return "Disconnected"
            case .disabled:
                return "Disabled"
            case .connecting:
                return "Connecting"
            case .connected:
                return "Connected"
            case .disconnecting:
                return "Disconnecting"
            case .reasserting:
                return "Reasserting"
            case .invalid:
                return "Invalid"
            case .configurationReadWriteFailed:
                return "System Error"
            }
        }
    }
}
