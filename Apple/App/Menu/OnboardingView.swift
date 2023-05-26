//
//  OnboardingView.swift
//  App
//
//  Created by Juan Pablo Gutierrez on 25/05/23.
//
//  Represents the onboarding screen

import SwiftUI

func isFirstTime() -> Bool {
    // TODO REMOVE
    // DEBUG ONLY
    UserDefaults.standard.set(false, forKey: "launchedBefore")
    // REMOVE
    let launchedBefore = UserDefaults.standard.bool(forKey: "launchedBefore")
    if launchedBefore {
        print("Not first launch.")
    } else {
        print("First launch, setting UserDefault.")
        UserDefaults.standard.set(true, forKey: "launchedBefore")
    }
    return !launchedBefore
}

struct OnboardingView: View {
    var body: some View {
        if isFirstTime() {
            HStack(alignment: .center) {
                Text("Built by and for hacker")
                Divider()
                    .frame(height: 350.0)
                // Must develop this part, almost ready
                VStack(alignment: .center) {
                    Text("Welcome to burrow")
                    Spacer()
                        .frame(height: /*@START_MENU_TOKEN@*/20.0/*@END_MENU_TOKEN@*/)
                    Text("This is a high-end VPN service")
                }
            }.padding(20)
        } else {
            VStack(alignment: .leading) {
                Text("Waaa")
            }.padding(20)
        }
    }
}
