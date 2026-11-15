import XCTest
@testable import DitherKit

final class ComponentStateTests: XCTestCase {
    func testLegendCanRevealInitiallyHiddenSeriesAndHideAgain() {
        let input = [Series(label: "Visible", tone: .green, values: [1, 2]), Series(label: "Initially hidden", tone: .purple, values: [3, 4], hidden: true)]
        var state = SeriesVisibility()
        XCTAssertEqual(state.applying(to: input).map(\.hidden), [false, true])
        var edited = state.applying(to: input)
        edited[1].hidden = false
        state.update(from: edited)
        XCTAssertEqual(state.applying(to: input).map(\.hidden), [false, false])
        edited[0].hidden = true
        state.update(from: edited)
        XCTAssertEqual(state.applying(to: input).map(\.hidden), [true, false])
    }
}
