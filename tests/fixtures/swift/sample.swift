import Foundation

// A small calculator.
let banner = "Calculator"

class Calculator {
    var total: Int

    init(seed: Int) {
        total = seed
    }

    func add(_ a: Int, _ b: Int) -> Int {
        return a + b
    }

    var doubled: Int {
        return total * 2
    }
}

func testAdds() {
    XCTAssertEqual(Calculator(seed: 0).add(1, 2), 3)
}
