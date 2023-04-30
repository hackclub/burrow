//
//  MemoryGame.swift
//  Memorize
//
//  Created by Thomas Stubblefield on 3/12/23.
//

import Foundation

struct Model {
    
    var status: Status = .unknown
    
    mutating func connectToBurrow() {
        print("let's get this working")
        status = .valid
        print(status)
    }
}

enum Status: CustomStringConvertible {
    case unknown
    case blank
    case valid
    case error
    case loading
    
    var description: String {
        switch self {
        case .unknown:
            return "Unknown"
        case .blank:
            return "Blank"
        case .valid:
            return "Valid"
        case .loading:
            return "Loading"
        default:
            return "Default"
        }
    }
}
