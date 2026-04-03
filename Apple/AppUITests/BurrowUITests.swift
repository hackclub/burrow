import XCTest

@MainActor
final class BurrowTailnetLoginUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    func testTailnetLoginThroughAuthentikWebSession() throws {
        let email = try requiredEnvironment("BURROW_UI_TEST_EMAIL")
        let username = ProcessInfo.processInfo.environment["BURROW_UI_TEST_USERNAME"] ?? email
        let password = try requiredEnvironment("BURROW_UI_TEST_PASSWORD")

        let app = XCUIApplication()
        app.launch()

        let tailnetButton = app.buttons["quick-add-tailnet"]
        XCTAssertTrue(tailnetButton.waitForExistence(timeout: 15), "Tailnet add button did not appear")
        tailnetButton.tap()

        let discoveryField = app.textFields["tailnet-discovery-email"]
        XCTAssertTrue(discoveryField.waitForExistence(timeout: 10), "Tailnet discovery email field did not appear")
        replaceText(in: discoveryField, with: email)

        let findServerButton = app.buttons["tailnet-find-server"]
        XCTAssertTrue(findServerButton.waitForExistence(timeout: 5), "Find Server button did not appear")
        findServerButton.tap()

        let discoveryCard = app.otherElements["tailnet-discovery-card"]
        XCTAssertTrue(discoveryCard.waitForExistence(timeout: 20), "Tailnet discovery result did not appear")

        let authorityField = app.textFields["tailnet-authority"]
        XCTAssertTrue(authorityField.waitForExistence(timeout: 10), "Tailnet authority field did not appear")
        XCTAssertTrue(
            waitForFieldValue(authorityField, containing: "ts.burrow.net", timeout: 20),
            "Tailnet authority was not populated from discovery"
        )

        let probeButton = app.buttons["tailnet-check-connection"]
        XCTAssertTrue(probeButton.waitForExistence(timeout: 5), "Check Connection button did not appear")
        probeButton.tap()

        let probeCard = app.otherElements["tailnet-authority-probe-card"]
        XCTAssertTrue(probeCard.waitForExistence(timeout: 20), "Tailnet connection probe did not complete")

        let signInButton = app.buttons["tailnet-start-sign-in"]
        XCTAssertTrue(signInButton.waitForExistence(timeout: 10), "Tailnet sign-in button did not appear")
        signInButton.tap()

        acceptAuthenticationPromptIfNeeded(in: app)

        let webSession = webAuthenticationSession()
        XCTAssertTrue(webSession.waitForExistence(timeout: 20), "Safari authentication session did not appear")

        signIntoAuthentik(in: webSession, username: username, password: password)

        app.activate()
        XCTAssertTrue(
            waitForButtonLabel(app.buttons["tailnet-start-sign-in"], equals: "Signed In", timeout: 60),
            "Tailnet sign-in never reached the running state"
        )
    }

    private func acceptAuthenticationPromptIfNeeded(in app: XCUIApplication) {
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let promptCandidates = [
            springboard.buttons["Continue"],
            springboard.buttons["Allow"],
            app.buttons["Continue"],
            app.buttons["Allow"],
        ]

        for button in promptCandidates where button.waitForExistence(timeout: 3) {
            button.tap()
            return
        }
    }

    private func webAuthenticationSession() -> XCUIApplication {
        let safariViewService = XCUIApplication(bundleIdentifier: "com.apple.SafariViewService")
        if safariViewService.waitForExistence(timeout: 5) {
            return safariViewService
        }

        let safari = XCUIApplication(bundleIdentifier: "com.apple.mobilesafari")
        _ = safari.waitForExistence(timeout: 5)
        return safari
    }

    private func signIntoAuthentik(in webSession: XCUIApplication, username: String, password: String) {
        let usernameField = firstExistingElement(
            in: webSession,
            queries: [
                { $0.textFields["Username"] },
                { $0.textFields["Email or Username"] },
                { $0.textFields["Email address"] },
                { $0.textFields["Email"] },
                { $0.webViews.textFields["Username"] },
                { $0.webViews.textFields["Email or Username"] },
                { $0.descendants(matching: .textField).firstMatch },
            ],
            timeout: 25
        )
        XCTAssertTrue(usernameField.exists, "Authentik username field did not appear")
        replaceText(in: usernameField, with: username)

        let immediatePasswordField = firstExistingSecureField(in: webSession, timeout: 2)
        if immediatePasswordField.exists {
            replaceSecureText(in: immediatePasswordField, with: password)
            tapFirstExistingButton(
                in: webSession,
                titles: ["Continue", "Sign In", "Log in", "Login"],
                timeout: 5
            )
            return
        }

        tapFirstExistingButton(
            in: webSession,
            titles: ["Continue", "Next", "Sign In", "Log in", "Login"],
            timeout: 5
        )

        let passwordField = firstExistingSecureField(in: webSession, timeout: 20)
        XCTAssertTrue(passwordField.exists, "Authentik password field did not appear")
        replaceSecureText(in: passwordField, with: password)
        tapFirstExistingButton(
            in: webSession,
            titles: ["Continue", "Sign In", "Log in", "Login"],
            timeout: 5
        )
    }

    private func firstExistingSecureField(in app: XCUIApplication, timeout: TimeInterval) -> XCUIElement {
        let candidates = [
            app.secureTextFields["Password"],
            app.secureTextFields["Password or Token"],
            app.webViews.secureTextFields["Password"],
            app.webViews.secureTextFields["Password or Token"],
            app.descendants(matching: .secureTextField).firstMatch,
        ]

        return firstExistingElement(from: candidates, timeout: timeout)
    }

    private func tapFirstExistingButton(
        in app: XCUIApplication,
        titles: [String],
        timeout: TimeInterval
    ) {
        let candidates = titles.flatMap { title in
            [
                app.buttons[title],
                app.webViews.buttons[title],
            ]
        } + [app.descendants(matching: .button).firstMatch]

        let button = firstExistingElement(from: candidates, timeout: timeout)
        XCTAssertTrue(button.exists, "Expected one of \(titles.joined(separator: ", ")) to appear")
        button.tap()
    }

    private func requiredEnvironment(_ key: String) throws -> String {
        guard let value = ProcessInfo.processInfo.environment[key],
              !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        else {
            throw XCTSkip("Missing required UI test environment variable \(key)")
        }
        return value
    }

    private func waitForFieldValue(
        _ field: XCUIElement,
        containing substring: String,
        timeout: TimeInterval
    ) -> Bool {
        let predicate = NSPredicate(format: "value CONTAINS %@", substring)
        let expectation = XCTNSPredicateExpectation(predicate: predicate, object: field)
        return XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed
    }

    private func waitForButtonLabel(
        _ button: XCUIElement,
        equals expected: String,
        timeout: TimeInterval
    ) -> Bool {
        let predicate = NSPredicate(format: "label == %@", expected)
        let expectation = XCTNSPredicateExpectation(predicate: predicate, object: button)
        return XCTWaiter.wait(for: [expectation], timeout: timeout) == .completed
    }

    private func firstExistingElement(
        in app: XCUIApplication,
        queries: [(XCUIApplication) -> XCUIElement],
        timeout: TimeInterval
    ) -> XCUIElement {
        firstExistingElement(from: queries.map { $0(app) }, timeout: timeout)
    }

    private func firstExistingElement(from candidates: [XCUIElement], timeout: TimeInterval) -> XCUIElement {
        let deadline = Date().addingTimeInterval(timeout)
        repeat {
            for candidate in candidates where candidate.exists {
                return candidate
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.2))
        } while Date() < deadline

        return candidates[0]
    }

    private func replaceText(in element: XCUIElement, with value: String) {
        element.tap()
        clearText(in: element)
        element.typeText(value)
    }

    private func replaceSecureText(in element: XCUIElement, with value: String) {
        element.tap()
        clearText(in: element)
        element.typeText(value)
    }

    private func clearText(in element: XCUIElement) {
        guard let currentValue = element.value as? String, !currentValue.isEmpty else {
            return
        }

        let deleteSequence = String(repeating: XCUIKeyboardKey.delete.rawValue, count: currentValue.count)
        element.typeText(deleteSequence)
    }
}
