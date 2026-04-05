import XCTest
import UIKit

@MainActor
final class BurrowTailnetLoginUITests: XCTestCase {
    private enum TailnetLoginMode: String, Decodable {
        case tailscale
        case discovered
    }

    private struct TestConfig: Decodable {
        let email: String
        let username: String
        let password: String
        let mode: TailnetLoginMode?
    }

    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    func testTailnetLoginThroughAuthentikWebSession() throws {
        let config = try loadTestConfig()
        let email = config.email
        let username = config.username
        let password = config.password
        let mode = config.mode ?? .tailscale
        let browserIdentity = mode == .tailscale ? email : username

        let app = XCUIApplication()
        app.launch()

        let tailnetButton = app.buttons["quick-add-tailnet"]
        XCTAssertTrue(tailnetButton.waitForExistence(timeout: 15), "Tailnet add button did not appear")
        tailnetButton.tap()

        configureTailnetIfNeeded(in: app, mode: mode)

        let discoveryField = app.textFields["tailnet-discovery-email"]
        XCTAssertTrue(discoveryField.waitForExistence(timeout: 10), "Tailnet discovery email field did not appear")
        replaceText(in: discoveryField, with: email)

        let serverCard = app.descendants(matching: .any)
            .matching(identifier: "tailnet-server-card")
            .firstMatch
        XCTAssertTrue(serverCard.waitForExistence(timeout: 5), "Tailnet server card did not appear")

        let signInButton = app.buttons["tailnet-start-sign-in"]
        XCTAssertTrue(signInButton.waitForExistence(timeout: 10), "Tailnet sign-in button did not appear")
        signInButton.tap()

        acceptAuthenticationPromptIfNeeded(in: app, timeout: 20)

        let webSession = webAuthenticationSession()
        XCTAssertTrue(webSession.waitForExistence(timeout: 20), "Safari authentication session did not appear")

        signIntoAuthentik(in: webSession, username: browserIdentity, password: password)

        app.activate()
        XCTAssertTrue(
            waitForTailnetSignedIn(in: app, timeout: 60),
            "Tailnet sign-in never reached the running state"
        )
    }

    private func configureTailnetIfNeeded(in app: XCUIApplication, mode: TailnetLoginMode) {
        guard mode == .discovered else { return }

        openTailnetMenu(in: app)
        tapMenuButton(named: "Edit Custom Server", in: app)

        openTailnetMenu(in: app)
        tapMenuButton(named: "Show Advanced Settings", in: app)

        let authorityField = app.textFields["tailnet-authority"]
        XCTAssertTrue(authorityField.waitForExistence(timeout: 10), "Tailnet authority field did not appear")
        replaceText(in: authorityField, with: "")
    }

    private func openTailnetMenu(in app: XCUIApplication) {
        let moreButton = app.buttons["More"]
        XCTAssertTrue(moreButton.waitForExistence(timeout: 5), "Tailnet menu button did not appear")
        moreButton.tap()
    }

    private func tapMenuButton(named title: String, in app: XCUIApplication) {
        let menuButton = firstExistingElement(
            from: [
                app.buttons[title],
                app.descendants(matching: .button)[title],
            ],
            timeout: 5
        )
        XCTAssertTrue(menuButton.exists, "Menu action \(title) did not appear")
        menuButton.tap()
    }

    private func acceptAuthenticationPromptIfNeeded(
        in app: XCUIApplication,
        timeout: TimeInterval
    ) {
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        let deadline = Date().addingTimeInterval(timeout)

        repeat {
            let promptCandidates = [
                springboard.buttons["Continue"],
                springboard.buttons["Allow"],
                app.buttons["Continue"],
                app.buttons["Allow"],
            ]

            for button in promptCandidates where button.exists && button.isHittable {
                button.tap()
                return
            }

            RunLoop.current.run(until: Date().addingTimeInterval(0.25))
        } while Date() < deadline

        let promptCandidates = [
            springboard.buttons["Continue"],
            springboard.buttons["Allow"],
            app.buttons["Continue"],
            app.buttons["Allow"],
        ]

        for button in promptCandidates where button.exists {
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
        followTailnetRedirectIfNeeded(in: webSession)

        if !webSession.exists {
            return
        }

        let immediatePasswordField = firstExistingSecureField(in: webSession, timeout: 2)
        if immediatePasswordField.exists {
            replaceSecureText(in: immediatePasswordField, within: webSession, with: password)
            submitAuthenticationForm(in: webSession, focusedField: immediatePasswordField)
            return
        }

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
            timeout: 12
        )
        if !usernameField.exists {
            return
        }
        replaceText(in: usernameField, with: username)

        tapFirstExistingButton(
            in: webSession,
            titles: ["Continue", "Next", "Sign In", "Log in", "Login"],
            timeout: 5
        )

        let passwordField = firstExistingSecureField(in: webSession, timeout: 20)
        XCTAssertTrue(passwordField.exists, "Authentik password field did not appear")
        replaceSecureText(in: passwordField, within: webSession, with: password)
        submitAuthenticationForm(in: webSession, focusedField: passwordField)
    }

    private func followTailnetRedirectIfNeeded(in webSession: XCUIApplication) {
        let redirectCandidates = [
            webSession.links["Found"],
            webSession.webViews.links["Found"],
            webSession.buttons["Found"],
            webSession.webViews.buttons["Found"],
        ]

        let redirectLink = firstExistingElement(from: redirectCandidates, timeout: 8)
        if redirectLink.exists {
            redirectLink.tap()
        }
    }

    private func firstExistingSecureField(in app: XCUIApplication, timeout: TimeInterval) -> XCUIElement {
        let candidates = [
            app.descendants(matching: .secureTextField).firstMatch,
            app.secureTextFields["Password"],
            app.secureTextFields["Password or Token"],
            app.webViews.secureTextFields["Password"],
            app.webViews.secureTextFields["Password or Token"],
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

    private func submitAuthenticationForm(in app: XCUIApplication, focusedField: XCUIElement) {
        focus(focusedField)
        focusedField.typeText("\n")
        if waitForAny(
            [
                { !focusedField.exists },
                { !app.staticTexts["Burrow Tailnet Authentication"].exists },
            ],
            timeout: 1.5
        ) {
            return
        }

        let keyboard = app.keyboards.firstMatch
        if keyboard.waitForExistence(timeout: 2) {
            let keyboardCandidates = [
                "Return",
                "return",
                "Go",
                "go",
                "Continue",
                "continue",
                "Done",
                "done",
                "Join",
                "join",
                "Sign In",
                "Log In",
                "Login",
            ]
            for title in keyboardCandidates {
                let key = keyboard.buttons[title]
                if key.exists && key.isHittable {
                    key.tap()
                    return
                }
            }

            if let lastKey = keyboard.buttons.allElementsBoundByIndex.last,
               lastKey.exists,
               lastKey.isHittable
            {
                lastKey.tap()
                return
            }
        }

        tapFirstExistingButton(
            in: app,
            titles: ["Continue", "Sign In", "Log in", "Login"],
            timeout: 5
        )
    }

    private func loadTestConfig() throws -> TestConfig {
        let environment = ProcessInfo.processInfo.environment
        if let email = nonEmptyEnvironment("BURROW_UI_TEST_EMAIL"),
           let password = nonEmptyEnvironment("BURROW_UI_TEST_PASSWORD")
        {
            return TestConfig(
                email: email,
                username: nonEmptyEnvironment("BURROW_UI_TEST_USERNAME") ?? email,
                password: password,
                mode: nonEmptyEnvironment("BURROW_UI_TEST_TAILNET_MODE")
                    .flatMap(TailnetLoginMode.init(rawValue:))
            )
        }

        let configPath = environment["BURROW_UI_TEST_CONFIG_PATH"] ?? "/tmp/burrow-ui-test-config.json"
        let configURL = URL(fileURLWithPath: configPath)
        guard FileManager.default.fileExists(atPath: configURL.path) else {
            throw XCTSkip(
                "Missing UI test configuration. Expected env vars or config file at \(configURL.path)"
            )
        }

        let data = try Data(contentsOf: configURL)
        return try JSONDecoder().decode(TestConfig.self, from: data)
    }

    private func nonEmptyEnvironment(_ key: String) -> String? {
        guard let value = ProcessInfo.processInfo.environment[key]?
            .trimmingCharacters(in: .whitespacesAndNewlines),
            !value.isEmpty
        else {
            return nil
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

    private func waitForTailnetSignedIn(in app: XCUIApplication, timeout: TimeInterval) -> Bool {
        let button = app.buttons["tailnet-start-sign-in"]
        let deadline = Date().addingTimeInterval(timeout)

        repeat {
            acceptAuthenticationPromptIfNeeded(in: app, timeout: 1)
            if button.exists, button.label == "Signed In" {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.3))
        } while Date() < deadline

        return button.exists && button.label == "Signed In"
    }

    private func waitForAny(_ conditions: [() -> Bool], timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        repeat {
            if conditions.contains(where: { $0() }) {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.2))
        } while Date() < deadline
        return conditions.contains(where: { $0() })
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
        focus(element)
        clearText(in: element)
        element.typeText(value)
    }

    private func replaceSecureText(in element: XCUIElement, within app: XCUIApplication, with value: String) {
        UIPasteboard.general.string = value
        focus(element)
        for revealMenu in [
            { element.doubleTap() },
            { element.press(forDuration: 1.2) },
        ] {
            revealMenu()
            let pasteButton = firstExistingElement(from: pasteCandidates(in: app), timeout: 3)
            if pasteButton.exists {
                pasteButton.tap()
                return
            }
        }

        focus(element)
        element.typeText(value)
    }

    private func clearText(in element: XCUIElement) {
        guard let currentValue = element.value as? String, !currentValue.isEmpty else {
            return
        }

        let deleteSequence = String(repeating: XCUIKeyboardKey.delete.rawValue, count: currentValue.count)
        element.typeText(deleteSequence)
    }

    private func focus(_ element: XCUIElement) {
        element.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        RunLoop.current.run(until: Date().addingTimeInterval(0.3))
    }

    private func pasteCandidates(in app: XCUIApplication) -> [XCUIElement] {
        let pasteLabels = ["Paste", "Incolla", "Paste from Clipboard"]
        return pasteLabels.flatMap { label in
            [
                app.menuItems[label],
                app.buttons[label],
                app.webViews.buttons[label],
                app.descendants(matching: .button).matching(NSPredicate(format: "label == %@", label)).firstMatch,
                app.descendants(matching: .menuItem).matching(NSPredicate(format: "label == %@", label)).firstMatch,
            ]
        }
    }
}
