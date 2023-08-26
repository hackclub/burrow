//
//  NetworkSettingsConverter.swift
//  NetworkExtension
//
//  Created by Jett Chen on 2023/7/7.
//

import Foundation
import NetworkExtension

public struct TunCrateNetworkSettings {
    let addr: Int64
    let netmask: Int64
    let mtu: Int32
}

extension TunCrateNetworkSettings {
    var decodedIPv4Addr: String? {
        return decodeIPv4(addr)
    }

    var decodedIPv4Netmask: String? {
        return decodeIPv4(netmask)
    }

    var decodedMTU: Int? {
        return mtu >= 0 ? Int(mtu) : nil
    }

    private func decodeIPv4(_ addr: Int64) -> String? {
        if addr < 0 {
            return nil
        }
        let bytes = (
            UInt8((addr & 0xFF000000) >> 24),
            UInt8((addr & 0x00FF0000) >> 16),
            UInt8((addr & 0x0000FF00) >> 8),
            UInt8(addr & 0x000000FF)
        )
        return "\(bytes.0).\(bytes.1).\(bytes.2).\(bytes.3)"
    }
    
    func generateNetworkSettings() -> NEPacketTunnelNetworkSettings {
            let neSettings = NEPacketTunnelNetworkSettings()

            if let addr = decodedIPv4Addr, let netmask = decodedIPv4Netmask {
                neSettings.ipv4Settings = NEIPv4Settings(addresses: [addr], subnetMasks: [netmask])
            }
            if let mtuValue = decodedMTU {
                neSettings.mtu = NSNumber(value: mtuValue)
            }
            return neSettings
        }

}

