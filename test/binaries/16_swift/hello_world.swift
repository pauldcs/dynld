/*
 * swiftc hello_world.swift -o hello_world
 */

import Foundation

final class HelloWorld {
    func hello_world() {
        let message = "Hello, World!"
        FileHandle.standardOutput.write(
            message.appending("\n").data(using: .utf8)!
        )
    }
}

autoreleasepool {
    let hw = HelloWorld()
    hw.hello_world()
}