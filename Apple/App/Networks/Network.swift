import SwiftUI

protocol Network {
    associatedtype Label: View

    var id: String { get }
    var backgroundColor: Color { get }

    var label: Label { get }
}
