import XCTest
@testable import DitherIR

final class ProgramMetadataTests: XCTestCase {
    func testFirstBloomZeroRadiusAndTableBoundary() {
        XCTAssertEqual(Program.bloomIndex(in: [0]), 1)
        XCTAssertEqual(Program.bloomIndex(in: [2, 99, 2, 14, 4]), 3)
        XCTAssertEqual(Program.bloomIndex(in: [4, 14, 4, 0, 0]), 1)
        XCTAssertEqual(Program.bloomIndex(in: [4, 14.5, 4, 0, 0]), 1)
        XCTAssertEqual(Program.bloomIndex(in: [4, 15, 4, 0, 0]), 5)
        XCTAssertEqual(Program.bloomIndex(in: [10, 99, 2, 14, 4, 0, 0, 14, 4, 1, 0.5]), 3)
    }
    func testShaderWalkLimit() {
        for preceding in [65_535, 65_536] {
            var data = [Float(preceding * 2 + 4)]
            for _ in 0..<preceding { data += [99, 2] }
            let split = data.count
            data += [14, 4, 0, 0]
            XCTAssertEqual(Program.bloomIndex(in: data), preceding < 65_536 ? split : data.count)
        }
    }
    func testMalformedListsAreRejectedIncludingAfterBloom() {
        let lists: [[Float]] = [[], [.nan], [.infinity], [-1], [1, 99],
            [2, 99, 0], [2, 99, 2.5], [6, 14, 4, 0, 0, 99, 0]]
        for data in lists { XCTAssertNil(Program.bloomIndex(in: data), "\(data)") }
    }
}
