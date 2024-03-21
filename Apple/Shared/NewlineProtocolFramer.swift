import Foundation
import Network

final class NewlineProtocolFramer: NWProtocolFramerImplementation {
    private static let delimeter: UInt8 = 10 // `\n`

    static let definition = NWProtocolFramer.Definition(implementation: NewlineProtocolFramer.self)
    static let label = "Lines"

    init(framer: NWProtocolFramer.Instance) { }

    func start(framer: NWProtocolFramer.Instance) -> NWProtocolFramer.StartResult { .ready }
    func stop(framer: NWProtocolFramer.Instance) -> Bool { true }

    func wakeup(framer: NWProtocolFramer.Instance) { }
    func cleanup(framer: NWProtocolFramer.Instance) { }

    func handleInput(framer: NWProtocolFramer.Instance) -> Int {
        while true {
            var result: [Data] = []
            let parsed = framer.parseInput(minimumIncompleteLength: 1, maximumLength: 16_000) { buffer, _ in
                guard let buffer else { return 0 }
                var lines = buffer
                    .split(separator: Self.delimeter, omittingEmptySubsequences: false)
                    .map { Data($0) }
                guard lines.count > 1 else { return 0 }
                _ = lines.popLast()

                result = lines
                return lines.reduce(lines.count) { $0 + $1.count }
            }

            guard parsed && !result.isEmpty else { break }

            for line in result {
                framer.deliverInput(data: line, message: .init(instance: framer), isComplete: true)
            }
        }
        return 0
    }

    func handleOutput(
        framer: NWProtocolFramer.Instance,
        message: NWProtocolFramer.Message,
        messageLength: Int,
        isComplete: Bool
    ) {
        do {
            try framer.writeOutputNoCopy(length: messageLength)
            framer.writeOutput(data: [Self.delimeter])
        } catch {
        }
    }
}
